//! Integration test: sends every autorun Telegram message type using the real
//! TelegramClient code to verify formatting, buttons, and delivery.
//!
//! Run with: cargo test --test test_telegram_messages -- --nocapture --include-ignored

use std::time::Duration;

use rex_cli::autorun::telegram::TelegramClient;
use rex_cli::autorun::types::{escape_html, DIV};

#[tokio::test]
#[ignore]
async fn test_all_autorun_message_types() {
    dotenvy::dotenv().ok();

    let token = std::env::var("REX_AUTORUN_TELEGRAM_BOT_TOKEN")
        .expect("REX_AUTORUN_TELEGRAM_BOT_TOKEN not set");
    let chat_id: i64 = std::env::var("REX_TELEGRAM_CHAT_ID")
        .expect("REX_TELEGRAM_CHAT_ID not set")
        .parse()
        .expect("REX_TELEGRAM_CHAT_ID not a valid i64");

    let project_id = "orderbook-engine";
    let tg = TelegramClient::new(
        token,
        chat_id,
        None,
        std::path::PathBuf::from("."),
        project_id.to_string(),
    );
    let delay = Duration::from_millis(1500);

    // 1. STARTUP — notify_with_buttons (Reply + Stats + Kill)
    eprintln!("--- Sending: startup ---");
    tg.notify_with_buttons(
        &format!(
            "🚀 <b>Autorun started</b>  ·  <code>{pid}</code>\n\
             {DIV}\n\
             📂 <b>Project:</b> Orderbook Engine\n\
             📁 <b>Directory:</b> <code>/srv/projects/orderbook</code>\n\
             {DIV}\n\
             <b>Commands:</b>\n\
             <code>/kill {pid}</code> — stop autorun\n\
             <code>/query {pid}</code> — show live stats",
            pid = escape_html(project_id),
        ),
        project_id,
    )
    .await;
    tokio::time::sleep(delay).await;

    // 2. COMPLETION — notify_with_status_buttons (Stats + Kill, no Reply)
    eprintln!("--- Sending: completion ---");
    tg.notify_with_status_buttons(
        &format!(
            "✅ <b>Completed #1</b>  ·  <code>{pid}</code>  ·  <code>onboarding:goal</code>\n\
             {DIV}\n\
             Defined project goal: high-performance orderbook matching engine.\n\
             {DIV}\n\
             ⚡ <code>71.2 tok/s</code>  ·  📊 <code>42.1%</code> context\n\
             💰 <code>$0.47</code>  ·  ⏱ <code>2m 15s</code>",
            pid = escape_html(project_id),
        ),
        project_id,
    )
    .await;
    tokio::time::sleep(delay).await;

    // 3. NEEDS INPUT — send_with_buttons (Reply + Stats + Kill)
    eprintln!("--- Sending: needs input ---");
    let _question_msg_id = tg
        .send_with_buttons(
            &format!(
                "💬 <b>Input needed</b>  ·  <code>{pid}</code>  ·  <code>onboarding:scope</code>\n\
                 {DIV}\n\
                 The project scope needs definition. Should the matching engine support:\n\n\
                 1. Limit orders only (simpler, faster to build)\n\
                 2. Limit + market orders (standard exchange feature set)\n\
                 3. Full order types including stop-loss, iceberg, and FOK/IOC\n\n\
                 Also: should the WebSocket feed be public or authenticated?\n\
                 {DIV}\n\
                 ⚡ <code>68.5 tok/s</code>  ·  📊 <code>31.7%</code> context\n\n\
                 <i>Reply to this message with your answer</i>",
                pid = escape_html(project_id),
            ),
            project_id,
        )
        .await
        .expect("failed to send question");
    tokio::time::sleep(delay).await;

    // 4. ACK — plain notify (no buttons)
    eprintln!("--- Sending: ack ---");
    tg.notify(&format!(
        "👍 <b>Received</b>  ·  <code>{pid}</code>\nOn it, resuming the session now.",
        pid = escape_html(project_id),
    ))
    .await;
    tokio::time::sleep(delay).await;

    // 5. ERROR — plain notify (no buttons)
    eprintln!("--- Sending: error ---");
    tg.notify(&format!(
        "⚠️ <b>Error</b>  ·  <code>{pid}</code>  ·  <code>design:architecture</code>\n\
         {DIV}\n\
         ⚡ <code>55.0 tok/s</code>  ·  📊 <code>78.3%</code> context\n\
         {DIV}\n\
         <blockquote>Operator returned error: failed to parse design document — missing required field 'modules'</blockquote>",
        pid = escape_html(project_id),
    ))
    .await;
    tokio::time::sleep(delay).await;

    // 6. PROJECT DONE — plain notify (no buttons)
    eprintln!("--- Sending: project done ---");
    tg.notify(&format!(
        "🏁 <b>Project complete!</b>  ·  <code>{pid}</code>\n\
         {DIV}\n\
         ⚡ <code>65.8 tok/s</code>  ·  📊 <code>51.2%</code> context\n\
         📊 <code>12</code> invocations  ·  💰 <code>$8.34</code>  ·  ⏱ <code>1h 22m</code>",
        pid = escape_html(project_id),
    ))
    .await;
    tokio::time::sleep(delay).await;

    // 7. KILLED — plain notify (no buttons)
    eprintln!("--- Sending: killed ---");
    tg.notify(&format!(
        "🛑 <b>Killed</b>  ·  <code>{pid}</code>\n{DIV}\nStopped by /kill command",
        pid = escape_html(project_id),
    ))
    .await;
    tokio::time::sleep(delay).await;

    // 8. BUDGET LIMIT — plain notify (no buttons)
    eprintln!("--- Sending: budget limit ---");
    tg.notify(&format!(
        "💸 <b>Budget limit reached</b>  ·  <code>{pid}</code>\n\
         {DIV}\n\
         💰 <code>$500.00</code> / <code>$500.00</code> — stopping",
        pid = escape_html(project_id),
    ))
    .await;

    eprintln!("\n=== ALL MESSAGE TYPES SENT ===");
}
