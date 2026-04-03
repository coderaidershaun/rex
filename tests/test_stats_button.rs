//! Integration test: sends a completion message with inline Stats/Kill buttons,
//! then polls for callback queries and responds when the user presses a button.
//!
//! Run with: cargo test --test test_stats_button -- --nocapture --include-ignored

use std::time::Duration;
use teloxide::prelude::*;
use teloxide::types::{
    ChatId, InlineKeyboardButton, InlineKeyboardMarkup, ParseMode,
};

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

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
            return Some(uid + 1);
        }
    }
    None
}

const DIV: &str = "⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯";

#[tokio::test]
#[ignore]
async fn test_stats_button() {
    dotenvy::dotenv().ok();

    let token = std::env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set");
    let chat_id_raw: i64 = std::env::var("TELEGRAM_CHAT_ID")
        .expect("TELEGRAM_CHAT_ID not set")
        .parse()
        .expect("TELEGRAM_CHAT_ID not a valid i64");

    let project_id = "btn-test";
    let chat_id = ChatId(chat_id_raw);
    let bot = Bot::new(&token);
    let http = reqwest::Client::new();

    // Flush stale updates
    let mut offset = flush_updates(&token).await.unwrap_or(0);

    // Send completion message with Stats + Kill buttons
    let completion_msg = format!(
        "✅ <b>Completed #3</b>  ·  <code>{pid}</code>\n\
         {div}\n\
         Implemented orderbook matching engine with price-time priority\n\
         {div}\n\
         ⚡ <code>67.3 tok/s</code>  ·  📊 <code>42.1%</code> context\n\
         💰 <code>$0.47</code>  ·  ⏱ <code>38s</code>",
        pid = escape_html(project_id),
        div = DIV,
    );

    let keyboard = InlineKeyboardMarkup::new(vec![vec![
        InlineKeyboardButton::callback("📊 Stats", "query"),
        InlineKeyboardButton::callback("🛑 Kill", "kill"),
    ]]);

    eprintln!("=== Sending completion with buttons ===");
    bot.send_message(chat_id, &completion_msg)
        .parse_mode(ParseMode::Html)
        .reply_markup(keyboard)
        .await
        .expect("failed to send completion");
    eprintln!("=== Press the Stats or Kill button (1 min timeout) ===");

    // Poll for callback queries
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    let api_url = format!("https://api.telegram.org/bot{token}/getUpdates");
    let answer_url = format!("https://api.telegram.org/bot{token}/answerCallbackQuery");

    loop {
        if tokio::time::Instant::now() >= deadline {
            panic!("Timed out waiting for button press");
        }

        let body = serde_json::json!({
            "offset": offset,
            "timeout": 1,
            "allowed_updates": ["callback_query"],
        });

        let resp = http
            .post(&api_url)
            .json(&body)
            .timeout(Duration::from_secs(10))
            .send()
            .await;

        let json: serde_json::Value = match resp {
            Ok(r) if r.status().is_success() => r.json().await.unwrap_or_default(),
            _ => {
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

            let cq = &update["callback_query"];
            if cq.is_null() {
                continue;
            }

            let callback_id = cq["id"].as_str().unwrap_or("");
            let data = cq["data"].as_str().unwrap_or("");
            let cq_chat_id = cq["message"]["chat"]["id"].as_i64().unwrap_or(0);

            if cq_chat_id != chat_id_raw {
                continue;
            }

            eprintln!("=== Button pressed: {data} ===");

            match data {
                "query" => {
                    // Answer the callback with a toast
                    let _ = http.post(&answer_url)
                        .json(&serde_json::json!({
                            "callback_query_id": callback_id,
                            "text": "Loading stats...",
                        }))
                        .send()
                        .await;

                    // Send the stats message
                    let stats_msg = format!(
                        "📊 <b>Stats</b>  ·  <code>{pid}</code>\n\
                         {div}\n\
                         🔄 Invocation <code>#4</code>\n\
                         💰 Cost so far: <code>$1.23</code>\n\
                         ⏱ Uptime: <code>12m</code>\n\
                         🤖 Model: <code>claude-sonnet-4-5</code>",
                        pid = escape_html(project_id),
                        div = DIV,
                    );
                    bot.send_message(chat_id, &stats_msg)
                        .parse_mode(ParseMode::Html)
                        .await
                        .expect("failed to send stats");

                    eprintln!("\n=== TEST PASSED: Stats button works ===");
                    return;
                }
                "kill" => {
                    let _ = http.post(&answer_url)
                        .json(&serde_json::json!({
                            "callback_query_id": callback_id,
                            "text": "Kill command received!",
                            "show_alert": true,
                        }))
                        .send()
                        .await;

                    bot.send_message(chat_id, &format!(
                        "🛑 <b>Killed by button</b>  ·  <code>{pid}</code>",
                        pid = escape_html(project_id),
                    ))
                        .parse_mode(ParseMode::Html)
                        .await
                        .expect("failed to send kill ack");

                    eprintln!("\n=== TEST PASSED: Kill button works ===");
                    return;
                }
                _ => {
                    let _ = http.post(&answer_url)
                        .json(&serde_json::json!({
                            "callback_query_id": callback_id,
                        }))
                        .send()
                        .await;
                }
            }
        }
    }
}
