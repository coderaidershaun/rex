//! Rex-chat daemon: project-agnostic Telegram bot that auto-discovers rex projects.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use chrono::Utc;
use clap::Parser;
use tracing::{error, info, warn};

use crate::autorun::state::read_state;
use crate::autorun::types::{escape_html, DIV};
use crate::errors::{RexError, RexResult};
use crate::models::planning::{PlanningStatus, PlanningStore};

use super::discovery;
use super::sessions::SessionManager;
use super::telegram::{ChatTelegramClient, InlineButton, Update};

/// CLI arguments for `rex-chat`.
#[derive(Parser)]
#[command(name = "rex-chat", about = "Telegram chat interface for rex projects")]
pub struct Args {
    /// Root directory to scan for rex projects (default: $HOME)
    #[arg(long)]
    pub scan_dir: Option<PathBuf>,

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
    // Resolve scan directory: explicit flag > $HOME > current dir
    let scan_dir = match &args.scan_dir {
        Some(d) => std::fs::canonicalize(d).map_err(|e| RexError::FileRead {
            path: d.display().to_string(),
            source: e,
        })?,
        None => {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            PathBuf::from(home)
        }
    };

    // Load .env from current directory (or parent chain)
    dotenvy::dotenv().ok();

    // Read Telegram credentials
    let telegram_token =
        std::env::var("REX_AUTOCHAT_TELEGRAM_BOT_TOKEN").map_err(|_| RexError::EnvVar {
            name: "REX_AUTOCHAT_TELEGRAM_BOT_TOKEN".into(),
            detail: "set in environment or .env".into(),
        })?;
    let telegram_chat_id: i64 = std::env::var("REX_TELEGRAM_CHAT_ID")
        .map_err(|_| RexError::EnvVar {
            name: "REX_TELEGRAM_CHAT_ID".into(),
            detail: "set in environment or .env".into(),
        })?
        .parse()
        .map_err(|e| RexError::EnvVar {
            name: "REX_TELEGRAM_CHAT_ID".into(),
            detail: format!("must be a valid integer: {e}"),
        })?;

    // Initialize Telegram client
    let mut tg = ChatTelegramClient::new(telegram_token, telegram_chat_id, 0);

    // Send startup notification
    let project_count = discovery::discover_projects(&scan_dir)
        .map(|p| p.len())
        .unwrap_or(0);
    tg.notify(&format!(
        "🏠 <b>Rex Chat online</b>\n\
         📂 Scanning <code>{dir}</code>\n\
         📋 Found <b>{n}</b> project{s}\n\n\
         Send /menu to see your projects.",
        dir = escape_html(&scan_dir.display().to_string()),
        n = project_count,
        s = if project_count == 1 { "" } else { "s" },
    ))
    .await;

    info!(
        scan_dir = %scan_dir.display(),
        projects = project_count,
        "rex-chat daemon started"
    );

    // Session manager
    let mut sessions = SessionManager::new();
    let session_timeout = Duration::from_secs(args.session_timeout_mins * 60);

    // Signal handling
    let mut sigterm =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;

    let result = tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("SIGINT received -- shutting down");
            Ok(ExitCode::from(0))
        }
        _ = sigterm.recv() => {
            info!("SIGTERM received -- shutting down");
            Ok(ExitCode::from(0))
        }
        result = poll_loop(&mut tg, &mut sessions, &scan_dir, &args, session_timeout) => {
            result
        }
    };

    // Kill all active Claude processes to prevent orphans
    sessions.kill_all();

    // Cleanup
    tg.notify("🏠 <b>Rex Chat offline</b>").await;
    info!("rex-chat daemon stopped");

    result
}

/// Main poll loop.
async fn poll_loop(
    tg: &mut ChatTelegramClient,
    sessions: &mut SessionManager,
    scan_dir: &Path,
    args: &Args,
    session_timeout: Duration,
) -> RexResult<ExitCode> {
    let mut cleanup_counter = 0u32;
    let mut snapshot = ProjectSnapshot::capture(scan_dir);
    let mut last_scan = tokio::time::Instant::now();
    let scan_interval = Duration::from_secs(5);

    loop {
        let updates = tg.poll_updates().await;

        // Periodically cleanup stale sessions
        cleanup_counter += 1;
        if cleanup_counter % 30 == 0 {
            sessions.cleanup_stale(session_timeout);
        }

        // State monitoring: detect project/autorun changes every 5 seconds
        if last_scan.elapsed() >= scan_interval {
            last_scan = tokio::time::Instant::now();
            let new_snapshot = ProjectSnapshot::capture(scan_dir);
            let changes = snapshot.diff(&new_snapshot);
            for change in &changes {
                notify_state_change(tg, change).await;
            }
            if !changes.is_empty() {
                snapshot = new_snapshot;
            }
        }

        for update in updates {
            match update {
                Update::CallbackQuery {
                    id,
                    data,
                    message_id: _,
                } => {
                    tg.answer_callback_query(&id).await;
                    handle_callback(tg, sessions, scan_dir, args, &data).await;
                }
                Update::TextMessage {
                    text,
                    reply_to_message_id,
                } => {
                    handle_text_message(
                        tg,
                        sessions,
                        scan_dir,
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
    scan_dir: &Path,
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
            show_project_menu(tg, scan_dir).await;
        }
        "chat" => {
            if !target.is_empty() {
                start_chat_prompt(tg, sessions, target).await;
            }
        }
        "start" => {
            if !target.is_empty() {
                start_autorun(tg, scan_dir, target).await;
            }
        }
        "status" => {
            if !target.is_empty() {
                show_autorun_status(tg, scan_dir, target).await;
            }
        }
        "stop" => {
            if !target.is_empty() {
                stop_autorun(tg, scan_dir, target).await;
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
    scan_dir: &Path,
    args: &Args,
    text: &str,
    reply_to: Option<i64>,
) {
    let trimmed = text.trim();

    // -- Slash commands -------------------------------------------------------

    if trimmed.starts_with('/') {
        handle_slash_command(tg, sessions, scan_dir, args, trimmed).await;
        return;
    }

    // -- Reply-to routing -----------------------------------------------------

    if let Some(reply_to_id) = reply_to {
        if let Some(project_id) = sessions.lookup_reply(reply_to_id) {
            let pid = project_id.to_string();
            invoke_chat(tg, sessions, scan_dir, args, &pid, trimmed).await;
        }
        return;
    }

    // -- Project-prefixed messages: "project-id: message" ---------------------

    let known_ids = discover_project_ids(scan_dir);
    if let Some((pid, message)) = parse_project_prefix(trimmed, &known_ids) {
        sessions.last_active = Some(pid.clone());
        invoke_chat(tg, sessions, scan_dir, args, &pid, message).await;
        return;
    }

    // -- Smart routing: recent session > single project > pick first ----------

    if let Some(target) = resolve_chat_target(sessions, scan_dir) {
        invoke_chat(tg, sessions, scan_dir, args, &target, trimmed).await;
    } else {
        // Multiple projects, no context -- pick the first one and tell the user
        let projects = discovery::discover_projects(scan_dir).unwrap_or_default();
        if projects.is_empty() {
            tg.notify("No projects found. Run <code>rex project create</code> to create one.")
                .await;
        } else {
            let first = &projects[0].id;
            sessions.last_active = Some(first.clone());
            tg.notify(&format!(
                "📋 Multiple projects found. Defaulting to <code>{}</code>.\n\
                 Use <code>/chat project-id</code> to switch.\n\
                 Use <code>/projects</code> to list all.",
                escape_html(first),
            ))
            .await;
            invoke_chat(tg, sessions, scan_dir, args, first, trimmed).await;
        }
    }
}

/// Handle slash commands.
async fn handle_slash_command(
    tg: &mut ChatTelegramClient,
    sessions: &mut SessionManager,
    scan_dir: &Path,
    args: &Args,
    text: &str,
) {
    // Split command and arguments
    let (cmd, arg) = match text.split_once(|c: char| c.is_whitespace()) {
        Some((c, a)) => (c, a.trim()),
        None => (text, ""),
    };

    match cmd {
        "/clear" => {
            tg.clear_history().await;
        }
        "/menu" => {
            show_project_menu(tg, scan_dir).await;
        }
        "/commands" | "/help" => {
            tg.notify(
                "📋 <b>Rex Chat Commands</b>\n\n\
                 <b>Project management:</b>\n\
                 <code>/start &lt;id&gt;</code> -- Start autorun\n\
                 <code>/stop &lt;id&gt;</code> -- Stop autorun\n\
                 <code>/status &lt;id&gt;</code> -- Show autorun status\n\
                 <code>/chat &lt;id&gt;</code> -- Switch active project\n\
                 <code>/menu</code> -- Show project dashboard\n\n\
                 <b>Chat:</b>\n\
                 <code>id: message</code> -- Chat with specific project\n\
                 <code>message</code> -- Chat with active project\n\n\
                 <b>Utility:</b>\n\
                 <code>/projects</code> -- List all discovered projects\n\
                 <code>/clear</code> -- Clear chat history\n\
                 <code>/commands</code> -- Show this help",
            )
            .await;
        }
        "/projects" => {
            show_project_list(tg, scan_dir).await;
        }
        "/chat" => {
            if arg.is_empty() {
                if let Some(ref active) = sessions.last_active {
                    tg.notify(&format!(
                        "💬 Active project: <code>{}</code>\n\
                         Just type your message to chat.",
                        escape_html(active),
                    ))
                    .await;
                } else {
                    tg.notify("💬 No active project. Use <code>/chat project-id</code> to select one.")
                        .await;
                }
            } else {
                // Switch active project
                let known_ids = discover_project_ids(scan_dir);
                if known_ids.contains(&arg.to_string()) {
                    sessions.last_active = Some(arg.to_string());
                    tg.notify(&format!(
                        "💬 Switched to <code>{}</code>\nJust type your message to chat.",
                        escape_html(arg),
                    ))
                    .await;
                } else {
                    tg.notify(&format!(
                        "❌ Project <code>{}</code> not found",
                        escape_html(arg),
                    ))
                    .await;
                }
            }
        }
        "/start" => {
            if arg.is_empty() {
                show_project_menu(tg, scan_dir).await;
            } else {
                start_autorun(tg, scan_dir, arg).await;
            }
        }
        "/stop" => {
            if arg.is_empty() {
                tg.notify("Usage: <code>/stop project-id</code>").await;
            } else {
                stop_autorun(tg, scan_dir, arg).await;
            }
        }
        "/status" => {
            if arg.is_empty() {
                // Show status for active project or all running
                show_all_running_status(tg, scan_dir).await;
            } else {
                show_autorun_status(tg, scan_dir, arg).await;
            }
        }
        _ => {
            // Unknown slash command -- treat as chat if we have context
            if let Some(target) = resolve_chat_target(sessions, scan_dir) {
                invoke_chat(tg, sessions, scan_dir, args, &target, text).await;
            } else {
                tg.notify(&format!(
                    "Unknown command. Send <code>/commands</code> for help.",
                ))
                .await;
            }
        }
    }
}

// -- Action handlers ----------------------------------------------------------

/// Show the project dashboard with inline buttons.
async fn show_project_menu(tg: &mut ChatTelegramClient, scan_dir: &Path) {
    let projects = match discovery::discover_projects(scan_dir) {
        Ok(p) => p,
        Err(e) => {
            tg.notify(&format!(
                "❌ Failed to discover projects: {}",
                escape_html(&e.to_string())
            ))
            .await;
            return;
        }
    };

    if projects.is_empty() {
        tg.notify(
            "🏠 <b>Rex Chat</b>\n\nNo projects found. \
             Run <code>rex project create</code> to create one.",
        )
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
    scan_dir: &Path,
    args: &Args,
    project_id: &str,
    message: &str,
) {
    // Find project directory
    let proj_dir = match find_project_dir(scan_dir, project_id) {
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
        "🔍 <b>Rex Chat</b>  ·  <code>{pid}</code>\n{DIV}\n{status}",
        pid = escape_html(project_id),
        status = random_thinking_message(),
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
    scan_dir: &Path,
    project_id: &str,
) {
    let proj_dir = match find_project_dir(scan_dir, project_id) {
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
    let state_path = Path::new(&proj_dir).join(".rex-autorun.json");
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
    scan_dir: &Path,
    project_id: &str,
) {
    let proj_dir = match find_project_dir(scan_dir, project_id) {
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

    let proj_path = Path::new(&proj_dir);
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
async fn show_autorun_status(
    tg: &mut ChatTelegramClient,
    scan_dir: &Path,
    project_id: &str,
) {
    let projects = match discovery::discover_projects(scan_dir) {
        Ok(p) => p,
        Err(e) => {
            tg.notify(&format!("❌ {}", escape_html(&e.to_string())))
                .await;
            return;
        }
    };

    let Some(proj) = projects.iter().find(|p| p.id == project_id) else {
        tg.notify(&format!(
            "❌ Project <code>{}</code> not found",
            escape_html(project_id)
        ))
        .await;
        return;
    };

    let state_path = Path::new(&proj.directory).join(".rex-autorun.json");
    let Some(state) = read_state(&state_path) else {
        tg.notify(&format!(
            "📊 <code>{}</code> -- not running",
            escape_html(project_id)
        ))
        .await;
        return;
    };

    let uptime = format_duration_since(&state.stats.started_at);

    // Load task counts from planning data
    let proj_path = Path::new(&proj.directory);
    let (tasks_done, tasks_total) = task_counts(proj_path, &proj.id);

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

// -- State monitoring ---------------------------------------------------------

/// Snapshot of project state for change detection.
struct ProjectSnapshot {
    /// project_id -> running status
    projects: HashMap<String, bool>,
}

enum StateChange {
    ProjectAdded { id: String, running: bool },
    ProjectRemoved { id: String },
    AutorunStarted { id: String },
    AutorunStopped { id: String },
}

impl ProjectSnapshot {
    fn capture(scan_dir: &Path) -> Self {
        let projects = discovery::discover_projects(scan_dir)
            .unwrap_or_default()
            .into_iter()
            .map(|p| (p.id, p.running))
            .collect();
        Self { projects }
    }

    fn diff(&self, new: &Self) -> Vec<StateChange> {
        let mut changes = Vec::new();

        // New projects
        for (id, &running) in &new.projects {
            if !self.projects.contains_key(id) {
                changes.push(StateChange::ProjectAdded {
                    id: id.clone(),
                    running,
                });
            }
        }

        // Removed projects
        for id in self.projects.keys() {
            if !new.projects.contains_key(id) {
                changes.push(StateChange::ProjectRemoved { id: id.clone() });
            }
        }

        // Autorun status changes
        for (id, &new_running) in &new.projects {
            if let Some(&old_running) = self.projects.get(id) {
                if !old_running && new_running {
                    changes.push(StateChange::AutorunStarted { id: id.clone() });
                } else if old_running && !new_running {
                    changes.push(StateChange::AutorunStopped { id: id.clone() });
                }
            }
        }

        changes
    }
}

async fn notify_state_change(tg: &mut ChatTelegramClient, change: &StateChange) {
    let msg = match change {
        StateChange::ProjectAdded { id, running } => {
            let status = if *running { " (autorun active)" } else { "" };
            info!(project = %id, "new project discovered");
            format!(
                "📦 <b>New project discovered</b>\n<code>{}</code>{status}",
                escape_html(id),
            )
        }
        StateChange::ProjectRemoved { id } => {
            info!(project = %id, "project removed");
            format!(
                "🗑 <b>Project removed</b>\n<code>{}</code>",
                escape_html(id),
            )
        }
        StateChange::AutorunStarted { id } => {
            info!(project = %id, "autorun started");
            format!(
                "🟢 <b>Autorun started</b>\n<code>{}</code>",
                escape_html(id),
            )
        }
        StateChange::AutorunStopped { id } => {
            info!(project = %id, "autorun stopped");
            format!(
                "🔴 <b>Autorun stopped</b>\n<code>{}</code>",
                escape_html(id),
            )
        }
    };
    tg.notify(&msg).await;
}

// -- Helpers ------------------------------------------------------------------

/// Determine which project a bare message should route to.
/// Returns the project ID if routing is unambiguous.
fn resolve_chat_target(sessions: &SessionManager, scan_dir: &Path) -> Option<String> {
    // If the user has chatted recently, continue that conversation
    if let Some(ref last) = sessions.last_active {
        return Some(last.clone());
    }
    // If there's exactly one project, route there
    let projects = discovery::discover_projects(scan_dir).ok()?;
    if projects.len() == 1 {
        return Some(projects[0].id.clone());
    }
    None
}

/// Parse "project-id: message" or "project-id message" prefix syntax.
/// Only matches if the prefix is a known project ID.
fn parse_project_prefix<'a>(text: &'a str, known_ids: &[String]) -> Option<(String, &'a str)> {
    // Try "id: message" first
    if let Some((prefix, rest)) = text.split_once(':') {
        let prefix = prefix.trim();
        if known_ids.iter().any(|id| id == prefix) {
            return Some((prefix.to_string(), rest.trim()));
        }
    }
    // Try "id message" (first word is a project ID)
    if let Some((first, rest)) = text.split_once(|c: char| c.is_whitespace()) {
        let first = first.trim();
        if known_ids.iter().any(|id| id == first) {
            return Some((first.to_string(), rest.trim()));
        }
    }
    None
}

/// Get all known project IDs from discovery.
fn discover_project_ids(scan_dir: &Path) -> Vec<String> {
    discovery::discover_projects(scan_dir)
        .unwrap_or_default()
        .into_iter()
        .map(|p| p.id)
        .collect()
}

/// Show a compact text list of all discovered projects.
async fn show_project_list(tg: &mut ChatTelegramClient, scan_dir: &Path) {
    let projects = match discovery::discover_projects(scan_dir) {
        Ok(p) => p,
        Err(e) => {
            tg.notify(&format!("❌ {}", escape_html(&e.to_string())))
                .await;
            return;
        }
    };

    if projects.is_empty() {
        tg.notify("No projects found.").await;
        return;
    }

    let mut msg = format!("📋 <b>Projects</b> ({})\n{DIV}\n", projects.len());
    for p in &projects {
        let status = if p.running { "🟢" } else { "⚪" };
        msg.push_str(&format!(
            "{status} <code>{id}</code> -- {dir}\n",
            id = escape_html(&p.id),
            dir = escape_html(&p.directory),
        ));
    }
    tg.notify(&msg).await;
}

/// Show status for all currently running autoruns.
async fn show_all_running_status(tg: &mut ChatTelegramClient, scan_dir: &Path) {
    let projects = match discovery::discover_projects(scan_dir) {
        Ok(p) => p,
        Err(e) => {
            tg.notify(&format!("❌ {}", escape_html(&e.to_string())))
                .await;
            return;
        }
    };

    let running: Vec<_> = projects.iter().filter(|p| p.running).collect();
    if running.is_empty() {
        tg.notify("No autoruns currently running.").await;
        return;
    }

    let mut msg = format!("📊 <b>Running Autoruns</b> ({})\n{DIV}\n", running.len());
    for proj in &running {
        if let Some(ref state) = proj.autorun_state {
            let uptime = format_duration_since(&state.stats.started_at);
            let proj_path = Path::new(&proj.directory);
            let (tasks_done, tasks_total) = task_counts(proj_path, &proj.id);
            msg.push_str(&format!(
                "\n🟢 <code>{id}</code>\n\
                 ⏱ {uptime} · 🔄 #{inv} · 📋 {done}/{total} · 💰 ${cost:.2}\n",
                id = escape_html(&proj.id),
                inv = state.invocation_count,
                done = tasks_done,
                total = tasks_total,
                cost = state.stats.total_cost_usd,
            ));
        }
    }
    tg.notify(&msg).await;
}

/// Find the directory for a project by ID via filesystem discovery.
fn find_project_dir(scan_dir: &Path, project_id: &str) -> Option<String> {
    let projects = discovery::discover_projects(scan_dir).ok()?;
    projects
        .into_iter()
        .find(|p| p.id == project_id)
        .map(|p| p.directory)
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
        "\n...\n\n<i>(truncated)</i>"
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
fn task_counts(project_dir: &Path, project_id: &str) -> (usize, usize) {
    let rex_project_dir = project_dir.join("rex").join(project_id);
    let Ok(store) = PlanningStore::load(&rex_project_dir) else {
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

/// Pick a random thinking/working status message.
fn random_thinking_message() -> &'static str {
    const MESSAGES: &[&str] = &[
        "Thinking...",
        "On it...",
        "Working on it...",
        "Let me look into that...",
        "Digging in...",
        "Give me a moment...",
        "Processing...",
        "Looking into it...",
        "One sec...",
        "Checking...",
        "Pulling up the details...",
        "Investigating...",
        "Hang tight...",
        "Let me check...",
        "Spinning up...",
        "Crunching...",
        "Searching the codebase...",
        "Reading the code...",
        "Analyzing...",
        "Diving in...",
        "Looking around...",
        "Running through it...",
        "Figuring it out...",
        "Browsing the project...",
        "Scanning...",
        "Taking a look...",
        "Getting the context...",
        "Picking up where we left off...",
        "Loading context...",
        "Rifling through the code...",
        "Parsing your request...",
        "Consulting the source...",
        "Let me see...",
        "Poking around...",
        "Tracing through...",
        "Gathering info...",
        "Sifting through the files...",
        "Assembling an answer...",
        "Bear with me...",
        "Reviewing the code...",
        "Mapping it out...",
        "Following the trail...",
        "Chasing that down...",
        "Pulling threads...",
        "Wiring it together...",
        "Reading up...",
        "Checking the source...",
        "Almost there...",
        "Connecting the dots...",
        "Mulling it over...",
    ];
    let idx = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as usize
        % MESSAGES.len();
    MESSAGES[idx]
}

/// Simple shell escaping for paths.
fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
