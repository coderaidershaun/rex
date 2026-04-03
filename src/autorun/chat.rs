//! Interactive chat sessions — parallel Q&A about the running project via Telegram.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tokio::sync::{mpsc, Mutex};
use tracing::{error, info};

use super::telegram::TelegramClient;
use super::types::{escape_html, ClaudeOutput, DIV};
use crate::errors::{RexError, RexResult};

// ── Types ──────────────────────────────────────────────────────────────────

/// Events routed from the main poller to a chat session.
pub(crate) enum ChatEvent {
    /// User sent a reply.
    Reply(String),
    /// User clicked the Restart button.
    Restart,
}

pub(crate) struct ActiveSession {
    pub(crate) event_tx: mpsc::Sender<ChatEvent>,
}

pub(crate) struct Inner {
    pub(crate) telegram_token: String,
    pub(crate) telegram_chat_id: i64,
    pub(crate) project_id: String,
    pub(crate) project_dir: PathBuf,
    pub(crate) sessions: HashMap<String, ActiveSession>,
    /// Telegram message_id → chat_id for reply routing.
    pub(crate) message_to_chat: HashMap<i64, String>,
}

/// Thread-safe handle shared between the main poller and chat tasks.
pub(crate) type ChatManager = Arc<Mutex<Inner>>;

// ── Constructor ────────────────────────────────────────────────────────────

pub(crate) fn new_manager(
    telegram_token: String,
    telegram_chat_id: i64,
    project_id: String,
    project_dir: PathBuf,
) -> ChatManager {
    Arc::new(Mutex::new(Inner {
        telegram_token,
        telegram_chat_id,
        project_id,
        project_dir,
        sessions: HashMap::new(),
        message_to_chat: HashMap::new(),
    }))
}

// ── Routing API (called by the main poller) ────────────────────────────────

/// Start a new chat if `target_project_id` matches. Returns `true` if started.
pub(crate) async fn try_start_chat(
    manager: &ChatManager,
    target_project_id: &str,
    query: &str,
) -> bool {
    let (token, tg_chat_id, project_id, project_dir) = {
        let inner = manager.lock().await;
        if target_project_id != inner.project_id {
            return false;
        }
        (
            inner.telegram_token.clone(),
            inner.telegram_chat_id,
            inner.project_id.clone(),
            inner.project_dir.clone(),
        )
    };

    let chat_id = generate_chat_id();
    let (tx, rx) = mpsc::channel::<ChatEvent>(16);

    {
        let mut inner = manager.lock().await;
        inner
            .sessions
            .insert(chat_id.clone(), ActiveSession { event_tx: tx });
    }

    let mgr = Arc::clone(manager);
    let cid = chat_id.clone();
    let q = query.to_string();
    info!(chat_id = %cid, "starting new chat session");

    tokio::spawn(async move {
        run_session(mgr, cid, project_id, project_dir, token, tg_chat_id, q, rx).await;
    });

    true
}

/// Register a Telegram message as belonging to a chat (for reply routing).
pub(crate) async fn register_message(manager: &ChatManager, message_id: i64, chat_id: &str) {
    manager
        .lock()
        .await
        .message_to_chat
        .insert(message_id, chat_id.to_string());
}

/// Look up the chat_id for a Telegram message_id.
pub(crate) async fn lookup_chat(manager: &ChatManager, message_id: i64) -> Option<String> {
    manager
        .lock()
        .await
        .message_to_chat
        .get(&message_id)
        .cloned()
}

/// Forward a user reply to the chat session.
pub(crate) async fn route_reply(manager: &ChatManager, chat_id: &str, text: String) -> bool {
    let inner = manager.lock().await;
    match inner.sessions.get(chat_id) {
        Some(s) => s.event_tx.send(ChatEvent::Reply(text)).await.is_ok(),
        None => false,
    }
}

/// Forward a restart event to the chat session.
pub(crate) async fn route_restart(manager: &ChatManager, chat_id: &str) -> bool {
    let inner = manager.lock().await;
    match inner.sessions.get(chat_id) {
        Some(s) => s.event_tx.send(ChatEvent::Restart).await.is_ok(),
        None => false,
    }
}

/// Remove a session and all its message mappings.
async fn remove_session(manager: &ChatManager, chat_id: &str) {
    let mut inner = manager.lock().await;
    inner.sessions.remove(chat_id);
    inner.message_to_chat.retain(|_, v| v != chat_id);
}

// ── Chat Session Task ──────────────────────────────────────────────────────

const CHAT_SYSTEM_PROMPT: &str = r#"You are a project investigation assistant answering questions via Telegram.

RULES:
- Investigate the project files to answer the user's question accurately.
- Be concise. Your response will be sent via Telegram (4096 char limit).
- Keep responses under 3000 characters.
- Do NOT modify any files. Read-only investigation only.
- Do NOT output any JSON status objects.
- Use plain text. Do not use HTML tags or markdown formatting.
- Focus on being helpful and direct."#;

#[allow(clippy::too_many_arguments)]
async fn run_session(
    manager: ChatManager,
    chat_id: String,
    project_id: String,
    project_dir: PathBuf,
    telegram_token: String,
    telegram_chat_id: i64,
    initial_query: String,
    mut rx: mpsc::Receiver<ChatEvent>,
) {
    let tg = TelegramClient::new(telegram_token, telegram_chat_id, None);

    // Send thinking indicator
    let thinking = format!(
        "🔍 <b>Chat</b>  ·  <code>{pid}</code>  ·  <code>{cid}</code>\n{DIV}\nInvestigating...",
        pid = escape_html(&project_id),
        cid = chat_id,
    );
    let msg_id = match tg.send_message(&thinking).await {
        Ok(id) => id,
        Err(e) => {
            error!(chat_id = %chat_id, "thinking message failed: {e}");
            remove_session(&manager, &chat_id).await;
            return;
        }
    };

    // First Claude invocation
    let session_name = format!("rex-chat-{project_id}-{chat_id}");
    let prompt = format!(
        "Project \"{pid}\" at {dir}.\n\nUser's question: {q}",
        pid = project_id,
        dir = project_dir.display(),
        q = initial_query,
    );

    let (response, mut claude_session_id) =
        match spawn_and_await(&project_dir, &prompt, None, &session_name).await {
            Ok(pair) => pair,
            Err(e) => {
                let _ = tg
                    .edit_message(
                        msg_id,
                        &format_error(&project_id, &chat_id, &e.to_string()),
                    )
                    .await;
                remove_session(&manager, &chat_id).await;
                return;
            }
        };

    // Replace thinking with response + buttons
    let formatted = format_response(&project_id, &chat_id, &response);
    if tg
        .edit_message_with_chat_buttons(msg_id, &formatted, &chat_id)
        .await
        .is_err()
    {
        match tg.send_with_chat_buttons(&formatted, &chat_id).await {
            Ok(id) => {
                register_message(&manager, id, &chat_id).await;
            }
            Err(e) => {
                error!(chat_id = %chat_id, "response send failed: {e}");
                remove_session(&manager, &chat_id).await;
                return;
            }
        }
    } else {
        register_message(&manager, msg_id, &chat_id).await;
    }

    // Multi-turn loop
    let idle_timeout = Duration::from_secs(30 * 60);
    loop {
        match tokio::time::timeout(idle_timeout, rx.recv()).await {
            Ok(Some(ChatEvent::Reply(text))) => {
                info!(chat_id = %chat_id, len = text.len(), "chat reply");

                let t = format!(
                    "🔍 <b>Chat</b>  ·  <code>{pid}</code>  ·  <code>{cid}</code>\n\
                     {DIV}\n\
                     Investigating...",
                    pid = escape_html(&project_id),
                    cid = chat_id,
                );
                let t_id = match tg.send_message(&t).await {
                    Ok(id) => id,
                    Err(e) => {
                        error!(chat_id = %chat_id, "thinking failed: {e}");
                        break;
                    }
                };

                match spawn_and_await(
                    &project_dir,
                    &text,
                    Some(&claude_session_id),
                    &session_name,
                )
                .await
                {
                    Ok((resp, sid)) => {
                        claude_session_id = sid;
                        let fmt = format_response(&project_id, &chat_id, &resp);
                        if tg
                            .edit_message_with_chat_buttons(t_id, &fmt, &chat_id)
                            .await
                            .is_err()
                        {
                            match tg.send_with_chat_buttons(&fmt, &chat_id).await {
                                Ok(id) => {
                                    register_message(&manager, id, &chat_id).await;
                                }
                                Err(e) => {
                                    error!(chat_id = %chat_id, "send failed: {e}");
                                    break;
                                }
                            }
                        } else {
                            register_message(&manager, t_id, &chat_id).await;
                        }
                    }
                    Err(e) => {
                        let _ = tg
                            .edit_message(
                                t_id,
                                &format_error(&project_id, &chat_id, &e.to_string()),
                            )
                            .await;
                        break;
                    }
                }
            }
            Ok(Some(ChatEvent::Restart)) => {
                info!(chat_id = %chat_id, "chat restarted by user");
                let _ = tg
                    .send_message(&format!(
                        "🔄 <b>Chat ended</b>  ·  <code>{pid}</code>  ·  <code>{cid}</code>\n\
                         {DIV}\n\
                         Session restarted. Use <code>/chat</code> to begin a new one.",
                        pid = escape_html(&project_id),
                        cid = chat_id,
                    ))
                    .await;
                break;
            }
            Ok(None) => break,
            Err(_) => {
                info!(chat_id = %chat_id, "chat timed out (30 min inactivity)");
                let _ = tg
                    .send_message(&format!(
                        "⏰ <b>Chat timed out</b>  ·  <code>{pid}</code>  ·  <code>{cid}</code>\n\
                         {DIV}\n\
                         30 min inactivity. Use <code>/chat</code> to start a new one.",
                        pid = escape_html(&project_id),
                        cid = chat_id,
                    ))
                    .await;
                break;
            }
        }
    }

    remove_session(&manager, &chat_id).await;
}

// ── Claude Spawning ────────────────────────────────────────────────────────

async fn spawn_and_await(
    project_dir: &Path,
    prompt: &str,
    session_id: Option<&str>,
    session_name: &str,
) -> RexResult<(String, String)> {
    let mut cmd = tokio::process::Command::new("claude");
    cmd.current_dir(project_dir);

    if let Some(sid) = session_id {
        cmd.arg("--resume").arg(sid);
    }

    cmd.arg("-p")
        .arg(prompt)
        .arg("--output-format")
        .arg("json")
        .arg("--dangerously-skip-permissions")
        .arg("--max-turns")
        .arg("30")
        .arg("--max-budget-usd")
        .arg("5.00")
        .arg("--append-system-prompt")
        .arg(CHAT_SYSTEM_PROMPT);

    if session_id.is_none() {
        cmd.arg("--name").arg(session_name);
    }

    cmd.env("REX_AUTORUN", "1");

    #[cfg(unix)]
    unsafe {
        cmd.pre_exec(|| {
            libc::setpgid(0, 0);
            Ok(())
        });
    }

    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);

    info!(session_name, "spawning chat claude");
    let child = cmd.spawn().map_err(|e| {
        RexError::ClaudeProcess(format!("failed to spawn chat claude: {e}"))
    })?;

    await_chat_claude(child).await
}

async fn await_chat_claude(
    mut child: tokio::process::Child,
) -> RexResult<(String, String)> {
    let timeout = Duration::from_secs(300);
    let result = tokio::time::timeout(timeout, async {
        use tokio::io::AsyncReadExt;

        let mut stdout_handle = child.stdout.take();
        let mut stderr_handle = child.stderr.take();

        let (stdout_buf, _) = tokio::join!(
            async {
                let mut buf = Vec::new();
                if let Some(ref mut s) = stdout_handle {
                    s.read_to_end(&mut buf).await.ok();
                }
                buf
            },
            async {
                let mut buf = Vec::new();
                if let Some(ref mut s) = stderr_handle {
                    s.read_to_end(&mut buf).await.ok();
                }
                buf
            },
        );

        let _status = child.wait().await?;
        Ok::<_, std::io::Error>(stdout_buf)
    })
    .await;

    match result {
        Ok(Ok(stdout_buf)) => {
            let stdout_str = String::from_utf8_lossy(&stdout_buf);
            let output: ClaudeOutput =
                serde_json::from_str(&stdout_str).map_err(|e| RexError::JsonParse {
                    context: format!(
                        "chat output: {}",
                        stdout_str.chars().take(200).collect::<String>()
                    ),
                    source: e,
                })?;
            Ok((output.result, output.session_id))
        }
        Ok(Err(e)) => Err(RexError::ClaudeProcess(format!(
            "chat process error: {e}"
        ))),
        Err(_) => Err(RexError::ClaudeProcess(
            "chat claude timed out (5 min)".into(),
        )),
    }
}

// ── Formatting ─────────────────────────────────────────────────────────────

fn generate_chat_id() -> String {
    format!("{:08x}", Utc::now().timestamp_subsec_nanos())
}

fn format_response(project_id: &str, chat_id: &str, response: &str) -> String {
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
        "🗨️ <b>Chat</b>  ·  <code>{pid}</code>  ·  <code>{cid}</code>\n\
         {DIV}\n\
         {resp}{suffix}\n\
         {DIV}\n\
         <i>💬 Reply to continue  ·  🔄 Restart for new session</i>",
        pid = escape_html(project_id),
        cid = chat_id,
        resp = escape_html(&content),
    )
}

fn format_error(project_id: &str, chat_id: &str, error: &str) -> String {
    format!(
        "❌ <b>Chat error</b>  ·  <code>{pid}</code>  ·  <code>{cid}</code>\n\
         {DIV}\n\
         {err}",
        pid = escape_html(project_id),
        cid = chat_id,
        err = escape_html(error),
    )
}
