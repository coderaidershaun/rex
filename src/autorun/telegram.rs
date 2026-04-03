//! Telegram Bot API client with polling, retry logic, and command handling.

use std::time::Duration;

use crate::errors::{RexError, RexResult};
use serde_json::Value;
use tracing::{debug, error, info, warn};

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

    /// Long-poll `getUpdates` until a matching reply or `/kill` command arrives.
    ///
    /// Only accepts messages that are replies to `expected_message_id`.
    /// Non-matching messages are logged and skipped.
    /// `/query` commands are answered inline and polling continues.
    pub async fn wait_for_reply(
        &mut self,
        expected_message_id: i64,
        project_id: &str,
        timeout: Duration,
        query_response: &str,
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
                "allowed_updates": ["message"],
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
    pub async fn poll_for_kill(
        &mut self,
        project_id: &str,
        query_response: &str,
    ) -> RexResult<()> {
        loop {
            let body = serde_json::json!({
                "offset": self.update_offset,
                "timeout": 1,
                "allowed_updates": ["message"],
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
}

/// Exponential backoff: 10 * 2^(attempt-1) seconds, capped at 80.
fn backoff_delay(attempt: u32) -> Duration {
    let secs = (10u64 * 2u64.pow(attempt.saturating_sub(1))).min(80);
    Duration::from_secs(secs)
}
