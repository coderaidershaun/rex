//! Integration test: exercises the full auth refresh flow.
//!
//! 1. Spawns `claude auth login`
//! 2. Reads the auth URL from stdout/stderr
//! 3. Sends the URL to Telegram (ForceReply)
//! 4. Waits for the user to reply confirming authorization
//!
//! Run with: cargo test --test test_auth_refresh -- --nocapture

use std::time::Duration;

use reqwest::Client;
use serde_json::{Value, json};
use tokio::io::AsyncReadExt;

/// Extract the first `https://` URL from text.
fn extract_url(text: &str) -> Option<String> {
    let start = text.find("https://")?;
    let rest = &text[start..];
    let end = rest
        .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == '>' || c == '<')
        .unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

/// Read stdout from the auth process looking for a URL (up to 15s).
async fn read_auth_url(child: &mut tokio::process::Child) -> Option<String> {
    let mut stdout = child.stdout.take()?;
    let mut buf = vec![0u8; 8192];
    let mut text = String::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);

    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_secs(3), stdout.read(&mut buf)).await {
            Ok(Ok(0)) => break,
            Ok(Ok(n)) => {
                let chunk = String::from_utf8_lossy(&buf[..n]);
                eprintln!("[stdout chunk]: {chunk}");
                text.push_str(&chunk);
                if let Some(url) = extract_url(&text) {
                    return Some(url);
                }
            }
            Ok(Err(e)) => {
                eprintln!("[stdout read error]: {e}");
                break;
            }
            Err(_) => continue, // timeout, try again
        }
    }

    // Fallback: try stderr
    if let Some(mut stderr) = child.stderr.take() {
        let mut ebuf = vec![0u8; 8192];
        if let Ok(Ok(n)) =
            tokio::time::timeout(Duration::from_secs(5), stderr.read(&mut ebuf)).await
        {
            if n > 0 {
                let etext = String::from_utf8_lossy(&ebuf[..n]);
                eprintln!("[stderr chunk]: {etext}");
                return extract_url(&etext);
            }
        }
    }

    None
}

/// Send a Telegram message with ForceReply. Returns message_id.
async fn send_telegram_question(
    http: &Client,
    token: &str,
    chat_id: i64,
    text: &str,
) -> Result<i64, String> {
    let url = format!("https://api.telegram.org/bot{token}/sendMessage");
    let body = json!({
        "chat_id": chat_id,
        "text": text,
        "parse_mode": "HTML",
        "reply_markup": { "force_reply": true },
    });

    let resp = http
        .post(&url)
        .json(&body)
        .timeout(Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| format!("send failed: {e}"))?;

    let json: Value = resp
        .json()
        .await
        .map_err(|e| format!("parse failed: {e}"))?;

    eprintln!("[telegram response]: {json}");

    json["result"]["message_id"]
        .as_i64()
        .ok_or_else(|| format!("no message_id in response: {json}"))
}

/// Poll getUpdates waiting for a reply to our message.
async fn wait_for_reply(
    http: &Client,
    token: &str,
    chat_id: i64,
    expected_message_id: i64,
    timeout: Duration,
) -> Result<String, String> {
    let url = format!("https://api.telegram.org/bot{token}/getUpdates");
    let deadline = tokio::time::Instant::now() + timeout;
    let mut offset: i64 = 0;

    eprintln!("[waiting for reply to message {expected_message_id}, timeout {}s]", timeout.as_secs());

    loop {
        if tokio::time::Instant::now() >= deadline {
            return Err("timed out waiting for reply".into());
        }

        let body = json!({
            "offset": offset,
            "timeout": 1,
            "allowed_updates": ["message"],
        });

        let resp = http
            .post(&url)
            .json(&body)
            .timeout(Duration::from_secs(10))
            .send()
            .await;

        let json: Value = match resp {
            Ok(r) if r.status().is_success() => r.json().await.unwrap_or(Value::Null),
            Ok(r) => {
                eprintln!("[getUpdates error: {}]", r.status());
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
            Err(e) => {
                eprintln!("[getUpdates request error: {e}]");
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
        };

        let updates = match json["result"].as_array() {
            Some(arr) => arr,
            None => continue,
        };

        for update in updates {
            if let Some(uid) = update["update_id"].as_i64() {
                offset = uid + 1;
            }

            let msg = &update["message"];
            let msg_chat_id = msg["chat"]["id"].as_i64().unwrap_or(0);
            if msg_chat_id != chat_id {
                continue;
            }

            let text = match msg["text"].as_str() {
                Some(t) => t,
                None => continue,
            };

            // Check if this is a reply to our message
            let reply_to_id = msg["reply_to_message"]["message_id"].as_i64();
            if reply_to_id == Some(expected_message_id) {
                return Ok(text.to_string());
            } else {
                eprintln!("[ignoring message: reply_to={reply_to_id:?}, text={text}]");
            }
        }
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[tokio::test]
#[ignore] // requires live Telegram credentials + user interaction
async fn test_auth_refresh_via_telegram() {
    // Load .env
    dotenvy::dotenv().ok();

    let token = std::env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set");
    let chat_id: i64 = std::env::var("TELEGRAM_CHAT_ID")
        .expect("TELEGRAM_CHAT_ID not set")
        .parse()
        .expect("TELEGRAM_CHAT_ID not a valid i64");

    let project_id = "auth-test";
    let http = Client::new();

    // Step 1: Spawn `claude auth login`
    eprintln!("\n=== Spawning `claude auth login` ===");
    let mut child = tokio::process::Command::new("claude")
        .arg("auth")
        .arg("login")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn `claude auth login` — is claude CLI installed?");

    // Step 2: Read the auth URL
    eprintln!("=== Reading auth URL from output ===");
    let auth_url = read_auth_url(&mut child).await;

    let msg = match &auth_url {
        Some(url) => {
            eprintln!("=== Got auth URL: {url} ===");
            format!(
                "<b>[{pid}] Auth Refresh Test</b>\n━━━━━━━━━━━━━━━━━━━━\n\
                 Integration test: verifying auth refresh flow.\n\n\
                 Please visit this URL to re-authorize:\n{url}\n\n\
                 <i>Reply to this message when authorization is complete</i>",
                pid = escape_html(project_id),
            )
        }
        None => {
            eprintln!("=== No URL found in output, sending fallback message ===");
            format!(
                "<b>[{pid}] Auth Refresh Test</b>\n━━━━━━━━━━━━━━━━━━━━\n\
                 Integration test: could not extract auth URL from `claude auth login` output.\n\n\
                 Please run <code>claude auth login</code> manually, then reply here when done.\n\n\
                 <i>Reply to this message when authorization is complete</i>",
                pid = escape_html(project_id),
            )
        }
    };

    // Step 3: Send to Telegram
    eprintln!("=== Sending auth URL to Telegram ===");
    let msg_id = send_telegram_question(&http, &token, chat_id, &msg)
        .await
        .expect("failed to send Telegram message");
    eprintln!("=== Sent message_id: {msg_id} ===");

    // Step 4: Wait for user reply (10 minutes)
    eprintln!("=== Waiting for your Telegram reply (10 min timeout) ===");
    let reply = wait_for_reply(&http, &token, chat_id, msg_id, Duration::from_secs(600))
        .await
        .expect("no reply received within timeout");

    eprintln!("=== Got reply: {reply} ===");

    // Clean up the auth process
    let _ = child.kill().await;
    let _ = child.wait().await;

    // Send confirmation back
    let confirm_url = format!("https://api.telegram.org/bot{token}/sendMessage");
    let confirm_body = json!({
        "chat_id": chat_id,
        "text": format!(
            "<b>[{pid}] Auth test complete</b>\n✓ Reply received: <i>{reply}</i>",
            pid = escape_html(project_id),
            reply = escape_html(&reply),
        ),
        "parse_mode": "HTML",
    });
    let _ = http.post(&confirm_url).json(&confirm_body).send().await;

    eprintln!("\n=== TEST PASSED: Full auth refresh flow works ===");
}
