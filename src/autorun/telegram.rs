//! Telegram Bot API client with cooperative triage polling.
//!
//! When multiple autoruns share one bot token, a file lock ensures only one
//! polls `getUpdates` at a time. The poller triages updates and routes
//! cross-project messages via inbox files.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::errors::{RexError, RexResult};
use serde_json::Value;
use tracing::{debug, error, info, warn};

use super::inbox;

/// Outcome of polling for updates.
pub enum TelegramPollResult {
    /// A reply matching the expected message_id was received.
    Reply(String),
    /// A /kill command matching our project_id was received.
    Kill,
}

/// Raw HTTP client for the Telegram Bot API with cooperative triage.
pub struct TelegramClient {
    token: String,
    chat_id: i64,
    http: reqwest::Client,
    /// Tracks `getUpdates` offset to avoid replaying stale messages.
    pub update_offset: i64,
    /// Project root directory (contains registry and lock files).
    root_dir: PathBuf,
    /// This autorun's project ID.
    pub our_project_id: String,
}

impl TelegramClient {
    pub fn new(
        token: String,
        chat_id: i64,
        initial_offset: Option<i64>,
        root_dir: PathBuf,
        project_id: String,
    ) -> Self {
        Self {
            token,
            chat_id,
            http: reqwest::Client::new(),
            update_offset: initial_offset.unwrap_or(0),
            root_dir,
            our_project_id: project_id,
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
                        "auth error ({}): check REX_AUTORUN_TELEGRAM_BOT_TOKEN",
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

    // ── Sending methods ──────────────────────────────────────────────────

    /// Send a message and return the `message_id`. Retries with exponential backoff.
    pub async fn send_message(&self, text: &str) -> RexResult<i64> {
        let body = serde_json::json!({
            "chat_id": self.chat_id,
            "text": text,
            "parse_mode": "HTML",
        });
        self.send_with_body(&body).await
    }

    /// Send a message with inline Reply + Stats + Kill buttons on one row.
    pub async fn send_with_buttons(&self, text: &str, project_id: &str) -> RexResult<i64> {
        let body = serde_json::json!({
            "chat_id": self.chat_id,
            "text": text,
            "parse_mode": "HTML",
            "reply_markup": {
                "inline_keyboard": [[
                    { "text": "💬 Reply", "callback_data": format!("reply:{project_id}") },
                    { "text": "📊 Stats", "callback_data": format!("query:{project_id}") },
                    { "text": "🛑 Kill", "callback_data": format!("kill:{project_id}") },
                ]]
            },
        });
        self.send_with_body(&body).await
    }

    /// Send a force-reply prompt (used when the Reply button is pressed).
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

    // ── Receiving: cooperative triage polling ────────────────────────────

    /// Long-poll until a matching reply or `/kill` command arrives.
    ///
    /// Uses cooperative triage: acquires a file lock to poll Telegram exclusively,
    /// routes cross-project messages via inbox, falls back to inbox when lock
    /// is held by another autorun.
    pub(crate) async fn wait_for_reply(
        &mut self,
        mut expected_message_id: i64,
        project_id: &str,
        timeout: Duration,
        query_response: &str,
    ) -> RexResult<TelegramPollResult> {
        let deadline = tokio::time::Instant::now() + timeout;
        info!(
            timeout_secs = timeout.as_secs(),
            expected_message_id,
            "waiting for reply"
        );

        // Update registry with our expected message
        inbox::update_expected_message_id(&self.root_dir, project_id, Some(expected_message_id));

        loop {
            if tokio::time::Instant::now() >= deadline {
                return Err(RexError::Telegram(format!(
                    "human reply timeout exceeded ({} days)",
                    timeout.as_secs() / 86400
                )));
            }

            // Check inbox for messages routed by another autorun
            if let Some(msg) = inbox::read_inbox(&self.root_dir) {
                match msg {
                    inbox::InboxMessage::Reply { text } => {
                        info!(reply_len = text.len(), "received reply via inbox");
                        return Ok(TelegramPollResult::Reply(text));
                    }
                    inbox::InboxMessage::Kill => {
                        info!("received kill via inbox");
                        return Ok(TelegramPollResult::Kill);
                    }
                    inbox::InboxMessage::Query => {
                        info!("received query via inbox, sending stats");
                        self.notify(query_response).await;
                    }
                }
            }

            // Try to acquire poll lock
            let _lock = match inbox::try_acquire_poll_lock(&self.root_dir) {
                Some(guard) => guard,
                None => {
                    // Another autorun is polling — just check inbox and retry
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };

            // We hold the lock — poll Telegram
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

            // Load registry for cross-autorun routing
            let registry = inbox::load_registry(&self.root_dir);

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

                    // Parse "action:project_id" format
                    let (action, target_pid) = match data.split_once(':') {
                        Some((a, t)) => (a, t),
                        None => (data, ""),
                    };

                    // Route by target project_id
                    if !target_pid.is_empty() && target_pid != project_id {
                        // Route to another autorun via inbox
                        if let Some(target) = registry.autoruns.get(target_pid) {
                            let target_dir = Path::new(&target.project_dir);
                            match action {
                                "kill" => {
                                    let _ = inbox::write_inbox(target_dir, &inbox::InboxMessage::Kill);
                                }
                                "query" => {
                                    let _ = inbox::write_inbox(target_dir, &inbox::InboxMessage::Query);
                                }
                                "reply" => {
                                    // Send a reply prompt — this autorun handles it since
                                    // it's the one polling, then routes the actual reply later
                                    if let Ok(prompt_id) = self.send_reply_prompt(target_pid).await {
                                        // Register in registry so replies get routed correctly
                                        inbox::update_expected_message_id(
                                            &self.root_dir, target_pid, Some(prompt_id),
                                        );
                                    }
                                }
                                _ => {}
                            }
                        }
                        continue;
                    }

                    // Handle for our project
                    match action {
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
                                expected_message_id = prompt_id;
                                inbox::update_expected_message_id(
                                    &self.root_dir, project_id, Some(prompt_id),
                                );
                            }
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

                // Check for /kill command
                if let Some(target) = text.strip_prefix("/kill") {
                    let target = target.trim();
                    if target.is_empty() || target == project_id {
                        info!("received /kill command for project {project_id}");
                        return Ok(TelegramPollResult::Kill);
                    }
                    // Route to another autorun
                    if let Some(entry) = registry.autoruns.get(target) {
                        let target_dir = Path::new(&entry.project_dir);
                        let _ = inbox::write_inbox(target_dir, &inbox::InboxMessage::Kill);
                    }
                    continue;
                }

                // Check for /query command
                if let Some(target) = text.strip_prefix("/query") {
                    let target = target.trim();
                    if target.is_empty() || target == project_id {
                        info!("received /query command, sending stats");
                        self.notify(query_response).await;
                        continue;
                    }
                    // Route to another autorun
                    if let Some(entry) = registry.autoruns.get(target) {
                        let target_dir = Path::new(&entry.project_dir);
                        let _ = inbox::write_inbox(target_dir, &inbox::InboxMessage::Query);
                    }
                    continue;
                }

                // /clear command
                if text.starts_with("/clear") {
                    info!("received /clear command");
                    self.clear_history().await;
                    continue;
                }

                // /commands, /start, /menu — show available commands
                if text.starts_with("/commands")
                    || text.starts_with("/start")
                    || text.starts_with("/menu")
                {
                    self.send_commands_help().await;
                    continue;
                }

                // Check reply-to matching
                let reply_to_msg_id =
                    msg["reply_to_message"]["message_id"].as_i64();
                match reply_to_msg_id {
                    Some(id) if id == expected_message_id => {
                        info!(
                            reply_len = text.len(),
                            "received matching telegram reply"
                        );
                        return Ok(TelegramPollResult::Reply(text.to_string()));
                    }
                    Some(id) => {
                        // Check if this reply is for another autorun
                        let mut routed = false;
                        for (pid, entry) in &registry.autoruns {
                            if pid != project_id
                                && entry.expected_message_id == Some(id)
                            {
                                let target_dir = Path::new(&entry.project_dir);
                                let _ = inbox::write_inbox(
                                    target_dir,
                                    &inbox::InboxMessage::Reply {
                                        text: text.to_string(),
                                    },
                                );
                                info!(
                                    target_project = %pid,
                                    "routed reply to another autorun via inbox"
                                );
                                routed = true;
                                break;
                            }
                        }
                        if !routed {
                            debug!(
                                expected = expected_message_id,
                                got = id,
                                "ignoring reply to unknown message"
                            );
                        }
                    }
                    None => {
                        // No reply-to — send "unprocessed" response
                        self.send_unprocessed_response(&registry).await;
                    }
                }
            }
            // Lock is released here when _lock goes out of scope
        }
    }

    /// Poll for a `/kill` command during Claude execution.
    ///
    /// Uses cooperative triage: acquires file lock to poll, routes cross-project
    /// messages via inbox.
    pub(crate) async fn poll_for_kill(
        &mut self,
        project_id: &str,
        query_response: &str,
    ) -> RexResult<()> {
        loop {
            // Check inbox for messages routed by another autorun
            if let Some(msg) = inbox::read_inbox(&self.root_dir) {
                match msg {
                    inbox::InboxMessage::Kill => {
                        info!("received kill via inbox during execution");
                        return Ok(());
                    }
                    inbox::InboxMessage::Query => {
                        info!("received query via inbox during execution");
                        self.notify(query_response).await;
                    }
                    inbox::InboxMessage::Reply { .. } => {
                        debug!("ignoring inbox reply during execution");
                    }
                }
            }

            // Try to acquire poll lock
            let _lock = match inbox::try_acquire_poll_lock(&self.root_dir) {
                Some(guard) => guard,
                None => {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };

            // We hold the lock — poll Telegram
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

            // Load registry for cross-autorun routing
            let registry = inbox::load_registry(&self.root_dir);

            for update in updates {
                if let Some(uid) = update["update_id"].as_i64() {
                    self.update_offset = uid + 1;
                }

                // Handle callback queries
                if let Some(cb) = update.get("callback_query") {
                    let cb_chat_id = cb["message"]["chat"]["id"].as_i64().unwrap_or(0);
                    if cb_chat_id != self.chat_id {
                        continue;
                    }
                    let data = cb["data"].as_str().unwrap_or("");
                    let cb_id = cb["id"].as_str().unwrap_or("");
                    self.answer_callback_query(cb_id).await;

                    let (action, target_pid) = match data.split_once(':') {
                        Some((a, t)) => (a, t),
                        None => (data, ""),
                    };

                    // Route to another autorun if target doesn't match
                    if !target_pid.is_empty() && target_pid != project_id {
                        if let Some(target) = registry.autoruns.get(target_pid) {
                            let target_dir = Path::new(&target.project_dir);
                            match action {
                                "kill" => {
                                    let _ = inbox::write_inbox(target_dir, &inbox::InboxMessage::Kill);
                                }
                                "query" => {
                                    let _ = inbox::write_inbox(target_dir, &inbox::InboxMessage::Query);
                                }
                                "reply" => {
                                    let _ = self.send_reply_prompt(target_pid).await;
                                }
                                _ => {}
                            }
                        }
                        continue;
                    }

                    match action {
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
                    // /kill command
                    if let Some(target) = text.strip_prefix("/kill") {
                        let target = target.trim();
                        if target.is_empty() || target == project_id {
                            info!(
                                "received /kill command during claude execution"
                            );
                            return Ok(());
                        }
                        if let Some(entry) = registry.autoruns.get(target) {
                            let target_dir = Path::new(&entry.project_dir);
                            let _ = inbox::write_inbox(target_dir, &inbox::InboxMessage::Kill);
                        }
                    }
                    // /query command
                    else if let Some(target) = text.strip_prefix("/query") {
                        let target = target.trim();
                        if target.is_empty() || target == project_id {
                            info!("received /query during claude execution");
                            self.notify(query_response).await;
                        } else if let Some(entry) = registry.autoruns.get(target) {
                            let target_dir = Path::new(&entry.project_dir);
                            let _ = inbox::write_inbox(target_dir, &inbox::InboxMessage::Query);
                        }
                    }
                    // /clear command
                    else if text.starts_with("/clear") {
                        self.clear_history().await;
                    }
                    // /commands, /start, /menu
                    else if text.starts_with("/commands")
                        || text.starts_with("/start")
                        || text.starts_with("/menu")
                    {
                        self.send_commands_help().await;
                    }
                    // Reply-to messages — route to correct autorun
                    else if let Some(reply_to_id) = msg["reply_to_message"]["message_id"].as_i64() {
                        for (pid, entry) in &registry.autoruns {
                            if entry.expected_message_id == Some(reply_to_id) {
                                let target_dir = Path::new(&entry.project_dir);
                                let _ = inbox::write_inbox(
                                    target_dir,
                                    &inbox::InboxMessage::Reply { text: text.to_string() },
                                );
                                debug!(target_project = %pid, "routed reply during execution");
                                break;
                            }
                        }
                    }
                    // Bare message — send unprocessed
                    else {
                        self.send_unprocessed_response(&registry).await;
                    }
                }
            }
            // Lock is released here when _lock goes out of scope
        }
    }

    // ── Delete / clear ────────────────────────────────────────────────────

    /// Delete a message by ID. Silently ignores failures.
    async fn delete_message(&self, message_id: i64) {
        let body = serde_json::json!({
            "chat_id": self.chat_id,
            "message_id": message_id,
        });
        let _ = self
            .http
            .post(self.api_url("deleteMessage"))
            .json(&body)
            .timeout(Duration::from_secs(5))
            .send()
            .await;
    }

    /// Clear chat history by deleting recent messages.
    async fn clear_history(&self) {
        let marker_id = match self.send_message("🗑 Clearing...").await {
            Ok(id) => id,
            Err(_) => return,
        };
        for msg_id in (1..marker_id).rev().take(200) {
            self.delete_message(msg_id).await;
        }
        self.delete_message(marker_id).await;
    }

    /// Send the list of available commands.
    async fn send_commands_help(&self) {
        self.notify(
            "📋 <b>Autorun Commands</b>\n\n\
             <code>/kill &lt;project-id&gt;</code> — Stop an autorun\n\
             <code>/query &lt;project-id&gt;</code> — Show live stats\n\
             <code>/commands</code> — Show this help\n\
             <code>/clear</code> — Clear chat history",
        )
        .await;
    }

    // ── Notification helpers ─────────────────────────────────────────────

    /// Fire-and-forget notification. Logs errors internally, never propagates.
    pub async fn notify(&self, text: &str) {
        if let Err(e) = self.send_message(text).await {
            error!("failed to send telegram notification: {e}");
        }
    }

    /// Fire-and-forget notification with inline Stats + Kill buttons (no Reply).
    pub async fn notify_with_status_buttons(&self, text: &str, project_id: &str) {
        let body = serde_json::json!({
            "chat_id": self.chat_id,
            "text": text,
            "parse_mode": "HTML",
            "reply_markup": {
                "inline_keyboard": [[
                    { "text": "📊 Stats", "callback_data": format!("query:{project_id}") },
                    { "text": "🛑 Kill", "callback_data": format!("kill:{project_id}") },
                ]]
            },
        });
        if let Err(e) = self.send_with_body(&body).await {
            error!("failed to send telegram notification with status buttons: {e}");
        }
    }

    /// Fire-and-forget notification with inline Reply + Stats + Kill buttons.
    pub async fn notify_with_buttons(&self, text: &str, project_id: &str) {
        if let Err(e) = self.send_with_buttons(text, project_id).await {
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

    /// Send an "unprocessed" response listing active autoruns.
    async fn send_unprocessed_response(&self, registry: &inbox::AutorunRegistry) {
        let mut msg = "⚠️ <b>Unprocessed</b>\n\nPlease reply to a specific message, or use:\n\
            <code>/kill &lt;project-id&gt;</code>\n\
            <code>/query &lt;project-id&gt;</code>\n"
            .to_string();

        if registry.autoruns.is_empty() {
            msg.push_str("\n<i>No active autoruns.</i>");
        } else {
            msg.push_str("\n<b>Active autoruns:</b>\n");
            for (pid, _entry) in &registry.autoruns {
                msg.push_str(&format!(
                    "  <code>{}</code>\n",
                    super::types::escape_html(pid),
                ));
            }
        }

        self.notify(&msg).await;
    }
}

/// Exponential backoff: 10 * 2^(attempt-1) seconds, capped at 80.
fn backoff_delay(attempt: u32) -> Duration {
    let secs = (10u64 * 2u64.pow(attempt.saturating_sub(1))).min(80);
    Duration::from_secs(secs)
}
