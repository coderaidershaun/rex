//! Integration test: sends every rex-chat Telegram message type using the real
//! ChatTelegramClient code to verify formatting, buttons, and delivery.
//!
//! Run with: cargo test --test test_chat_messages -- --nocapture --include-ignored

use std::time::Duration;

use rex_cli::autorun::types::{escape_html, DIV};
use rex_cli::chat::telegram::{ChatTelegramClient, InlineButton};

#[tokio::test]
#[ignore]
async fn test_all_chat_message_types() {
    dotenvy::dotenv().ok();

    let token = std::env::var("REX_AUTOCHAT_TELEGRAM_BOT_TOKEN")
        .expect("REX_AUTOCHAT_TELEGRAM_BOT_TOKEN not set");
    let chat_id: i64 = std::env::var("REX_TELEGRAM_CHAT_ID")
        .expect("REX_TELEGRAM_CHAT_ID not set")
        .parse()
        .expect("REX_TELEGRAM_CHAT_ID not a valid i64");

    let project_id = "orderbook-engine";
    let tg = ChatTelegramClient::new(token, chat_id, 0);
    let delay = Duration::from_millis(1500);

    // 1. STARTUP — plain notify
    eprintln!("--- Sending: startup ---");
    tg.notify("🏠 <b>Rex Chat online</b>\n\nSend /menu to see your projects.")
        .await;
    tokio::time::sleep(delay).await;

    // 2. PROJECT MENU (running + available) — send_with_buttons
    eprintln!("--- Sending: project menu ---");
    let menu_msg = format!(
        "🏠 <b>Rex Chat</b>\n\
         {DIV}\n\n\
         🟢 <b>RUNNING</b>\n\
         \u{00a0}\u{00a0}<code>orderbook-engine</code> · ⏱ 1h 22m · 💰 $8.34 · 🔄 #12\n\n\
         {DIV}\n\n\
         📁 <b>AVAILABLE</b>\n\
         \u{00a0}\u{00a0}<code>portfolio-tracker</code> · /srv/projects/portfolio\n",
    );
    let menu_buttons = vec![
        // Running: Chat + Status + Stop on one row
        vec![
            InlineButton {
                text: "💬 orderbook-engine".to_string(),
                callback_data: "chat:orderbook-engine".to_string(),
            },
            InlineButton {
                text: "📊".to_string(),
                callback_data: "status:orderbook-engine".to_string(),
            },
            InlineButton {
                text: "🛑".to_string(),
                callback_data: "stop:orderbook-engine".to_string(),
            },
        ],
        // Available: Start + Chat on one row
        vec![
            InlineButton {
                text: "🚀 portfolio-tracker".to_string(),
                callback_data: "start:portfolio-tracker".to_string(),
            },
            InlineButton {
                text: "💬 Chat".to_string(),
                callback_data: "chat:portfolio-tracker".to_string(),
            },
        ],
    ];
    tg.send_with_buttons(&menu_msg, &menu_buttons)
        .await
        .expect("failed to send project menu");
    tokio::time::sleep(delay).await;

    // 3. CHAT PROMPT — send_force_reply
    eprintln!("--- Sending: chat prompt (ForceReply) ---");
    let prompt_text = format!(
        "💬 <b>Chat</b>  ·  <code>{pid}</code>\n<i>Type your message below</i>",
        pid = escape_html(project_id),
    );
    let prompt_msg_id = tg
        .send_force_reply(&prompt_text)
        .await
        .expect("failed to send chat prompt");
    eprintln!("    ForceReply message_id: {prompt_msg_id}");
    tokio::time::sleep(delay).await;

    // 4. THINKING INDICATOR — plain send_message
    eprintln!("--- Sending: thinking ---");
    let thinking_text = format!(
        "🔍 <b>Rex Chat</b>  ·  <code>{pid}</code>\n{DIV}\nThinking...",
        pid = escape_html(project_id),
    );
    let thinking_msg_id = tg
        .send_message(&thinking_text)
        .await
        .expect("failed to send thinking");
    tokio::time::sleep(delay).await;

    // 5. CHAT RESPONSE (edit thinking → response) — edit_message_with_buttons
    eprintln!("--- Sending: chat response (edit thinking message) ---");
    let response_text = "The orderbook matching engine uses a price-time priority \
        algorithm. Buy orders are sorted by price (highest first), then by arrival \
        time. Sell orders are sorted by price (lowest first), then by arrival time.\n\n\
        A match occurs when the best buy price >= best sell price. The trade executes \
        at the resting order's price.\n\n\
        The current implementation supports limit orders only. Market orders can be \
        added by treating them as limit orders with an extreme price (MAX for buys, \
        0 for sells).";
    let formatted = format_chat_response(project_id, response_text);
    let response_buttons = vec![vec![
        InlineButton {
            text: "💬 Reply".to_string(),
            callback_data: format!("rc_reply:{project_id}"),
        },
        InlineButton {
            text: "🏠 Menu".to_string(),
            callback_data: "menu".to_string(),
        },
    ]];
    tg.edit_message_with_buttons(thinking_msg_id, &formatted, &response_buttons)
        .await
        .expect("failed to edit thinking → response");
    tokio::time::sleep(delay).await;

    // 6. CHAT ERROR (edit thinking → error) — edit_message
    eprintln!("--- Sending: thinking (will become error) ---");
    let thinking2_id = tg
        .send_message(&format!(
            "🔍 <b>Rex Chat</b>  ·  <code>{pid}</code>\n{DIV}\nThinking...",
            pid = escape_html(project_id),
        ))
        .await
        .expect("failed to send thinking 2");
    tokio::time::sleep(delay).await;

    eprintln!("--- Sending: chat error (edit thinking message) ---");
    let error_text = format!(
        "❌ <b>Chat error</b>  ·  <code>{pid}</code>\n{DIV}\n<blockquote>chat claude timed out (10 min)</blockquote>",
        pid = escape_html(project_id),
    );
    tg.edit_message(thinking2_id, &error_text)
        .await
        .expect("failed to edit thinking → error");
    tokio::time::sleep(delay).await;

    // 7. AUTORUN STATUS — plain notify
    eprintln!("--- Sending: autorun status ---");
    let status_msg = format!(
        "📊 <b>Status</b>  ·  <code>{pid}</code>\n\
         {DIV}\n\
         ⏱ <b>Uptime:</b> <code>1h 22m</code>\n\
         🔄 <b>Phase:</b> <code>Running</code>\n\
         📊 <b>Invocations:</b> <code>12</code>\n\
         ✅ <b>Items completed:</b> <code>8</code>\n\
         💰 <b>Cost:</b> <code>$8.34</code>",
        pid = escape_html(project_id),
    );
    tg.notify(&status_msg).await;
    tokio::time::sleep(delay).await;

    // 8. AUTORUN STARTED — plain notify
    eprintln!("--- Sending: autorun started ---");
    tg.notify(&format!(
        "🚀 Started autorun for <code>{}</code>",
        escape_html(project_id)
    ))
    .await;
    tokio::time::sleep(delay).await;

    // 9. STOP SENT — plain notify
    eprintln!("--- Sending: stop sent ---");
    tg.notify(&format!(
        "🛑 Sent SIGTERM to <code>{}</code> (pgid 12345)",
        escape_html(project_id)
    ))
    .await;
    tokio::time::sleep(delay).await;

    // 10. NO PROJECTS — plain notify
    eprintln!("--- Sending: no projects ---");
    tg.notify("🏠 <b>Rex Chat</b>\n\nNo projects found. Run <code>rex init</code> to create one.")
        .await;
    tokio::time::sleep(delay).await;

    // 11. SHUTDOWN — plain notify
    eprintln!("--- Sending: shutdown ---");
    tg.notify("🏠 <b>Rex Chat offline</b>").await;

    eprintln!("\n=== ALL CHAT MESSAGE TYPES SENT ===");
}

/// Replicates the real `format_chat_response` from daemon.rs.
fn format_chat_response(project_id: &str, response: &str) -> String {
    let max_chars = 3000;
    let chars: Vec<char> = response.chars().collect();
    let (content, truncated) = if chars.len() > max_chars {
        (chars[..max_chars].iter().collect::<String>(), true)
    } else {
        (response.to_string(), false)
    };
    let suffix = if truncated {
        "\n…\n\n<i>(truncated)</i>"
    } else {
        ""
    };

    format!(
        "🗨️ <b>Rex Chat</b>  ·  <code>{pid}</code>\n\
         {DIV}\n\
         {resp}{suffix}\n\
         {DIV}\n\
         <i>💬 Reply to continue  ·  🏠 Menu</i>",
        pid = escape_html(project_id),
        resp = escape_html(&content),
    )
}
