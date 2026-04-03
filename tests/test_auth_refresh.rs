//! Integration test: sends separator style comparisons + full message flow
//! via teloxide, then waits for a reply.
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
    let bot = Bot::new(&token);
    let flushed_offset = flush_updates(&token).await;
    let mut tg = TelegramClient::new(token, chat_id_raw, flushed_offset);
    let delay = Duration::from_millis(800);

    // ── Separator comparison ─────────────────────────────────────────────
    // Send the same message with different separator styles

    let separators: &[(&str, &str)] = &[
        ("A: No separator (just spacing)", ""),
        ("B: Thin line ─", "\n─────────────\n"),
        ("C: Dotted ·", "\n· · · · · · · · · · · ·\n"),
        ("D: Em dash —", "\n———————————\n"),
        ("E: Blockquote body", "BLOCKQUOTE"),
    ];

    for (label, sep) in separators {
        let msg = if *sep == "BLOCKQUOTE" {
            format!(
                "✅ <b>Completed #3</b>  ·  <code>{pid}</code>\n\n\
                 <blockquote>Implemented orderbook matching engine with price-time priority\n\n\
                 ⚡ 67.3 tok/s  ·  📊 42.1% context\n\
                 💰 $0.47  ·  ⏱ 38s</blockquote>\n\n\
                 <i>{label}</i>",
                pid = escape_html(project_id),
                label = label,
            )
        } else if sep.is_empty() {
            format!(
                "✅ <b>Completed #3</b>  ·  <code>{pid}</code>\n\n\
                 Implemented orderbook matching engine with price-time priority\n\n\
                 ⚡ <code>67.3 tok/s</code>  ·  📊 <code>42.1%</code> context\n\
                 💰 <code>$0.47</code>  ·  ⏱ <code>38s</code>\n\n\
                 <i>{label}</i>",
                pid = escape_html(project_id),
                label = label,
            )
        } else {
            format!(
                "✅ <b>Completed #3</b>  ·  <code>{pid}</code>{sep}\
                 Implemented orderbook matching engine with price-time priority\n\n\
                 ⚡ <code>67.3 tok/s</code>  ·  📊 <code>42.1%</code> context\n\
                 💰 <code>$0.47</code>  ·  ⏱ <code>38s</code>\n\n\
                 <i>{label}</i>",
                pid = escape_html(project_id),
                sep = sep,
                label = label,
            )
        };

        bot.send_message(chat_id, &msg)
            .parse_mode(ParseMode::Html)
            .await
            .expect("failed to send separator sample");
        tokio::time::sleep(delay).await;
    }

    // ── Now send a full needs_input question so we can verify reply works ─
    let question = "Which separator style do you prefer? Reply A, B, C, D, or E.";
    let question_msg = format!(
        "💬 <b>Input needed</b>  ·  <code>{pid}</code>\n\n\
         <blockquote>{q}</blockquote>\n\
         <i>Reply to this message</i>",
        pid = escape_html(project_id),
        q = escape_html(question),
    );

    let sent = bot
        .send_message(chat_id, &question_msg)
        .parse_mode(ParseMode::Html)
        .reply_markup(ReplyMarkup::ForceReply(
            teloxide::types::ForceReply::new(),
        ))
        .await
        .expect("failed to send question");
    let msg_id = sent.id.0 as i64;

    let query_response = "Waiting for your separator preference...";
    let result = tg
        .wait_for_reply(msg_id, project_id, Duration::from_secs(60), query_response)
        .await;

    match result {
        Ok(rex_cli::autorun::telegram::TelegramPollResult::Reply(reply)) => {
            eprintln!("=== Got reply: {reply} ===");
            bot.send_message(chat_id, &format!("👍 <b>Noted: {}</b>", escape_html(&reply)))
                .parse_mode(ParseMode::Html)
                .await
                .expect("failed to send ack");
            eprintln!("\n=== TEST PASSED ===");
        }
        Ok(rex_cli::autorun::telegram::TelegramPollResult::Kill) => {
            panic!("Received /kill");
        }
        Err(e) => {
            panic!("wait_for_reply failed: {e}");
        }
    }
}
