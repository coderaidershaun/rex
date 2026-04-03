//! Integration test: sends realistic autorun Telegram messages using teloxide
//! with inline keyboards and rich formatting, then waits for a reply.
//!
//! Run with: cargo test --test test_auth_refresh -- --nocapture --include-ignored

use std::time::Duration;

use rex_cli::autorun::telegram::TelegramClient;
use teloxide::prelude::*;
use teloxide::types::{
    ChatId, InlineKeyboardButton, InlineKeyboardMarkup, ParseMode, ReplyMarkup,
};

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Flush all pending Telegram updates and return the offset that skips past them.
async fn flush_updates(token: &str) -> Option<i64> {
    let http = reqwest::Client::new();
    let url = format!("https://api.telegram.org/bot{token}/getUpdates");
    let body = serde_json::json!({ "offset": -1, "timeout": 0 });
    let resp = http.post(&url).json(&body).send().await.ok()?;
    let json: serde_json::Value = resp.json().await.ok()?;
    if let Some(updates) = json["result"].as_array() {
        if let Some(last) = updates.last() {
            let uid = last["update_id"].as_i64()?;
            let confirm = serde_json::json!({ "offset": uid + 1, "timeout": 0 });
            let _ = http.post(&url).json(&confirm).send().await;
            eprintln!("[flushed updates, offset now {}]", uid + 1);
            return Some(uid + 1);
        }
    }
    None
}

#[tokio::test]
#[ignore]
async fn test_agent_messages_via_telegram() {
    dotenvy::dotenv().ok();

    let token = std::env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set");
    let chat_id_raw: i64 = std::env::var("TELEGRAM_CHAT_ID")
        .expect("TELEGRAM_CHAT_ID not set")
        .parse()
        .expect("TELEGRAM_CHAT_ID not a valid i64");

    let project_id = "msg-test";
    let chat_id = ChatId(chat_id_raw);

    // Teloxide bot for sending rich messages
    let bot = Bot::new(&token);

    // Flush stale updates, then create TelegramClient for reply polling
    let flushed_offset = flush_updates(&token).await;
    let mut tg = TelegramClient::new(token, chat_id_raw, flushed_offset);

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 1. STARTUP notification
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    let startup_msg = format!(
        "🚀 <b>Autorun started</b>  ·  <code>{pid}</code>\n\
         ━━━━━━━━━━━━━━━━━━━━\n\
         📂 <b>Project:</b> Orderbook Engine\n\
         📁 <b>Directory:</b> <code>/srv/projects/orderbook</code>\n\
         🤖 <b>Model:</b> claude-sonnet-4-5",
        pid = escape_html(project_id),
    );

    eprintln!("=== Sending startup notification ===");
    bot.send_message(chat_id, &startup_msg)
        .parse_mode(ParseMode::Html)
        .await
        .expect("failed to send startup");

    tokio::time::sleep(Duration::from_millis(800)).await;

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 2. COMPLETION notification with inline stats button
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    let completion_msg = format!(
        "✅ <b>Completed #3</b>  ·  <code>{pid}</code>\n\
         ━━━━━━━━━━━━━━━━━━━━\n\
         Implemented orderbook matching engine with price-time priority\n\n\
         ⚡ <code>67.3 tok/s</code>  ·  📊 <code>42.1%</code> context\n\
         💰 <code>$0.47</code>  ·  ⏱ <code>38s</code>",
        pid = escape_html(project_id),
    );

    let stats_keyboard = InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback("📊 Stats", "query"),
        InlineKeyboardButton::callback("🛑 Kill", "kill"),
    ]]);

    eprintln!("=== Sending completion notification ===");
    bot.send_message(chat_id, &completion_msg)
        .parse_mode(ParseMode::Html)
        .reply_markup(stats_keyboard)
        .await
        .expect("failed to send completion");

    tokio::time::sleep(Duration::from_millis(800)).await;

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 3. NEEDS_INPUT question with ForceReply + inline keyboard
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    let question = "The WebSocket feed requires authentication. Should I use \
                    API key auth (simpler, less secure) or OAuth2 (more complex, \
                    production-grade)? Also, do you want reconnection logic with \
                    exponential backoff, or just fail-fast on disconnect?";

    let question_msg = format!(
        "💬 <b>Input needed</b>  ·  <code>{pid}</code>\n\
         ━━━━━━━━━━━━━━━━━━━━\n\
         ⚡ <code>71.0 tok/s</code>  ·  📊 <code>55.8%</code> context\n\n\
         <blockquote>{q}</blockquote>\n\
         <i>Reply to this message with your answer</i>",
        pid = escape_html(project_id),
        q = escape_html(question),
    );

    eprintln!("=== Sending needs_input question ===");
    let sent = bot
        .send_message(chat_id, &question_msg)
        .parse_mode(ParseMode::Html)
        .reply_markup(ReplyMarkup::ForceReply(
            teloxide::types::ForceReply::new(),
        ))
        .await
        .expect("failed to send question");
    let msg_id = sent.id.0 as i64;
    eprintln!("=== Sent message_id: {msg_id} ===");

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // 4. Wait for reply using production TelegramClient (1 min timeout)
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    eprintln!("=== Waiting for reply (1 min timeout) ===");
    let query_response = format!(
        "📊 <b>Stats</b>  ·  <code>{pid}</code>\n\
         ━━━━━━━━━━━━━━━━━━━━\n\
         🔄 Invocation <code>#4</code>\n\
         💰 Cost so far: <code>$1.23</code>\n\
         ⏱ Uptime: <code>12m</code>",
        pid = escape_html(project_id),
    );
    let result = tg
        .wait_for_reply(msg_id, project_id, Duration::from_secs(60), &query_response)
        .await;

    match result {
        Ok(rex_cli::autorun::telegram::TelegramPollResult::Reply(reply)) => {
            eprintln!("=== Got reply: {reply} ===");

            // Send ack
            let ack_msg = format!(
                "👍 <b>Got it — resuming</b>  ·  <code>{pid}</code>",
                pid = escape_html(project_id),
            );
            bot.send_message(chat_id, &ack_msg)
                .parse_mode(ParseMode::Html)
                .await
                .expect("failed to send ack");

            tokio::time::sleep(Duration::from_millis(800)).await;

            // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
            // 5. PROJECT DONE summary
            // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
            let done_msg = format!(
                "🏁 <b>Project complete!</b>  ·  <code>{pid}</code>\n\
                 ━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n\n\
                 🔄 Invocations: <code>7</code>\n\
                 💰 Total cost:  <code>$2.34</code>\n\
                 ⏱ Duration:    <code>14m 22s</code>\n\
                 🤖 Model:       <code>claude-sonnet-4-5</code>",
                pid = escape_html(project_id),
            );
            bot.send_message(chat_id, &done_msg)
                .parse_mode(ParseMode::Html)
                .await
                .expect("failed to send done");

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
