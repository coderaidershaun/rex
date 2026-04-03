//! Integration test: sends realistic autorun Telegram messages (completion +
//! needs_input question) using the real TelegramClient, then waits for a reply.
//!
//! Run with: cargo test --test test_auth_refresh -- --nocapture --include-ignored

use std::time::Duration;
use rex_cli::autorun::telegram::TelegramClient;

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Flush all pending Telegram updates and return the offset that skips past them.
async fn flush_updates(token: &str) -> Option<i64> {
    let http = reqwest::Client::new();
    let url = format!("https://api.telegram.org/bot{token}/getUpdates");

    // offset -1 returns just the most recent update
    let body = serde_json::json!({ "offset": -1, "timeout": 0 });
    let resp = http.post(&url).json(&body).send().await.ok()?;
    let json: serde_json::Value = resp.json().await.ok()?;

    if let Some(updates) = json["result"].as_array() {
        if let Some(last) = updates.last() {
            let uid = last["update_id"].as_i64()?;
            // Confirm it by requesting offset = uid + 1
            let confirm = serde_json::json!({ "offset": uid + 1, "timeout": 0 });
            let _ = http.post(&url).json(&confirm).send().await;
            eprintln!("[flushed updates, offset now {}]", uid + 1);
            return Some(uid + 1);
        }
    }
    eprintln!("[no stale updates to flush]");
    None
}

#[tokio::test]
#[ignore] // requires live Telegram credentials + user interaction
async fn test_agent_messages_via_telegram() {
    dotenvy::dotenv().ok();

    let token = std::env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set");
    let chat_id: i64 = std::env::var("TELEGRAM_CHAT_ID")
        .expect("TELEGRAM_CHAT_ID not set")
        .parse()
        .expect("TELEGRAM_CHAT_ID not a valid i64");

    let project_id = "msg-test";

    // Flush all stale updates BEFORE creating the client
    let flushed_offset = flush_updates(&token).await;
    let mut tg = TelegramClient::new(token, chat_id, flushed_offset);

    // ── 1. Simulated completion notification (fire-and-forget, no reply needed) ──
    let completion_header = "<b>claude-sonnet-4-5-20250514</b>  |  67.3 tok/s  |  42.1% context";
    let completion_msg = format!(
        "{header}\n━━━━━━━━━━━━━━━━━━━━\n<b>[{pid}] Completed #3</b>\nImplemented orderbook matching engine with price-time priority\n\n<b>Cost:</b> $0.47  |  <b>Duration:</b> 38s",
        header = completion_header,
        pid = escape_html(project_id),
    );

    eprintln!("=== Sending completion notification ===");
    tg.notify(&completion_msg).await;
    eprintln!("=== Completion sent ===");

    // Small pause so messages arrive in order
    tokio::time::sleep(Duration::from_millis(500)).await;

    // ── 2. Simulated needs_input question (ForceReply, waits for reply) ──
    let question_header = "<b>claude-sonnet-4-5-20250514</b>  |  71.0 tok/s  |  55.8% context";
    let question = "The WebSocket feed requires authentication. Should I use API key auth (simpler, less secure) or OAuth2 (more complex, production-grade)? Also, do you want reconnection logic with exponential backoff, or just fail-fast on disconnect?";
    let question_msg = format!(
        "{header}\n━━━━━━━━━━━━━━━━━━━━\n<b>[{pid}] Input needed</b>\n\n{q}\n\n<i>Reply to this message with your answer</i>",
        header = question_header,
        pid = escape_html(project_id),
        q = escape_html(question),
    );

    eprintln!("=== Sending needs_input question ===");
    let msg_id = tg.send_question(&question_msg).await.expect("failed to send question");
    eprintln!("=== Sent message_id: {msg_id} ===");

    // ── 3. Wait for reply (1 minute timeout) ──
    eprintln!("=== Waiting for reply (1 min timeout) ===");
    let query_response = format!(
        "<b>[{pid}]</b>\nRunning invocation #4  |  Cost so far: $1.23  |  Uptime: 12m",
        pid = escape_html(project_id),
    );
    let result = tg
        .wait_for_reply(msg_id, project_id, Duration::from_secs(60), &query_response)
        .await;

    match result {
        Ok(rex_cli::autorun::telegram::TelegramPollResult::Reply(reply)) => {
            eprintln!("=== Got reply: {reply} ===");

            // Send ack — same as production send_ack
            let ack_msgs = [
                "Got it — resuming",
                "Roger that — on it",
                "Understood — continuing",
                "Copy that — proceeding",
            ];
            let ack = ack_msgs[0];
            tg.notify(&format!(
                "<b>[{pid}]</b>\n{ack}",
                pid = escape_html(project_id),
            )).await;

            eprintln!("\n=== TEST PASSED ===");
        }
        Ok(rex_cli::autorun::telegram::TelegramPollResult::Kill) => {
            panic!("Received /kill — test aborted");
        }
        Err(e) => {
            panic!("wait_for_reply failed: {e}");
        }
    }
}
