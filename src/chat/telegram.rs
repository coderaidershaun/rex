//! Telegram Bot API client for rex-chat daemon.
//!
//! Focused on inline keyboards, ForceReply, polling, and message routing.

use std::time::Duration;

use crate::errors::{RexError, RexResult};
use serde_json::Value;
use tracing::{debug, error, warn};

/// Inline keyboard button for Telegram.
pub struct InlineButton {
    pub text: String,
    pub callback_data: String,
}

/// A parsed Telegram update.
pub enum Update {
    /// Callback query from an inline keyboard button press.
    CallbackQuery {
        id: String,
        data: String,
        message_id: i64,
    },
    /// Text message from the user.
    TextMessage {
        text: String,
        reply_to_message_id: Option<i64>,
    },
}

/// Telegram client for rex-chat.
pub struct ChatTelegramClient {
    token: String,
    chat_id: i64,
    http: reqwest::Client,
    pub update_offset: i64,
}

impl ChatTelegramClient {
    pub fn new(token: String, chat_id: i64, initial_offset: i64) -> Self {
        Self {
            token,
            chat_id,
            http: reqwest::Client::new(),
            update_offset: initial_offset,
        }
    }

    fn api_url(&self, method: &str) -> String {
        format!("https://api.telegram.org/bot{}/{}", self.token, method)
    }

    /// POST to sendMessage with retry logic.
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
                        RexError::Telegram(format!("failed to parse sendMessage response: {e}"))
                    })?;
                    let msg_id = json["result"]["message_id"].as_i64().ok_or_else(|| {
                        RexError::Telegram("missing message_id in sendMessage response".into())
                    })?;
                    return Ok(msg_id);
                }
                Ok(r) if r.status() == 401 || r.status() == 403 => {
                    return Err(RexError::Telegram(format!(
                        "auth error ({}): check REX_AUTOCHAT_TELEGRAM_BOT_TOKEN",
                        r.status()
                    )));
                }
                Ok(r)
                    if (r.status() == 429 || r.status().is_server_error()) && attempt <= 5 =>
                {
                    let delay = backoff_delay(attempt);
                    warn!(status = r.status().as_u16(), attempt, "telegram API error, retrying");
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
                    warn!(attempt, "telegram request error: {e}, retrying");
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

    // ── Sending ─────────────────────────────────────────────────────────

    /// Send a plain HTML message.
    pub async fn send_message(&self, text: &str) -> RexResult<i64> {
        let body = serde_json::json!({
            "chat_id": self.chat_id,
            "text": text,
            "parse_mode": "HTML",
        });
        self.send_with_body(&body).await
    }

    /// Send a message with vertical inline keyboard buttons (one per row).
    pub async fn send_with_buttons(
        &self,
        text: &str,
        buttons: &[InlineButton],
    ) -> RexResult<i64> {
        let keyboard: Vec<Vec<serde_json::Value>> = buttons
            .iter()
            .map(|b| {
                vec![serde_json::json!({
                    "text": b.text,
                    "callback_data": b.callback_data,
                })]
            })
            .collect();

        let body = serde_json::json!({
            "chat_id": self.chat_id,
            "text": text,
            "parse_mode": "HTML",
            "reply_markup": {
                "inline_keyboard": keyboard,
            },
        });
        self.send_with_body(&body).await
    }

    /// Send a ForceReply prompt.
    pub async fn send_force_reply(&self, text: &str) -> RexResult<i64> {
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

    /// Edit an existing message and add buttons.
    pub async fn edit_message_with_buttons(
        &self,
        message_id: i64,
        text: &str,
        buttons: &[InlineButton],
    ) -> RexResult<()> {
        let keyboard: Vec<Vec<serde_json::Value>> = buttons
            .iter()
            .map(|b| {
                vec![serde_json::json!({
                    "text": b.text,
                    "callback_data": b.callback_data,
                })]
            })
            .collect();

        let body = serde_json::json!({
            "chat_id": self.chat_id,
            "message_id": message_id,
            "text": text,
            "parse_mode": "HTML",
            "reply_markup": {
                "inline_keyboard": keyboard,
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

    /// Fire-and-forget notification.
    pub async fn notify(&self, text: &str) {
        if let Err(e) = self.send_message(text).await {
            error!("failed to send telegram notification: {e}");
        }
    }

    /// Acknowledge a callback query.
    pub async fn answer_callback_query(&self, callback_query_id: &str) {
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

    // ── Polling ─────────────────────────────────────────────────────────

    /// Long-poll `getUpdates` and return parsed updates.
    /// Returns an empty vec on transient errors (retries internally).
    pub async fn poll_updates(&mut self) -> Vec<Update> {
        let body = serde_json::json!({
            "offset": self.update_offset,
            "timeout": 2,
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
            Ok(r) if r.status().is_success() => r.json().await.unwrap_or(Value::Null),
            Ok(r) => {
                warn!(status = r.status().as_u16(), "getUpdates error");
                tokio::time::sleep(Duration::from_secs(5)).await;
                return vec![];
            }
            Err(e) => {
                warn!("getUpdates request error: {e}");
                tokio::time::sleep(Duration::from_secs(5)).await;
                return vec![];
            }
        };

        let Some(updates) = json["result"].as_array() else {
            return vec![];
        };

        let mut result = Vec::new();

        for update in updates {
            if let Some(uid) = update["update_id"].as_i64() {
                self.update_offset = uid + 1;
            }

            // Callback query
            if let Some(cb) = update.get("callback_query") {
                let cb_chat_id = cb["message"]["chat"]["id"].as_i64().unwrap_or(0);
                if cb_chat_id != self.chat_id {
                    continue;
                }
                let data = cb["data"].as_str().unwrap_or("").to_string();
                let id = cb["id"].as_str().unwrap_or("").to_string();
                let message_id = cb["message"]["message_id"].as_i64().unwrap_or(0);
                result.push(Update::CallbackQuery {
                    id,
                    data,
                    message_id,
                });
                continue;
            }

            // Text message
            let msg = &update["message"];
            let msg_chat_id = msg["chat"]["id"].as_i64().unwrap_or(0);
            if msg_chat_id != self.chat_id {
                continue;
            }
            if let Some(text) = msg["text"].as_str() {
                let reply_to = msg["reply_to_message"]["message_id"].as_i64();
                result.push(Update::TextMessage {
                    text: text.to_string(),
                    reply_to_message_id: reply_to,
                });
            }
        }

        debug!(count = result.len(), "polled updates");
        result
    }
}

/// Exponential backoff: 10 * 2^(attempt-1) seconds, capped at 80.
fn backoff_delay(attempt: u32) -> Duration {
    let secs = (10u64 * 2u64.pow(attempt.saturating_sub(1))).min(80);
    Duration::from_secs(secs)
}
