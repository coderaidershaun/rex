use std::time::Duration;

use anyhow::{bail, Context, Result};
use serde_json::Value;
use tracing::{debug, error, info, warn};

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

    /// Send a message and return the `message_id`. Retries with exponential backoff.
    pub async fn send_message(&self, text: &str) -> Result<i64> {
        let body = serde_json::json!({
            "chat_id": self.chat_id,
            "text": text,
        });

        let mut attempt = 0u32;
        loop {
            attempt += 1;
            let resp = self.http
                .post(self.api_url("sendMessage"))
                .json(&body)
                .timeout(Duration::from_secs(30))
                .send()
                .await;

            match resp {
                Ok(r) if r.status().is_success() => {
                    let json: Value = r.json().await
                        .context("failed to parse sendMessage response")?;
                    let msg_id = json["result"]["message_id"]
                        .as_i64()
                        .context("missing message_id in sendMessage response")?;
                    debug!(msg_id, "telegram message sent");
                    return Ok(msg_id);
                }
                Ok(r) if r.status() == 401 || r.status() == 403 => {
                    bail!(
                        "telegram auth error ({}): check TELEGRAM_BOT_TOKEN",
                        r.status()
                    );
                }
                Ok(r) if (r.status() == 429 || r.status().is_server_error()) && attempt <= 5 => {
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
                    bail!("telegram sendMessage failed ({status}): {body_text}");
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
                    bail!("telegram sendMessage failed after {attempt} attempts: {e}");
                }
            }
        }
    }

    /// Long-poll `getUpdates` until a reply arrives from the correct chat, or timeout.
    pub async fn wait_for_reply(&mut self, timeout: Duration) -> Result<String> {
        let deadline = tokio::time::Instant::now() + timeout;
        info!(
            timeout_secs = timeout.as_secs(),
            "waiting for telegram reply"
        );

        loop {
            if tokio::time::Instant::now() >= deadline {
                bail!("human reply timeout exceeded ({} days)", timeout.as_secs() / 86400);
            }

            let body = serde_json::json!({
                "offset": self.update_offset,
                "timeout": 30,
                "allowed_updates": ["message"],
            });

            let resp = self.http
                .post(self.api_url("getUpdates"))
                .json(&body)
                .timeout(Duration::from_secs(45)) // slightly longer than Telegram's 30s long-poll
                .send()
                .await;

            let json: Value = match resp {
                Ok(r) if r.status().is_success() => {
                    r.json().await.unwrap_or(Value::Null)
                }
                Ok(r) => {
                    warn!(status = r.status().as_u16(), "getUpdates error, retrying");
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
                // Always advance the offset past every update we see
                if let Some(uid) = update["update_id"].as_i64() {
                    self.update_offset = uid + 1;
                }

                let msg = &update["message"];
                let chat_id = msg["chat"]["id"].as_i64().unwrap_or(0);
                if chat_id != self.chat_id {
                    continue;
                }

                if let Some(text) = msg["text"].as_str() {
                    info!(reply_len = text.len(), "received telegram reply");
                    return Ok(text.to_string());
                }
            }
        }
    }

    /// Fire-and-forget notification. Logs errors internally, never propagates.
    pub async fn notify(&self, text: &str) {
        match self.send_message(text).await {
            Ok(_) => {}
            Err(e) => {
                error!("failed to send telegram notification: {e}");
            }
        }
    }
}

/// Exponential backoff: 10 * 2^(attempt-1) seconds, capped at 80.
fn backoff_delay(attempt: u32) -> Duration {
    let secs = (10u64 * 2u64.pow(attempt.saturating_sub(1))).min(80);
    Duration::from_secs(secs)
}
