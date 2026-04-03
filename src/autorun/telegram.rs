//! Telegram Bot API client with polling, retry logic, and command handling.

use std::time::Duration;

use crate::errors::{RexError, RexResult};
use serde_json::Value;
use tracing::{debug, error, info, warn};

use super::chat;

/// Outcome of polling Telegram for updates.
pub enum TelegramPollResult {
    /// A reply matching the expected message_id was received.
    Reply(String),
    /// A /kill command matching our project_id was received.
    Kill,
}

/// Raw HTTP client for the Telegram Bot API.
pub struct TelegramClient {
    token: String,
    chat_id: i64,
    http: reqwest::Client,
    /// Tracks `getUpdates` offset to avoid replaying stale messages.
    pub update_offset: i64,
}

impl TelegramClient {
    pub fn new(token: String, chat_id: i64, initial_offset: Option<i64>) -> Self {
        Self {
            token,
            chat_id,
            http: reqwest::Client::new(),
            update_offset: initial_offset.unwrap_or(0),
        }
    }

    /// Base URL for all Telegram Bot API calls.
    fn api_url(&self, method: &str) -> String {
        format!("https://api.telegram.org/bot{}/{}", self.token, method)
    }

    /// Internal: POST to sendMessage with the given body, with retry logic.
    async fn send_with_body(&self, body: &serde_json::Value) -> RexResult<i64> {
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            let resp = self
                .http
                .post(self.api_url("sendMessage"))
                .json(body)
                .timeout(Duration::from_secs(30))
                .send()
                .await;

            match resp {
                Ok(r) if r.status().is_success() => {
                    let json: Value = r.json().await.map_err(|e| {
                        RexError::Telegram(format!(
                            "failed to parse sendMessage response: {e}"
                        ))
                    })?;
                    let msg_id = json["result"]["message_id"].as_i64().ok_or_else(|| {
                        RexError::Telegram(
                            "missing message_id in sendMessage response".into(),
                        )
                    })?;
                    debug!(msg_id, "telegram message sent");
                    return Ok(msg_id);
                }
                Ok(r) if r.status() == 401 || r.status() == 403 => {
                    return Err(RexError::Telegram(format!(
                        "auth error ({}): check TELEGRAM_BOT_TOKEN",
                        r.status()
                    )));
                }
                Ok(r)
                    if (r.status() == 429 || r.status().is_server_error())
                        && attempt <= 5 =>
                {
                    let delay = backoff_delay(attempt);
                    warn!(
                        status = r.status().as_u16(),
                        attempt,
                        delay_secs = delay.as_secs(),
                        "telegram API error, retrying"
                    );
                    tokio::time::sleep(delay).await;
                }
                Ok(r) => {
                    let status = r.status();
                    let body_text = r.text().await.unwrap_or_default();
                    return Err(RexError::Telegram(format!(
                        "sendMessage failed ({status}): {body_text}"
                    )));
                }
                Err(e) if attempt <= 5 => {
                    let delay = backoff_delay(attempt);
                    warn!(
                        attempt,
                        delay_secs = delay.as_secs(),
                        "telegram request error: {e}, retrying"
                    );
                    tokio::time::sleep(delay).await;
                }
                Err(e) => {
                    return Err(RexError::Telegram(format!(
                        "sendMessage failed after {attempt} attempts: {e}"
                    )));
                }
            }
        }
    }

    /// Send a message and return the `message_id`. Retries with exponential backoff.
    pub async fn send_message(&self, text: &str) -> RexResult<i64> {
        let body = serde_json::json!({
            "chat_id": self.chat_id,
            "text": text,
            "parse_mode": "HTML",
        });
        self.send_with_body(&body).await
    }

    /// Send a message with inline Reply + Stats + Kill buttons. Returns the `message_id`.
    pub async fn send_with_buttons(&self, text: &str) -> RexResult<i64> {
        let body = serde_json::json!({
            "chat_id": self.chat_id,
            "text": text,
            "parse_mode": "HTML",
            "reply_markup": {
                "inline_keyboard": [[
                    { "text": "💬 Reply", "callback_data": "reply" },
                    { "text": "📊 Stats", "callback_data": "query" },
                    { "text": "🛑 Kill", "callback_data": "kill" },
                ]]
            },
        });
        self.send_with_body(&body).await
    }

    /// Send a force-reply prompt (used when the Reply button is pressed).
    /// Returns the `message_id` of the prompt.
    pub async fn send_reply_prompt(&self, project_id: &str) -> RexResult<i64> {
        let text = format!(
            "💬 <b>Reply</b>  ·  <code>{}</code>\n\
             <i>Type your message below</i>",
            super::types::escape_html(project_id),
        );
        let body = serde_json::json!({
            "chat_id": self.chat_id,
            "text": text,
            "parse_mode": "HTML",
            "reply_markup": {
                "force_reply": true,
            },
        });
        self.send_with_body(&body).await
    }

    /// Send a question with `ForceReply` markup, prompting the user's client
    /// to reply directly. Returns the `message_id`.
    pub async fn send_question(&self, text: &str) -> RexResult<i64> {
        let body = serde_json::json!({
            "chat_id": self.chat_id,
            "text": text,
            "parse_mode": "HTML",
            "reply_markup": {
                "force_reply": true,
            },
        });
        self.send_with_body(&body).await
    }

    /// Send a message with inline Reply + Restart buttons for chat sessions.
    pub async fn send_with_chat_buttons(&self, text: &str, chat_id: &str) -> RexResult<i64> {
        let body = serde_json::json!({
            "chat_id": self.chat_id,
            "text": text,
            "parse_mode": "HTML",
            "reply_markup": {
                "inline_keyboard": [[
                    { "text": "💬 Reply", "callback_data": format!("cr:{chat_id}") },
                    { "text": "🔄 Restart", "callback_data": format!("cx:{chat_id}") },
                ]]
            },
        });
        self.send_with_body(&body).await
    }

    /// Send a force-reply prompt for a chat session.
    pub async fn send_chat_reply_prompt(&self, chat_id: &str) -> RexResult<i64> {
        let text = format!(
            "💬 <b>Chat Reply</b>  ·  <code>{cid}</code>\n\
             <i>Type your message below</i>",
            cid = super::types::escape_html(chat_id),
        );
        let body = serde_json::json!({
            "chat_id": self.chat_id,
            "text": text,
            "parse_mode": "HTML",
            "reply_markup": { "force_reply": true },
        });
        self.send_with_body(&body).await
    }

    /// Edit an existing message's text.
    pub async fn edit_message(&self, message_id: i64, text: &str) -> RexResult<()> {
        let body = serde_json::json!({
            "chat_id": self.chat_id,
            "message_id": message_id,
            "text": text,
            "parse_mode": "HTML",
        });
        let resp = self
            .http
            .post(self.api_url("editMessageText"))
            .json(&body)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| RexError::Telegram(format!("editMessageText failed: {e}")))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            return Err(RexError::Telegram(format!(
                "editMessageText failed ({status}): {body_text}"
            )));
        }
        Ok(())
    }

    /// Edit an existing message and add chat buttons.
    pub async fn edit_message_with_chat_buttons(
        &self,
        message_id: i64,
        text: &str,
        chat_id: &str,
    ) -> RexResult<()> {
        let body = serde_json::json!({
            "chat_id": self.chat_id,
            "message_id": message_id,
            "text": text,
            "parse_mode": "HTML",
            "reply_markup": {
                "inline_keyboard": [[
                    { "text": "💬 Reply", "callback_data": format!("cr:{chat_id}") },
                    { "text": "🔄 Restart", "callback_data": format!("cx:{chat_id}") },
                ]]
            },
        });
        let resp = self
            .http
            .post(self.api_url("editMessageText"))
            .json(&body)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| RexError::Telegram(format!("editMessageText failed: {e}")))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            return Err(RexError::Telegram(format!(
                "editMessageText failed ({status}): {body_text}"
            )));
        }
        Ok(())
    }

    /// Long-poll `getUpdates` until a matching reply or `/kill` command arrives.
    ///
    /// Only accepts messages that are replies to `expected_message_id`.
    /// Non-matching messages are logged and skipped.
    /// `/query` commands are answered inline and polling continues.
    pub(crate) async fn wait_for_reply(
        &mut self,
        mut expected_message_id: i64,
        project_id: &str,
        timeout: Duration,
        query_response: &str,
        chat_manager: &chat::ChatManager,
    ) -> RexResult<TelegramPollResult> {
        let deadline = tokio::time::Instant::now() + timeout;
        info!(
            timeout_secs = timeout.as_secs(),
            expected_message_id,
            "waiting for telegram reply"
        );

        loop {
            if tokio::time::Instant::now() >= deadline {
                return Err(RexError::Telegram(format!(
                    "human reply timeout exceeded ({} days)",
                    timeout.as_secs() / 86400
                )));
            }

            let body = serde_json::json!({
                "offset": self.update_offset,
                "timeout": 1,
                "allowed_updates": ["message", "callback_query"],
            });

            let resp = self
                .http
                .post(self.api_url("getUpdates"))
                .json(&body)
                .timeout(Duration::from_secs(10))
                .send()
                .await;

            let json: Value = match resp {
                Ok(r) if r.status().is_success() => {
                    r.json().await.unwrap_or(Value::Null)
                }
                Ok(r) => {
                    warn!(
                        status = r.status().as_u16(),
                        "getUpdates error, retrying"
                    );
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
                Err(e) => {
                    warn!("getUpdates request error: {e}, retrying");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            let updates = match json["result"].as_array() {
                Some(arr) => arr,
                None => continue,
            };

            for update in updates {
                if let Some(uid) = update["update_id"].as_i64() {
                    self.update_offset = uid + 1;
                }

                // Handle callback queries (inline button presses)
                if let Some(cb) = update.get("callback_query") {
                    let cb_chat_id = cb["message"]["chat"]["id"].as_i64().unwrap_or(0);
                    if cb_chat_id != self.chat_id {
                        continue;
                    }
                    let data = cb["data"].as_str().unwrap_or("");
                    let cb_id = cb["id"].as_str().unwrap_or("");
                    self.answer_callback_query(cb_id).await;

                    match data {
                        "kill" => {
                            info!("received kill button press for project {project_id}");
                            return Ok(TelegramPollResult::Kill);
                        }
                        "query" => {
                            info!("received stats button press, sending stats");
                            self.notify(query_response).await;
                        }
                        "reply" => {
                            info!("received reply button press, sending force-reply prompt");
                            if let Ok(prompt_id) = self.send_reply_prompt(project_id).await {
                                // Update expected_message_id so we accept replies to the prompt
                                expected_message_id = prompt_id;
                            }
                        }
                        d if d.starts_with("cr:") => {
                            let cid = &d[3..];
                            info!(chat_id = cid, "chat reply button pressed");
                            if let Ok(prompt_id) = self.send_chat_reply_prompt(cid).await {
                                chat::register_message(chat_manager, prompt_id, cid).await;
                            }
                        }
                        d if d.starts_with("cx:") => {
                            let cid = &d[3..];
                            info!(chat_id = cid, "chat restart button pressed");
                            chat::route_restart(chat_manager, cid).await;
                        }
                        _ => {
                            debug!(data, "ignoring unknown callback data");
                        }
                    }
                    continue;
                }

                let msg = &update["message"];
                let chat_id = msg["chat"]["id"].as_i64().unwrap_or(0);
                if chat_id != self.chat_id {
                    continue;
                }

                let text = match msg["text"].as_str() {
                    Some(t) => t,
                    None => continue,
                };

                // Check for /kill command FIRST (no reply-to required)
                if let Some(target) = text.strip_prefix("/kill") {
                    let target = target.trim();
                    if target.is_empty() || target == project_id {
                        info!("received /kill command for project {project_id}");
                        return Ok(TelegramPollResult::Kill);
                    }
                    debug!(
                        target_project = target,
                        our_project = project_id,
                        "ignoring /kill for different project"
                    );
                    continue;
                }

                // Check for /query command (no reply-to required)
                if let Some(target) = text.strip_prefix("/query") {
                    let target = target.trim();
                    if target.is_empty() || target == project_id {
                        info!("received /query command, sending stats");
                        self.notify(query_response).await;
                        continue;
                    }
                    debug!(
                        target_project = target,
                        our_project = project_id,
                        "ignoring /query for different project"
                    );
                    continue;
                }

                // Check for /chat command
                if let Some(rest) = text.strip_prefix("/chat ") {
                    let rest = rest.trim();
                    if let Some(pos) = rest.find(char::is_whitespace) {
                        let target = &rest[..pos];
                        let query = rest[pos..].trim();
                        if !query.is_empty() {
                            chat::try_start_chat(chat_manager, target, query).await;
                        }
                    }
                    continue;
                }

                // Route replies to active chat sessions
                let reply_to_msg_id =
                    msg["reply_to_message"]["message_id"].as_i64();
                if let Some(rid) = reply_to_msg_id {
                    if let Some(cid) = chat::lookup_chat(chat_manager, rid).await {
                        chat::route_reply(chat_manager, &cid, text.to_string()).await;
                        continue;
                    }
                }

                // Check reply-to matching
                match reply_to_msg_id {
                    Some(id) if id == expected_message_id => {
                        info!(
                            reply_len = text.len(),
                            "received matching telegram reply"
                        );
                        return Ok(TelegramPollResult::Reply(text.to_string()));
                    }
                    Some(id) => {
                        debug!(
                            expected = expected_message_id,
                            got = id,
                            "ignoring reply to different message"
                        );
                    }
                    None => {
                        let preview: String =
                            text.chars().take(50).collect();
                        debug!(
                            text_preview = preview,
                            "ignoring message without reply_to"
                        );
                    }
                }
            }
        }
    }

    /// Poll for a `/kill` command during Claude execution.
    ///
    /// Returns `Ok(())` when a matching `/kill` is found.
    /// `/query` commands are answered inline and polling continues.
    /// All other messages are consumed and discarded (offset advances).
    pub(crate) async fn poll_for_kill(
        &mut self,
        project_id: &str,
        query_response: &str,
        chat_manager: &chat::ChatManager,
    ) -> RexResult<()> {
        loop {
            let body = serde_json::json!({
                "offset": self.update_offset,
                "timeout": 1,
                "allowed_updates": ["message", "callback_query"],
            });

            let resp = self
                .http
                .post(self.api_url("getUpdates"))
                .json(&body)
                .timeout(Duration::from_secs(10))
                .send()
                .await;

            let json: Value = match resp {
                Ok(r) if r.status().is_success() => {
                    r.json().await.unwrap_or(Value::Null)
                }
                Ok(r) => {
                    warn!(
                        status = r.status().as_u16(),
                        "getUpdates error in kill poll, retrying"
                    );
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
                Err(e) => {
                    warn!(
                        "getUpdates request error in kill poll: {e}, retrying"
                    );
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            let updates = match json["result"].as_array() {
                Some(arr) => arr,
                None => continue,
            };

            for update in updates {
                if let Some(uid) = update["update_id"].as_i64() {
                    self.update_offset = uid + 1;
                }

                // Handle callback queries (inline button presses)
                if let Some(cb) = update.get("callback_query") {
                    let cb_chat_id = cb["message"]["chat"]["id"].as_i64().unwrap_or(0);
                    if cb_chat_id != self.chat_id {
                        continue;
                    }
                    let data = cb["data"].as_str().unwrap_or("");
                    let cb_id = cb["id"].as_str().unwrap_or("");
                    self.answer_callback_query(cb_id).await;

                    match data {
                        "kill" => {
                            info!("received kill button press during claude execution");
                            return Ok(());
                        }
                        "query" => {
                            info!("received stats button press during claude execution");
                            self.notify(query_response).await;
                        }
                        "reply" => {
                            info!("received reply button press during claude execution");
                            let _ = self.send_reply_prompt(project_id).await;
                        }
                        d if d.starts_with("cr:") => {
                            let cid = &d[3..];
                            info!(chat_id = cid, "chat reply button during execution");
                            if let Ok(prompt_id) = self.send_chat_reply_prompt(cid).await {
                                chat::register_message(chat_manager, prompt_id, cid).await;
                            }
                        }
                        d if d.starts_with("cx:") => {
                            let cid = &d[3..];
                            info!(chat_id = cid, "chat restart button during execution");
                            chat::route_restart(chat_manager, cid).await;
                        }
                        _ => {
                            debug!(data, "ignoring unknown callback data");
                        }
                    }
                    continue;
                }

                let msg = &update["message"];
                let chat_id = msg["chat"]["id"].as_i64().unwrap_or(0);
                if chat_id != self.chat_id {
                    continue;
                }

                if let Some(text) = msg["text"].as_str() {
                    if let Some(target) = text.strip_prefix("/kill") {
                        let target = target.trim();
                        if target.is_empty() || target == project_id {
                            info!(
                                "received /kill command during claude execution"
                            );
                            return Ok(());
                        }
                    }
                    if let Some(target) = text.strip_prefix("/query") {
                        let target = target.trim();
                        if target.is_empty() || target == project_id {
                            info!("received /query during claude execution");
                            self.notify(query_response).await;
                        }
                    }
                    if let Some(rest) = text.strip_prefix("/chat ") {
                        let rest = rest.trim();
                        if let Some(pos) = rest.find(char::is_whitespace) {
                            let target = &rest[..pos];
                            let query = rest[pos..].trim();
                            if !query.is_empty() {
                                chat::try_start_chat(chat_manager, target, query)
                                    .await;
                            }
                        }
                    }
                }

                // Route replies to active chat sessions
                if let Some(reply_to_id) =
                    msg["reply_to_message"]["message_id"].as_i64()
                {
                    if let Some(cid) =
                        chat::lookup_chat(chat_manager, reply_to_id).await
                    {
                        if let Some(t) = msg["text"].as_str() {
                            chat::route_reply(chat_manager, &cid, t.to_string())
                                .await;
                        }
                    }
                }
            }
        }
    }

    /// Fire-and-forget notification. Logs errors internally, never propagates.
    pub async fn notify(&self, text: &str) {
        if let Err(e) = self.send_message(text).await {
            error!("failed to send telegram notification: {e}");
        }
    }

    /// Fire-and-forget notification with inline Stats + Kill buttons.
    pub async fn notify_with_buttons(&self, text: &str) {
        if let Err(e) = self.send_with_buttons(text).await {
            error!("failed to send telegram notification with buttons: {e}");
        }
    }

    /// Acknowledge a callback query (removes the "loading" spinner on the button).
    async fn answer_callback_query(&self, callback_query_id: &str) {
        let body = serde_json::json!({
            "callback_query_id": callback_query_id,
        });
        let _ = self
            .http
            .post(self.api_url("answerCallbackQuery"))
            .json(&body)
            .timeout(Duration::from_secs(10))
            .send()
            .await;
    }
}

/// Exponential backoff: 10 * 2^(attempt-1) seconds, capped at 80.
fn backoff_delay(attempt: u32) -> Duration {
    let secs = (10u64 * 2u64.pow(attempt.saturating_sub(1))).min(80);
    Duration::from_secs(secs)
}
