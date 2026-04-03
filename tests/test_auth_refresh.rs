//! Integration test: exercises the full auth refresh flow using the real
//! TelegramClient — exactly as autorun does in production.
//!
//! Run with: cargo test --test test_auth_refresh -- --nocapture --include-ignored

use std::time::Duration;
use rex_cli::autorun::telegram::TelegramClient;
use tokio::io::AsyncReadExt;

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn extract_url(text: &str) -> Option<String> {
    let start = text.find("https://")?;
    let rest = &text[start..];
    let end = rest
        .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == '>' || c == '<')
        .unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

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
                eprintln!("[stdout]: {chunk}");
                text.push_str(&chunk);
                if let Some(url) = extract_url(&text) {
                    return Some(url);
                }
            }
            Ok(Err(e)) => {
                eprintln!("[stdout error]: {e}");
                break;
            }
            Err(_) => continue,
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
                eprintln!("[stderr]: {etext}");
                return extract_url(&etext);
            }
        }
    }

    None
}

#[tokio::test]
#[ignore] // requires live Telegram credentials + user interaction
async fn test_auth_refresh_via_telegram() {
    dotenvy::dotenv().ok();

    let token = std::env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set");
    let chat_id: i64 = std::env::var("TELEGRAM_CHAT_ID")
        .expect("TELEGRAM_CHAT_ID not set")
        .parse()
        .expect("TELEGRAM_CHAT_ID not a valid i64");

    let project_id = "auth-test";

    // Use the real TelegramClient — exactly as production autorun does.
    // Pass None for initial offset so it starts fresh (offset 0), then
    // flush stale updates before we send anything.
    let mut tg = TelegramClient::new(token, chat_id, None);

    // Flush any stale updates so our offset is current
    tg.notify("<b>[auth-test] Flushing stale updates...</b>").await;
    // Small sleep to let the flush message propagate
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Step 1: Spawn `claude auth login`
    eprintln!("\n=== Spawning `claude auth login` ===");
    let mut child = tokio::process::Command::new("claude")
        .arg("auth")
        .arg("login")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn `claude auth login`");

    // Step 2: Extract the auth URL
    eprintln!("=== Reading auth URL from output ===");
    let auth_url = read_auth_url(&mut child).await;

    // Step 3: Build message — same format as production attempt_auth_refresh
    let msg = match &auth_url {
        Some(url) => {
            eprintln!("=== Got auth URL: {url} ===");
            format!(
                "<b>[{pid}] Auth expired</b>\n━━━━━━━━━━━━━━━━━━━━\n\
                 Your Claude token has expired.\n\n\
                 Please visit this URL to re-authorize:\n{url}\n\n\
                 <i>Reply when authorization is complete</i>",
                pid = escape_html(project_id),
            )
        }
        None => {
            eprintln!("=== No URL found, sending fallback ===");
            format!(
                "<b>[{pid}] Auth expired</b>\n━━━━━━━━━━━━━━━━━━━━\n\
                 Your Claude token has expired.\n\n\
                 Please run <code>claude auth login</code> on the server, then reply here when done.\n\n\
                 <i>Reply when authorization is complete</i>",
                pid = escape_html(project_id),
            )
        }
    };

    // Step 4: Send via TelegramClient::send_question (ForceReply) — same as production
    eprintln!("=== Sending to Telegram ===");
    let msg_id = tg.send_question(&msg).await.expect("failed to send Telegram question");
    eprintln!("=== Sent message_id: {msg_id} ===");

    // Step 5: Wait for reply — using the real TelegramClient::wait_for_reply
    // 1 minute timeout for this test
    eprintln!("=== Waiting for reply (1 min timeout) ===");
    let query_response = format!(
        "<b>[{pid}]</b>\nWaiting for auth refresh — no stats available yet.",
        pid = escape_html(project_id),
    );
    let result = tg
        .wait_for_reply(msg_id, project_id, Duration::from_secs(60), &query_response)
        .await;

    // Clean up auth process
    let _ = child.kill().await;
    let _ = child.wait().await;

    // Step 6: Assert we got a reply
    match result {
        Ok(rex_cli::autorun::telegram::TelegramPollResult::Reply(reply)) => {
            eprintln!("=== Got reply: {reply} ===");
            tg.notify(&format!(
                "<b>[{pid}] Auth test complete</b>\n✓ Reply received: <i>{r}</i>",
                pid = escape_html(project_id),
                r = escape_html(&reply),
            )).await;
            eprintln!("\n=== TEST PASSED ===");
        }
        Ok(rex_cli::autorun::telegram::TelegramPollResult::Kill) => {
            panic!("Received /kill command — test aborted");
        }
        Err(e) => {
            panic!("wait_for_reply failed: {e}");
        }
    }
}
