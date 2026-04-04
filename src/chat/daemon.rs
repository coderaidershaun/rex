//! Rex-chat daemon: independent Telegram bot for project chat sessions.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use chrono::Utc;
use clap::Parser;
use tracing::{error, info, warn};

use crate::autorun::state::read_state;
use crate::autorun::types::{escape_html, DIV};
use crate::errors::{RexError, RexResult};
use crate::models::planning::{PlanningStatus, PlanningStore};
use crate::models::project::ProjectRegistry;

use super::discovery;
use super::sessions::SessionManager;
use super::telegram::{ChatTelegramClient, InlineButton, Update};

/// CLI arguments for `rex-chat`.
#[derive(Parser)]
#[command(name = "rex-chat", about = "Telegram chat interface for rex projects")]
pub struct Args {
    /// Directory containing rex/projects.json
    #[arg(long, default_value = ".")]
    pub project_dir: PathBuf,

    /// Max USD budget per chat invocation
    #[arg(long, default_value = "10.0")]
    pub max_budget_usd: f64,

    /// Max agentic turns per chat invocation
    #[arg(long, default_value = "50")]
    pub max_turns: u32,

    /// Session timeout in minutes
    #[arg(long, default_value = "30")]
    pub session_timeout_mins: u64,

    /// Log file path
    #[arg(long)]
    pub log_file: Option<PathBuf>,
}

pub async fn run(args: Args) -> RexResult<ExitCode> {
    let project_dir = std::fs::canonicalize(&args.project_dir).map_err(|e| {
        RexError::FileRead {
            path: args.project_dir.display().to_string(),
            source: e,
        }
    })?;

    // Load .env
    let env_path = project_dir.join(".env");
    if env_path.exists() {
        dotenvy::from_path(&env_path).ok();
    } else {
        dotenvy::dotenv().ok();
    }

    // Read Telegram credentials
    let telegram_token = std::env::var("REX_AUTOCHAT_TELEGRAM_BOT_TOKEN").map_err(|_| RexError::EnvVar {
        name: "REX_AUTOCHAT_TELEGRAM_BOT_TOKEN".into(),
        detail: "check .env".into(),
    })?;
    let telegram_chat_id: i64 = std::env::var("REX_TELEGRAM_CHAT_ID")
        .map_err(|_| RexError::EnvVar {
            name: "REX_TELEGRAM_CHAT_ID".into(),
            detail: "check .env".into(),
        })?
        .parse()
        .map_err(|e| RexError::EnvVar {
            name: "REX_TELEGRAM_CHAT_ID".into(),
            detail: format!("must be a valid integer: {e}"),
        })?;

    // Set CWD for ProjectRegistry::load()
    std::env::set_current_dir(&project_dir).map_err(|e| RexError::FileRead {
        path: project_dir.display().to_string(),
        source: e,
    })?;

    // Initialize Telegram client
    let mut tg = ChatTelegramClient::new(telegram_token, telegram_chat_id, 0);

    // Send startup notification
    tg.notify("🏠 <b>Rex Chat online</b>\n\nSend /menu to see your projects.")
        .await;

    info!("rex-chat daemon started");

    // Session manager
    let mut sessions = SessionManager::new();
    let session_timeout = Duration::from_secs(args.session_timeout_mins * 60);

    // Signal handling
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;

    let result = tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("SIGINT received — shutting down");
            Ok(ExitCode::from(0))
        }
        _ = sigterm.recv() => {
            info!("SIGTERM received — shutting down");
            Ok(ExitCode::from(0))
        }
        result = poll_loop(&mut tg, &mut sessions, &project_dir, &args, session_timeout) => {
            result
        }
    };

    // Cleanup
    tg.notify("🏠 <b>Rex Chat offline</b>").await;
    info!("rex-chat daemon stopped");

    result
}

/// Main poll loop.
async fn poll_loop(
    tg: &mut ChatTelegramClient,
    sessions: &mut SessionManager,
    project_dir: &PathBuf,
    args: &Args,
    session_timeout: Duration,
) -> RexResult<ExitCode> {
    let mut cleanup_counter = 0u32;

    loop {
        let updates = tg.poll_updates().await;

        // Periodically cleanup stale sessions
        cleanup_counter += 1;
        if cleanup_counter % 30 == 0 {
            sessions.cleanup_stale(session_timeout);
        }

        for update in updates {
            match update {
                Update::CallbackQuery { id, data, message_id: _ } => {
                    tg.answer_callback_query(&id).await;
                    handle_callback(tg, sessions, project_dir, args, &data).await;
                }
                Update::TextMessage {
                    text,
                    reply_to_message_id,
                } => {
                    handle_text_message(
                        tg,
                        sessions,
                        project_dir,
                        args,
                        &text,
                        reply_to_message_id,
                    )
                    .await;
                }
            }
        }
    }
}

/// Handle a callback query from an inline keyboard button.
async fn handle_callback(
    tg: &mut ChatTelegramClient,
    sessions: &mut SessionManager,
    project_dir: &PathBuf,
    _args: &Args,
    data: &str,
) {
    // Parse "action:project_id" format
    let (action, target) = match data.split_once(':') {
        Some((a, t)) => (a, t),
        None => (data, ""),
    };

    match action {
        "menu" => {
            show_project_menu(tg, project_dir).await;
        }
        "chat" => {
            if !target.is_empty() {
                start_chat_prompt(tg, sessions, target).await;
            }
        }
        "start" => {
            if !target.is_empty() {
                start_autorun(tg, project_dir, target).await;
            }
        }
        "status" => {
            if !target.is_empty() {
                show_autorun_status(tg, target).await;
            }
        }
        "stop" => {
            if !target.is_empty() {
                stop_autorun(tg, project_dir, target).await;
            }
        }
        "rc_reply" => {
            if !target.is_empty() {
                start_chat_prompt(tg, sessions, target).await;
            }
        }
        _ => {
            warn!(data, "unknown callback data");
        }
    }
}

/// Handle a text message from the user.
async fn handle_text_message(
    tg: &mut ChatTelegramClient,
    sessions: &mut SessionManager,
    project_dir: &PathBuf,
    args: &Args,
    text: &str,
    reply_to: Option<i64>,
) {
    // /clear → delete recent messages
    if text.starts_with("/clear") {
        tg.clear_history().await;
        return;
    }

    // /commands → show available commands
    if text.starts_with("/commands") {
        tg.notify(
            "📋 <b>Chat Commands</b>\n\n\
             <code>/menu</code> — Show project dashboard\n\
             <code>/start</code> — Show project dashboard\n\
             <code>/commands</code> — Show this help\n\
             <code>/clear</code> — Clear chat history",
        )
        .await;
        return;
    }

    // /start or /menu → show project dashboard
    if text.starts_with("/start") || text.starts_with("/menu") {
        show_project_menu(tg, project_dir).await;
        return;
    }

    // Reply-to routing
    if let Some(reply_to_id) = reply_to {
        if let Some(project_id) = sessions.lookup_reply(reply_to_id) {
            let pid = project_id.to_string();
            invoke_chat(tg, sessions, project_dir, args, &pid, text).await;
        }
        return;
    }

    // Unrecognized message → show project menu
    show_project_menu(tg, project_dir).await;
}

// ── Action handlers ──────────────────────────────────────────────────────

/// Show the project dashboard with inline buttons.
async fn show_project_menu(tg: &mut ChatTelegramClient, project_dir: &PathBuf) {
    let projects = match discovery::discover_projects(project_dir) {
        Ok(p) => p,
        Err(e) => {
            tg.notify(&format!(
                "❌ Failed to load projects: {}",
                escape_html(&e.to_string())
            ))
            .await;
            return;
        }
    };

    if projects.is_empty() {
        tg.notify("🏠 <b>Rex Chat</b>\n\nNo projects found. Run <code>rex init</code> to create one.")
            .await;
        return;
    }

    let mut msg = format!("🏠 <b>Rex Chat</b>\n{DIV}\n");
    let mut button_rows: Vec<Vec<InlineButton>> = Vec::new();

    // Running projects
    let running: Vec<_> = projects.iter().filter(|p| p.running).collect();
    if !running.is_empty() {
        msg.push_str("\n🟢 <b>RUNNING</b>\n");
        for proj in &running {
            if let Some(ref state) = proj.autorun_state {
                let uptime = format_duration_since(&state.stats.started_at);
                msg.push_str(&format!(
                    "  <code>{id}</code> · ⏱ {uptime} · 💰 ${cost:.2} · 🔄 #{inv}\n",
                    id = escape_html(&proj.id),
                    cost = state.stats.total_cost_usd,
                    inv = state.invocation_count,
                ));
            }
            button_rows.push(vec![
                InlineButton {
                    text: format!("💬 {}", proj.id),
                    callback_data: format!("chat:{}", proj.id),
                },
                InlineButton {
                    text: "📊".to_string(),
                    callback_data: format!("status:{}", proj.id),
                },
                InlineButton {
                    text: "🛑".to_string(),
                    callback_data: format!("stop:{}", proj.id),
                },
            ]);
        }
    }

    // Available (not running) projects
    let available: Vec<_> = projects.iter().filter(|p| !p.running).collect();
    if !available.is_empty() {
        if !running.is_empty() {
            msg.push_str(&format!("\n{DIV}\n"));
        }
        msg.push_str("\n📁 <b>AVAILABLE</b>\n");
        for proj in &available {
            msg.push_str(&format!(
                "  <code>{id}</code> · {dir}\n",
                id = escape_html(&proj.id),
                dir = escape_html(&proj.directory),
            ));
            button_rows.push(vec![
                InlineButton {
                    text: format!("🚀 {}", proj.id),
                    callback_data: format!("start:{}", proj.id),
                },
                InlineButton {
                    text: "💬 Chat".to_string(),
                    callback_data: format!("chat:{}", proj.id),
                },
            ]);
        }
    }

    if let Err(e) = tg.send_with_buttons(&msg, &button_rows).await {
        error!("failed to send project menu: {e}");
    }
}

/// Send a ForceReply prompt for a chat session.
async fn start_chat_prompt(
    tg: &mut ChatTelegramClient,
    sessions: &mut SessionManager,
    project_id: &str,
) {
    let text = format!(
        "💬 <b>Chat</b>  ·  <code>{pid}</code>\n<i>Type your message below</i>",
        pid = escape_html(project_id),
    );
    match tg.send_force_reply(&text).await {
        Ok(msg_id) => {
            sessions.register_message(msg_id, project_id);
        }
        Err(e) => {
            error!("failed to send chat prompt: {e}");
        }
    }
}

/// Invoke Claude for a chat session and send the response.
async fn invoke_chat(
    tg: &mut ChatTelegramClient,
    sessions: &mut SessionManager,
    project_dir: &PathBuf,
    args: &Args,
    project_id: &str,
    message: &str,
) {
    // Find project directory
    let proj_dir = match find_project_dir(project_dir, project_id) {
        Some(d) => d,
        None => {
            tg.notify(&format!(
                "❌ Project <code>{}</code> not found",
                escape_html(project_id)
            ))
            .await;
            return;
        }
    };

    // Ensure session exists
    sessions.get_or_create(project_id, &proj_dir);

    // Send thinking indicator
    let thinking_text = format!(
        "🔍 <b>Rex Chat</b>  ·  <code>{pid}</code>\n{DIV}\nThinking...",
        pid = escape_html(project_id),
    );
    let thinking_msg_id = match tg.send_message(&thinking_text).await {
        Ok(id) => id,
        Err(e) => {
            error!("failed to send thinking message: {e}");
            return;
        }
    };

    // Invoke Claude
    let prompt = format!(
        "Project \"{pid}\" at {dir}.\n\nUser's message: {msg}",
        pid = project_id,
        dir = proj_dir,
        msg = message,
    );
    match sessions
        .invoke_claude(project_id, &prompt, args.max_turns, args.max_budget_usd)
        .await
    {
        Ok(response) => {
            let formatted = format_chat_response(project_id, &response);
            let button_rows = vec![vec![
                InlineButton {
                    text: "💬 Reply".to_string(),
                    callback_data: format!("rc_reply:{project_id}"),
                },
                InlineButton {
                    text: "🏠 Menu".to_string(),
                    callback_data: "menu".to_string(),
                },
            ]];
            if tg
                .edit_message_with_buttons(thinking_msg_id, &formatted, &button_rows)
                .await
                .is_err()
            {
                // Edit failed (message too old?), send new
                match tg.send_with_buttons(&formatted, &button_rows).await {
                    Ok(id) => {
                        sessions.register_message(id, project_id);
                    }
                    Err(e) => error!("failed to send chat response: {e}"),
                }
            } else {
                sessions.register_message(thinking_msg_id, project_id);
            }
        }
        Err(e) => {
            let error_text = format!(
                "❌ <b>Chat error</b>  ·  <code>{pid}</code>\n{DIV}\n{err}",
                pid = escape_html(project_id),
                err = escape_html(&e.to_string()),
            );
            let _ = tg.edit_message(thinking_msg_id, &error_text).await;
        }
    }
}

/// Start an autorun for a project.
async fn start_autorun(
    tg: &mut ChatTelegramClient,
    project_dir: &PathBuf,
    project_id: &str,
) {
    let proj_dir = match find_project_dir(project_dir, project_id) {
        Some(d) => d,
        None => {
            tg.notify(&format!(
                "❌ Project <code>{}</code> not found",
                escape_html(project_id)
            ))
            .await;
            return;
        }
    };

    // Check if already running
    let state_path = std::path::Path::new(&proj_dir).join(".rex-autorun.json");
    if read_state(&state_path).is_some() {
        tg.notify(&format!(
            "⚠️ <code>{}</code> is already running",
            escape_html(project_id)
        ))
        .await;
        return;
    }

    // Start autorun in background
    let cmd = format!(
        "nohup rex-autorun --project-dir {} > /dev/null 2>&1 &",
        shell_escape(&proj_dir),
    );
    match tokio::process::Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .spawn()
    {
        Ok(_) => {
            tg.notify(&format!(
                "🚀 Started autorun for <code>{}</code>",
                escape_html(project_id)
            ))
            .await;
        }
        Err(e) => {
            tg.notify(&format!(
                "❌ Failed to start autorun for <code>{pid}</code>: {err}",
                pid = escape_html(project_id),
                err = escape_html(&e.to_string()),
            ))
            .await;
        }
    }
}

/// Stop a running autorun by sending SIGTERM via its state file.
async fn stop_autorun(
    tg: &mut ChatTelegramClient,
    project_dir: &PathBuf,
    project_id: &str,
) {
    let proj_dir = match find_project_dir(project_dir, project_id) {
        Some(d) => d,
        None => {
            tg.notify(&format!(
                "❌ Project <code>{}</code> not found",
                escape_html(project_id)
            ))
            .await;
            return;
        }
    };

    let proj_path = std::path::Path::new(&proj_dir);
    let state_path = proj_path.join(".rex-autorun.json");
    if let Some(state) = read_state(&state_path) {
        if let Some(pgid) = state.claude_pgid {
            unsafe {
                libc::killpg(pgid, libc::SIGTERM);
            }
            tg.notify(&format!(
                "🛑 Sent SIGTERM to <code>{}</code> (pgid {})",
                escape_html(project_id),
                pgid
            ))
            .await;
        } else {
            tg.notify(&format!(
                "❌ <code>{}</code> has no active process group",
                escape_html(project_id)
            ))
            .await;
        }
    } else {
        tg.notify(&format!(
            "❌ <code>{}</code> doesn't appear to be running",
            escape_html(project_id)
        ))
        .await;
    }
}

/// Show autorun status for a project.
async fn show_autorun_status(tg: &mut ChatTelegramClient, project_id: &str) {
    let registry = match ProjectRegistry::load() {
        Ok(r) => r,
        Err(e) => {
            tg.notify(&format!("❌ {}", escape_html(&e.to_string())))
                .await;
            return;
        }
    };

    let proj = registry
        .active
        .iter()
        .chain(registry.inactive.iter())
        .find(|p| p.id == project_id);

    let Some(proj) = proj else {
        tg.notify(&format!(
            "❌ Project <code>{}</code> not found",
            escape_html(project_id)
        ))
        .await;
        return;
    };

    let state_path = std::path::Path::new(&proj.directory).join(".rex-autorun.json");
    let Some(state) = read_state(&state_path) else {
        tg.notify(&format!(
            "📊 <code>{}</code> — not running",
            escape_html(project_id)
        ))
        .await;
        return;
    };

    let uptime = format_duration_since(&state.stats.started_at);

    // Load task counts from planning data
    let proj_path = std::path::Path::new(&proj.directory);
    let (tasks_done, tasks_total) = task_counts(proj_path);

    let msg = format!(
        "📊 <b>Status</b>  ·  <code>{pid}</code>\n\
         {DIV}\n\
         ⏱ <b>Uptime:</b> <code>{uptime}</code>\n\
         🔄 <b>Phase:</b> <code>{phase:?}</code>\n\
         📊 <b>Invocations:</b> <code>{inv}</code>\n\
         ✅ <b>Items completed:</b> <code>{items}</code>\n\
         📋 <b>Tasks:</b> <code>{tasks_done}/{tasks_total}</code>\n\
         💰 <b>Cost:</b> <code>${cost:.2}</code>",
        pid = escape_html(project_id),
        phase = state.phase,
        inv = state.invocation_count,
        items = state.stats.items_completed,
        cost = state.stats.total_cost_usd,
    );
    tg.notify(&msg).await;
}

// ── Helpers ──────────────────────────────────────────────────────────────

/// Find the directory for a project by ID.
fn find_project_dir(_project_dir: &PathBuf, project_id: &str) -> Option<String> {
    let registry = ProjectRegistry::load().ok()?;
    registry
        .active
        .iter()
        .chain(registry.inactive.iter())
        .find(|p| p.id == project_id)
        .map(|p| p.directory.clone())
}

/// Format chat response for Telegram.
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
         {resp}{suffix}",
        pid = escape_html(project_id),
        resp = escape_html(&content),
    )
}

/// Format duration from ISO 8601 start time to now.
fn format_duration_since(started_at: &str) -> String {
    let Ok(start) = chrono::DateTime::parse_from_rfc3339(started_at) else {
        return "unknown".to_string();
    };
    let elapsed = Utc::now().signed_duration_since(start);
    let total_secs = elapsed.num_seconds().max(0);
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    if hours > 0 {
        format!("{hours}h {mins}m")
    } else {
        format!("{mins}m")
    }
}

/// Count (completed, total) tasks from planning.json.
fn task_counts(project_dir: &std::path::Path) -> (usize, usize) {
    let Ok(store) = PlanningStore::load(project_dir) else {
        return (0, 0);
    };
    let total = store.tasks.len();
    let done = store
        .tasks
        .iter()
        .filter(|t| t.status == PlanningStatus::Completed)
        .count();
    (done, total)
}

/// Simple shell escaping for paths.
fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
