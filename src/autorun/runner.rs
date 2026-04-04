use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use crate::errors::{RexError, RexResult};
use chrono::Utc;
use clap::Parser;
use tracing::{error, info, warn};

use crate::models::project::ProjectRegistry;

use super::claude::{self, is_retryable};
use super::inbox;
use super::state::{self, RecoveryAction};
use super::telegram::{TelegramClient, TelegramPollResult};
use super::types::*;

/// CLI arguments for `rex-autorun`.
#[derive(Parser)]
#[command(name = "rex-autorun", about = "Headless autopilot for rex projects")]
pub struct Args {
    /// Rex project root directory (default: current directory)
    #[arg(long, default_value = ".")]
    pub project_dir: PathBuf,

    /// Max USD budget per claude invocation
    #[arg(long, default_value = "50.0")]
    pub max_budget_usd: f64,

    /// Max total USD budget across ALL invocations (hard stop)
    #[arg(long, default_value = "500.0")]
    pub max_total_budget_usd: f64,

    /// Max agentic turns per claude invocation
    #[arg(long, default_value = "200")]
    pub max_turns: u32,

    /// Claude process timeout in minutes
    #[arg(long, default_value = "60")]
    pub process_timeout_mins: u64,

    /// Max retries for transient failures
    #[arg(long, default_value = "5")]
    pub max_retries: u32,

    /// Human reply timeout in days
    #[arg(long, default_value = "1")]
    pub human_timeout_days: u64,

    /// Log file path (default: <project-dir>/.rex-autorun.log)
    #[arg(long)]
    pub log_file: Option<PathBuf>,
}

/// Main entry point — the core state machine.
// Exit codes:
//   0 — project completed successfully
//   1 — fatal/non-retryable error
//   2 — human reply timeout
//   3 — max retries exhausted
//   4 — SIGINT/SIGTERM
//   5 — budget limit reached
//   6 — killed via /kill command
pub async fn run(args: Args) -> RexResult<ExitCode> {
    // Resolve project directory to absolute path
    let project_dir = std::fs::canonicalize(&args.project_dir)
        .map_err(|e| RexError::FileRead { path: args.project_dir.display().to_string(), source: e })?;

    // Load .env from project dir
    let env_path = project_dir.join(".env");
    if env_path.exists() {
        dotenvy::from_path(&env_path).ok();
    } else {
        dotenvy::dotenv().ok();
    }

    // Read Telegram credentials from env
    let telegram_token = std::env::var("REX_AUTORUN_TELEGRAM_BOT_TOKEN")
        .map_err(|_| RexError::EnvVar { name: "REX_AUTORUN_TELEGRAM_BOT_TOKEN".into(), detail: "check .env".into() })?;
    let telegram_chat_id: i64 = std::env::var("REX_TELEGRAM_CHAT_ID")
        .map_err(|_| RexError::EnvVar { name: "REX_TELEGRAM_CHAT_ID".into(), detail: "check .env".into() })?
        .parse()
        .map_err(|e| RexError::EnvVar { name: "REX_TELEGRAM_CHAT_ID".into(), detail: format!("must be a valid integer: {e}") })?;

    // Load project info from projects.json
    // ProjectRegistry::load() reads from cwd, so we set cwd first.
    std::env::set_current_dir(&project_dir)
        .map_err(|e| RexError::FileRead { path: project_dir.display().to_string(), source: e })?;

    let registry = ProjectRegistry::load()?;
    let project = registry.active
        .ok_or_else(|| RexError::NotFound("no active rex project — run `rex project set-active` first".into()))?;

    let project_id = project.id.clone();
    let project_title = project.title.clone();
    let project_directory = project.directory.clone();

    info!(
        project_id = %project_id,
        project_title = %project_title,
        "loaded active project"
    );

    // State file and log file paths
    let state_path = project_dir.join(".rex-autorun.json");
    let log_path = args.log_file.clone().unwrap_or_else(|| project_dir.join(".rex-autorun.log"));

    // Register in the autorun registry early — before recovery, so that the
    // recovery path participates in cooperative triage and other autoruns can
    // route messages to us. The guard ensures deregistration on ALL exit paths.
    let _ = inbox::register_autorun(
        &project_dir,
        &project_id,
        inbox::AutorunEntry {
            pid: std::process::id(),
            project_dir: project_dir.display().to_string(),
            expected_message_id: None,
        },
    );
    let _registry_guard = RegistryGuard {
        root_dir: &project_dir,
        project_id: &project_id,
    };

    // Check/recover state
    let recovery = state::recover_state(&state_path);
    let (mut stats, mut invocation_count, telegram_offset) = match recovery {
        RecoveryAction::CleanStart => {
            info!("clean start — no state file found");
            let stats = RunStats {
                started_at: Utc::now().to_rfc3339(),
                ..Default::default()
            };
            (stats, 0u32, None)
        }
        RecoveryAction::StartFresh { stats, invocation_count } => {
            info!(invocation_count, "recovering from running state — starting fresh");
            (stats, invocation_count, None)
        }
        RecoveryAction::ResumePendingInput {
            session_id,
            question,
            telegram_update_offset,
            stats,
            invocation_count,
        } => {
            info!(
                session_id = %session_id,
                "recovering pending_input — will re-send question to Telegram"
            );

            let mut tg = TelegramClient::new(
                telegram_token.clone(),
                telegram_chat_id,
                telegram_update_offset,
                project_dir.clone(),
                project_id.clone(),
            );

            let msg = format!(
                "💬 <b>Input needed (recovered)</b>  ·  <code>{pid}</code>\n\
                 {DIV}\n\
                 <blockquote>{q}</blockquote>\n\n\
                 <i>Reply to this message with your answer</i>",
                pid = escape_html(&project_id),
                q = escape_html(&question),
            );
            let recovery_msg_id = match tg.send_question(&msg).await {
                Ok(id) => {
                    inbox::update_expected_message_id(&project_dir, &project_id, Some(id));
                    id
                }
                Err(e) => {
                    error!("failed to send recovered question: {e}");
                    tg.notify(&format!(
                        "⚠️ <b>Error sending recovered question</b>  ·  <code>{pid}</code>\n{DIV}\n{err}",
                        pid = escape_html(&project_id),
                        err = escape_html(&e.to_string()),
                    )).await;
                    state::delete_state(&state_path);
                    return Ok(ExitCode::from(1));
                }
            };

            // Wait for reply (with reply-to matching)
            let human_timeout = Duration::from_secs(args.human_timeout_days * 86400);
            let recovery_query = build_query_response(&project_id, &stats, invocation_count);
            match tg.wait_for_reply(recovery_msg_id, &project_id, human_timeout, &recovery_query).await {
                Ok(TelegramPollResult::Reply(reply)) => {
                    send_ack(&tg, &project_id).await;

                    log_event(&log_path, &LogEvent::InputReceived {
                        reply_length: reply.len(),
                        timestamp: now_iso(),
                    });

                    state::delete_state(&state_path);

                    // Resume the claude session with the user's reply
                    let timeout = Duration::from_secs(args.process_timeout_mins * 60);
                    let session_name = format!("rex-autorun-{project_id}-{invocation_count}");

                    let spawned = match claude::spawn_claude(
                        &project_dir,
                        &reply,
                        Some(&session_id),
                        &session_name,
                        args.max_turns,
                        args.max_budget_usd,
                    ) {
                        Ok(s) => s,
                        Err(e) => {
                            error!("failed to spawn resume: {e}");
                            tg.notify(&format!(
                                "⚠️ <b>Error spawning resume</b>  ·  <code>{pid}</code>\n{DIV}\n{err}",
                                pid = escape_html(&project_id),
                                err = escape_html(&e.to_string()),
                            )).await;
                            return Ok(ExitCode::from(1));
                        }
                    };
                    let pgid = spawned.pgid;

                    let recovery_q2 = build_query_response(&project_id, &stats, invocation_count);
                    let invoke_result = tokio::select! {
                        result = claude::await_claude(spawned, timeout) => result,
                        _ = tg.poll_for_kill(&project_id, &recovery_q2) => {
                            claude::kill_process_group(pgid).await;
                            Err(RexError::Killed)
                        }
                    };

                    match invoke_result {
                        Ok((output, _pgid)) => {
                            let cost = output.effective_cost();
                            let mut recovered_stats = stats;
                            recovered_stats.total_cost_usd += cost;
                            recovered_stats.invocations_completed += 1;
                            recovered_stats.push_context_percent(output.context_percent());
                            recovered_stats.push_session_duration_ms(output.duration_ms);

                            // Parse and handle — fall through to main loop
                            let op_result = claude::parse_operator_result(&output.result);

                            log_event(&log_path, &LogEvent::InvocationCompleted {
                                n: invocation_count,
                                status: match &op_result {
                                    Ok(r) => format!("{:?}", r.status),
                                    Err(_) => "parse_error".to_string(),
                                },
                                message: match &op_result {
                                    Ok(r) => r.message.clone(),
                                    Err(e) => e.to_string(),
                                },
                                session_id: output.session_id.clone(),
                                cost_usd: cost,
                                duration_ms: output.duration_ms,
                                timestamp: now_iso(),
                            });

                            match op_result {
                                Ok(r) if r.status == OperatorStatus::ProjectDone => {
                                    tg.notify(&format!(
                                        "🏁 <b>Project complete!</b>  ·  <code>{pid}</code>{itag}\n\
                                         {DIV}\n\
                                         {stats}\n\
                                         📊 <code>{inv}</code> invocations  ·  💰 <code>${cost:.2}</code>",
                                        pid = escape_html(&project_id),
                                        itag = item_tag(&r.item),
                                        stats = output.telegram_stats(),
                                        inv = recovered_stats.invocations_completed,
                                        cost = recovered_stats.total_cost_usd,
                                    )).await;
                                    state::delete_state(&state_path);
                                    return Ok(ExitCode::from(0));
                                }
                                _ => {
                                    // Continue to main loop
                                    (recovered_stats, invocation_count + 1, Some(tg.update_offset))
                                }
                            }
                        }
                        Err(ref e) if matches!(e, RexError::Killed) => {
                            return Ok(handle_kill(&tg, &project_id, &log_path, &state_path).await);
                        }
                        Err(RexError::AuthExpired(_)) => {
                            warn!("claude auth expired during recovery, attempting refresh");
                            match attempt_auth_refresh(&mut tg, &project_id, &project_dir, &log_path).await {
                                Ok(true) => {
                                    // Auth refreshed — fall through to main loop
                                    (stats, invocation_count, Some(tg.update_offset))
                                }
                                Ok(false) => {
                                    state::delete_state(&state_path);
                                    return Ok(ExitCode::from(1));
                                }
                                Err(RexError::Killed) => {
                                    return Ok(handle_kill(&tg, &project_id, &log_path, &state_path).await);
                                }
                                Err(e) => {
                                    error!("auth refresh failed: {e}");
                                    state::delete_state(&state_path);
                                    return Ok(ExitCode::from(1));
                                }
                            }
                        }
                        Err(e) => {
                            error!("resume invocation failed: {e}");
                            tg.notify(&format!(
                                "⚠️ <b>Error on resume</b>  ·  <code>{pid}</code>\n{DIV}\n{err}",
                                pid = escape_html(&project_id),
                                err = escape_html(&e.to_string()),
                            )).await;
                            (stats, invocation_count, Some(tg.update_offset))
                        }
                    }
                }
                Ok(TelegramPollResult::Kill) => {
                    return Ok(handle_kill(&tg, &project_id, &log_path, &state_path).await);
                }
                Err(e) => {
                    error!("human reply timeout: {e}");
                    tg.notify(&format!(
                        "⏰ <b>Timeout waiting for reply</b>  ·  <code>{pid}</code>\n{DIV}\nShutting down",
                        pid = escape_html(&project_id),
                    )).await;
                    state::delete_state(&state_path);
                    return Ok(ExitCode::from(2));
                }
            }
        }
    };

    // Create Telegram client for main loop
    let mut tg = TelegramClient::new(telegram_token, telegram_chat_id, telegram_offset, project_dir.clone(), project_id.clone());

    // Log + notify start
    log_event(&log_path, &LogEvent::Started {
        project_id: project_id.clone(),
        timestamp: now_iso(),
    });
    tg.notify_with_buttons(&format!(
        "🚀 <b>Autorun started</b>  ·  <code>{pid}</code>\n\
         {DIV}\n\
         📂 <b>Project:</b> {pt}\n\
         📁 <b>Directory:</b> <code>{pd}</code>\n\
         {DIV}\n\
         <b>Commands:</b>\n\
         <code>/kill {pid}</code> — stop autorun\n\
         <code>/query {pid}</code> — show live stats",
        pid = escape_html(&project_id),
        pt = escape_html(&project_title),
        pd = escape_html(&project_directory),
    ), &project_id).await;

    // Signal handling + main loop
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;

    let result = tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("SIGINT received — shutting down");
            tg.notify(&format!("🛑 <b>Autorun stopped</b>  ·  <code>{pid}</code>\n{DIV}\nSIGINT received", pid = escape_html(&project_id))).await;
            state::delete_state(&state_path);
            Ok(ExitCode::from(4))
        }
        _ = sigterm.recv() => {
            info!("SIGTERM received — shutting down");
            tg.notify(&format!("🛑 <b>Autorun stopped</b>  ·  <code>{pid}</code>\n{DIV}\nSIGTERM received", pid = escape_html(&project_id))).await;
            state::delete_state(&state_path);
            Ok(ExitCode::from(4))
        }
        result = main_loop(
            &args,
            &project_dir,
            &project_id,
            &state_path,
            &log_path,
            &mut tg,
            &mut stats,
            &mut invocation_count,
        ) => {
            result
        }
    };

    // Deregister from the autorun registry
    // _registry_guard drops here, ensuring deregistration on all exit paths
    result
}

/// The core invocation loop.
#[allow(clippy::too_many_arguments)]
async fn main_loop(
    args: &Args,
    project_dir: &Path,
    project_id: &str,
    state_path: &Path,
    log_path: &Path,
    tg: &mut TelegramClient,
    stats: &mut RunStats,
    invocation_count: &mut u32,
) -> RexResult<ExitCode> {
    let mut consecutive_errors = 0u32;
    let mut auth_refreshed = false;

    loop {
        // Budget check
        if stats.total_cost_usd >= args.max_total_budget_usd {
            warn!(
                total = stats.total_cost_usd,
                limit = args.max_total_budget_usd,
                "total budget exceeded"
            );
            tg.notify(&format!(
                "💸 <b>Budget limit reached</b>  ·  <code>{pid}</code>\n\
                 {DIV}\n\
                 💰 <code>${used:.2}</code> / <code>${limit:.2}</code> — stopping",
                pid = escape_html(project_id),
                used = stats.total_cost_usd,
                limit = args.max_total_budget_usd,
            ))
            .await;
            state::delete_state(state_path);
            return Ok(ExitCode::from(5));
        }

        *invocation_count += 1;
        let n = *invocation_count;
        let session_name = format!("rex-autorun-{project_id}-{n}");
        let timeout = Duration::from_secs(args.process_timeout_mins * 60);

        log_event(log_path, &LogEvent::InvocationStarted {
            n,
            timestamp: now_iso(),
        });

        // Write running state before spawn
        let running_state = AutorunState {
            phase: AutorunPhase::Running,
            session_id: None,
            claude_pid: None,
            claude_pgid: None,
            pending_question: None,
            telegram_message_id: None,
            telegram_update_offset: Some(tg.update_offset),
            invocation_count: n,
            updated_at: now_iso(),
            stats: stats.clone(),
        };
        if let Err(e) = state::write_state_atomic(state_path, &running_state) {
            warn!("failed to write running state: {e}");
        }

        // Spawn claude and record PID for orphan cleanup
        let spawned = match claude::spawn_claude(
            project_dir,
            "/rex-operator",
            None,
            &session_name,
            args.max_turns,
            args.max_budget_usd,
        ) {
            Ok(s) => s,
            Err(e) => {
                // Treat spawn failure as an invocation error
                let err_str = e.to_string();
                consecutive_errors += 1;

                log_event(log_path, &LogEvent::Error {
                    message: err_str.clone(),
                    retryable: is_retryable(&err_str),
                    attempt: consecutive_errors,
                    timestamp: now_iso(),
                });

                if is_retryable(&err_str) && consecutive_errors <= args.max_retries {
                    let delay = retry_delay(consecutive_errors);
                    warn!(attempt = consecutive_errors, max = args.max_retries, delay_secs = delay, "retryable spawn error, backing off");
                    tg.notify(&format!(
                        "⚠️ <b>Error</b>  ·  <code>{pid}</code>\n{DIV}\n{err}\n\n🔄 Retrying in {delay}s (attempt {consecutive_errors}/{max})",
                        pid = escape_html(project_id), err = escape_html(&err_str), max = args.max_retries,
                    )).await;
                    state::delete_state(state_path);
                    tokio::time::sleep(Duration::from_secs(delay)).await;
                    continue;
                }

                error!("non-retryable spawn error: {err_str}");
                tg.notify(&format!(
                    "🚨 <b>Fatal error</b>  ·  <code>{pid}</code>\n{DIV}\n{err}",
                    pid = escape_html(project_id), err = escape_html(&err_str),
                )).await;
                state::delete_state(state_path);
                return Ok(ExitCode::from(1));
            }
        };

        let pgid = spawned.pgid;

        // Update state with PID/PGID so crash recovery can kill orphans
        let mut with_pid = running_state.clone();
        with_pid.claude_pid = Some(spawned.pid);
        with_pid.claude_pgid = Some(spawned.pgid);
        let _ = state::write_state_atomic(state_path, &with_pid);

        // Race await_claude against /kill command from Telegram
        let query_resp = build_query_response(project_id, stats, n);
        let invoke_result = tokio::select! {
            result = claude::await_claude(spawned, timeout) => result,
            _ = tg.poll_for_kill(project_id, &query_resp) => {
                claude::kill_process_group(pgid).await;
                Err(RexError::Killed)
            }
        };

        match invoke_result {
            Ok((output, _pgid)) => {
                consecutive_errors = 0;
                let cost = output.effective_cost();
                stats.total_cost_usd += cost;
                stats.invocations_completed += 1;
                stats.push_context_percent(output.context_percent());
                stats.push_session_duration_ms(output.duration_ms);

                let op_result = claude::parse_operator_result(&output.result);

                log_event(log_path, &LogEvent::InvocationCompleted {
                    n,
                    status: match &op_result {
                        Ok(r) => format!("{:?}", r.status),
                        Err(_) => "parse_error".to_string(),
                    },
                    message: match &op_result {
                        Ok(r) => r.message.clone(),
                        Err(e) => e.to_string(),
                    },
                    session_id: output.session_id.clone(),
                    cost_usd: cost,
                    duration_ms: output.duration_ms,
                    timestamp: now_iso(),
                });

                match op_result {
                    Ok(result) => match result.status {
                        OperatorStatus::Completed => {
                            stats.items_completed += 1;
                            tg.notify_with_status_buttons(&format!(
                                "✅ <b>Completed #{n}</b>  ·  <code>{pid}</code>{itag}\n\
                                 {DIV}\n\
                                 {msg}\n\
                                 {DIV}\n\
                                 {stats}\n\
                                 💰 <code>${cost:.2}</code>  ·  ⏱ <code>{dur}</code>",
                                pid = escape_html(project_id),
                                itag = item_tag(&result.item),
                                msg = escape_html(&result.message),
                                stats = output.telegram_stats(),
                                cost = cost,
                                dur = format_duration_ms(output.duration_ms),
                            ), project_id)
                            .await;

                            state::delete_state(state_path);
                            // Brief cooldown to avoid hammering the API and let the filesystem settle.
                            info!(n, "invocation completed, cooling down 5s");
                            tokio::time::sleep(Duration::from_secs(5)).await;
                            continue;
                        }
                        OperatorStatus::ProjectDone => {
                            let duration = format_duration_since(&stats.started_at);
                            tg.notify(&format!(
                                "🏁 <b>Project complete!</b>  ·  <code>{pid}</code>{itag}\n\
                                 {DIV}\n\
                                 {stats}\n\
                                 📊 <code>{inv}</code> invocations  ·  💰 <code>${cost:.2}</code>  ·  ⏱ <code>{duration}</code>",
                                pid = escape_html(project_id),
                                itag = item_tag(&result.item),
                                stats = output.telegram_stats(),
                                inv = stats.invocations_completed,
                                cost = stats.total_cost_usd,
                            ))
                            .await;

                            log_event(log_path, &LogEvent::ProjectDone {
                                total_cost_usd: stats.total_cost_usd,
                                total_invocations: stats.invocations_completed,
                                total_duration: duration,
                                timestamp: now_iso(),
                            });

                            state::delete_state(state_path);
                            return Ok(ExitCode::from(0));
                        }
                        OperatorStatus::NeedsInput => {
                            // Multi-round input loop: ask → wait → resume → check.
                            // Loops here until the session returns something other
                            // than NeedsInput, preserving conversation context.
                            let current_item = result.item;
                            let mut current_question = result.message;
                            let mut current_session_id = output.session_id.clone();

                            loop {
                                info!(session_id = %current_session_id, "needs user input");

                                // Persist pending state (survives crash)
                                let pending_state = AutorunState {
                                    phase: AutorunPhase::PendingInput,
                                    session_id: Some(current_session_id.clone()),
                                    claude_pid: None,
                                    claude_pgid: None,
                                    pending_question: Some(current_question.clone()),
                                    telegram_message_id: None,
                                    telegram_update_offset: Some(tg.update_offset),
                                    invocation_count: n,
                                    updated_at: now_iso(),
                                    stats: stats.clone(),
                                };
                                let _ = state::write_state_atomic(state_path, &pending_state);

                                log_event(log_path, &LogEvent::NeedsInput {
                                    question: current_question.clone(),
                                    session_id: current_session_id.clone(),
                                    timestamp: now_iso(),
                                });

                                // Send question to Telegram with force_reply
                                let msg = format!(
                                    "💬 <b>Input needed</b>  ·  <code>{pid}</code>{itag}\n\
                                     {DIV}\n\
                                     <blockquote>{q}</blockquote>\n\
                                     {DIV}\n\
                                     {stats}\n\n\
                                     <i>Reply to this message with your answer</i>",
                                    pid = escape_html(project_id),
                                    itag = item_tag(&current_item),
                                    q = escape_html(&current_question),
                                    stats = output.telegram_stats(),
                                );
                                let question_msg_id = match tg.send_question(&msg).await {
                                    Ok(id) => {
                                        inbox::update_expected_message_id(project_dir, project_id, Some(id));
                                        let mut updated = pending_state;
                                        updated.telegram_message_id = Some(id);
                                        updated.telegram_update_offset = Some(tg.update_offset);
                                        let _ = state::write_state_atomic(state_path, &updated);
                                        id
                                    }
                                    Err(e) => {
                                        error!("failed to send question: {e}");
                                        tg.notify(&format!(
                                            "⚠️ <b>Error sending question</b>  ·  <code>{pid}</code>\n{DIV}\n{err}",
                                            pid = escape_html(project_id),
                                            err = escape_html(&e.to_string()),
                                        )).await;
                                        state::delete_state(state_path);
                                        return Ok(ExitCode::from(1));
                                    }
                                };

                                // Wait for reply (with reply-to matching)
                                let human_timeout = Duration::from_secs(args.human_timeout_days * 86400);
                                let input_query = build_query_response(project_id, stats, n);
                                let reply = match tg.wait_for_reply(question_msg_id, project_id, human_timeout, &input_query).await {
                                    Ok(TelegramPollResult::Reply(r)) => r,
                                    Ok(TelegramPollResult::Kill) => {
                                        return Ok(handle_kill(tg, project_id, log_path, state_path).await);
                                    }
                                    Err(e) => {
                                        error!("human reply timeout: {e}");
                                        tg.notify(&format!(
                                            "⏰ <b>Timeout waiting for reply</b>  ·  <code>{pid}</code>\n{DIV}\nShutting down",
                                            pid = escape_html(project_id),
                                        )).await;
                                        state::delete_state(state_path);
                                        return Ok(ExitCode::from(2));
                                    }
                                };

                                // Send acknowledgment
                                send_ack(tg, project_id).await;
                                info!(reply_len = reply.len(), "got user reply, resuming session");
                                log_event(log_path, &LogEvent::InputReceived {
                                    reply_length: reply.len(),
                                    timestamp: now_iso(),
                                });
                                state::delete_state(state_path);

                                // Resume session — spawn with PID tracking + await
                                let spawned = match claude::spawn_claude(
                                    project_dir,
                                    &reply,
                                    Some(&current_session_id),
                                    &session_name,
                                    args.max_turns,
                                    args.max_budget_usd,
                                ) {
                                    Ok(s) => s,
                                    Err(e) => {
                                        error!("failed to spawn resume: {e}");
                                        tg.notify(&format!(
                                            "⚠️ <b>Error spawning resume</b>  ·  <code>{pid}</code>\n{DIV}\n{err}",
                                            pid = escape_html(project_id),
                                            err = escape_html(&e.to_string()),
                                        )).await;
                                        return Ok(ExitCode::from(1));
                                    }
                                };

                                let resume_pgid = spawned.pgid;

                                // Write PID to state for orphan cleanup
                                let resume_running = AutorunState {
                                    phase: AutorunPhase::Running,
                                    session_id: Some(current_session_id.clone()),
                                    claude_pid: Some(spawned.pid),
                                    claude_pgid: Some(spawned.pgid),
                                    pending_question: None,
                                    telegram_message_id: None,
                                    telegram_update_offset: Some(tg.update_offset),
                                    invocation_count: n,
                                    updated_at: now_iso(),
                                    stats: stats.clone(),
                                };
                                let _ = state::write_state_atomic(state_path, &resume_running);

                                // Race await_claude against /kill command
                                let resume_query = build_query_response(project_id, stats, n);
                                let resume_result = tokio::select! {
                                    result = claude::await_claude(spawned, timeout) => result,
                                    _ = tg.poll_for_kill(project_id, &resume_query) => {
                                        claude::kill_process_group(resume_pgid).await;
                                        Err(RexError::Killed)
                                    }
                                };

                                let resume_output = match resume_result {
                                    Ok((output, _pgid)) => output,
                                    Err(ref e) if matches!(e, RexError::Killed) => {
                                        return Ok(handle_kill(tg, project_id, log_path, state_path).await);
                                    }
                                    Err(RexError::AuthExpired(_)) if !auth_refreshed => {
                                        warn!("claude auth expired during resume, attempting refresh");
                                        match attempt_auth_refresh(tg, project_id, project_dir, log_path).await {
                                            Ok(true) => {
                                                auth_refreshed = true;
                                                state::delete_state(state_path);
                                                break; // back to outer loop for fresh invocation
                                            }
                                            Ok(false) => {
                                                state::delete_state(state_path);
                                                return Ok(ExitCode::from(1));
                                            }
                                            Err(RexError::Killed) => {
                                                return Ok(handle_kill(tg, project_id, log_path, state_path).await);
                                            }
                                            Err(e) => {
                                                error!("auth refresh failed: {e}");
                                                state::delete_state(state_path);
                                                return Ok(ExitCode::from(1));
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let err_str = e.to_string();
                                        if is_retryable(&err_str) && consecutive_errors < args.max_retries {
                                            consecutive_errors += 1;
                                            let delay = retry_delay(consecutive_errors);
                                            tg.notify(&format!(
                                                "⚠️ <b>Error (resume)</b>  ·  <code>{pid}</code>\n{DIV}\n{err}\n\n🔄 Retrying in {delay}s (attempt {consecutive_errors}/{max})",
                                                pid = escape_html(project_id),
                                                err = escape_html(&err_str),
                                                max = args.max_retries,
                                            )).await;
                                            tokio::time::sleep(Duration::from_secs(delay)).await;
                                            state::delete_state(state_path);
                                            // Break to outer loop for fresh invocation
                                            break;
                                        }
                                        tg.notify(&format!(
                                            "⚠️ <b>Error (resume)</b>  ·  <code>{pid}</code>\n{DIV}\n{err}",
                                            pid = escape_html(project_id),
                                            err = escape_html(&err_str),
                                        )).await;
                                        state::delete_state(state_path);
                                        return Ok(ExitCode::from(1));
                                    }
                                };

                                stats.total_cost_usd += resume_output.effective_cost();
                                stats.invocations_completed += 1;
                                stats.push_context_percent(resume_output.context_percent());
                                stats.push_session_duration_ms(resume_output.duration_ms);

                                let resume_op = match claude::parse_operator_result(&resume_output.result) {
                                    Ok(r) => r,
                                    Err(e) => {
                                        error!("failed to parse resume result: {e}");
                                        tg.notify(&format!(
                                            "⚠️ <b>Error parsing resume</b>  ·  <code>{pid}</code>\n{DIV}\n{err}",
                                            pid = escape_html(project_id),
                                            err = escape_html(&e.to_string()),
                                        )).await;
                                        state::delete_state(state_path);
                                        return Ok(ExitCode::from(1));
                                    }
                                };

                                log_event(log_path, &LogEvent::InvocationCompleted {
                                    n,
                                    status: format!("{:?}", resume_op.status),
                                    message: resume_op.message.clone(),
                                    session_id: resume_output.session_id.clone(),
                                    cost_usd: resume_output.effective_cost(),
                                    duration_ms: resume_output.duration_ms,
                                    timestamp: now_iso(),
                                });

                                match resume_op.status {
                                    OperatorStatus::NeedsInput => {
                                        // Another question — stay in this inner loop
                                        info!("skill needs another input round");
                                        current_question = resume_op.message;
                                        current_session_id = resume_output.session_id;
                                        continue;
                                    }
                                    OperatorStatus::Completed => {
                                        stats.items_completed += 1;
                                        tg.notify_with_status_buttons(&format!(
                                            "✅ <b>Completed #{n}</b>  ·  <code>{pid}</code>{itag}\n\
                                             {DIV}\n\
                                             {msg}\n\
                                             {DIV}\n\
                                             {rstats}\n\
                                             💰 <code>${cost:.2}</code>  ·  ⏱ <code>{dur}</code>",
                                            pid = escape_html(project_id),
                                            itag = item_tag(&resume_op.item),
                                            msg = escape_html(&resume_op.message),
                                            rstats = resume_output.telegram_stats(),
                                            cost = resume_output.effective_cost(),
                                            dur = format_duration_ms(resume_output.duration_ms),
                                        ), project_id).await;
                                        state::delete_state(state_path);
                                        // Brief cooldown to avoid hammering the API and let the filesystem settle.
                                        info!(n, "invocation completed, cooling down 5s");
                                        tokio::time::sleep(Duration::from_secs(5)).await;
                                        break; // back to outer loop
                                    }
                                    OperatorStatus::ProjectDone => {
                                        let duration = format_duration_since(&stats.started_at);
                                        tg.notify(&format!(
                                            "🏁 <b>Project complete!</b>  ·  <code>{pid}</code>{itag}\n\
                                             {DIV}\n\
                                             {rstats}\n\
                                             📊 <code>{inv}</code> invocations  ·  💰 <code>${cost:.2}</code>  ·  ⏱ <code>{duration}</code>",
                                            pid = escape_html(project_id),
                                            itag = item_tag(&resume_op.item),
                                            rstats = resume_output.telegram_stats(),
                                            inv = stats.invocations_completed,
                                            cost = stats.total_cost_usd,
                                        )).await;
                                        log_event(log_path, &LogEvent::ProjectDone {
                                            total_cost_usd: stats.total_cost_usd,
                                            total_invocations: stats.invocations_completed,
                                            total_duration: duration,
                                            timestamp: now_iso(),
                                        });
                                        state::delete_state(state_path);
                                        return Ok(ExitCode::from(0));
                                    }
                                    OperatorStatus::Error => {
                                        error!("operator error after resume: {}", resume_op.message);
                                        tg.notify(&format!(
                                            "⚠️ <b>Error</b>  ·  <code>{pid}</code>{itag}\n\
                                             {DIV}\n\
                                             {rstats}\n\
                                             {DIV}\n\
                                             {msg}",
                                            pid = escape_html(project_id),
                                            itag = item_tag(&resume_op.item),
                                            rstats = resume_output.telegram_stats(),
                                            msg = escape_html(&resume_op.message),
                                        )).await;
                                        state::delete_state(state_path);
                                        return Ok(ExitCode::from(1));
                                    }
                                }
                            }
                            // Broke out of inner loop — continue outer loop
                            continue;
                        }
                        OperatorStatus::Error => {
                            error!("operator returned error: {}", result.message);

                            log_event(log_path, &LogEvent::Error {
                                message: result.message.clone(),
                                retryable: false,
                                attempt: consecutive_errors,
                                timestamp: now_iso(),
                            });

                            tg.notify(&format!(
                                "⚠️ <b>Error</b>  ·  <code>{pid}</code>{itag}\n\
                                 {DIV}\n\
                                 {stats}\n\
                                 {DIV}\n\
                                 {msg}",
                                pid = escape_html(project_id),
                                itag = item_tag(&result.item),
                                stats = output.telegram_stats(),
                                msg = escape_html(&result.message),
                            )).await;
                            state::delete_state(state_path);
                            return Ok(ExitCode::from(1));
                        }
                    },
                    Err(e) => {
                        // Failed to parse operator result
                        error!("failed to parse operator result: {e}");

                        log_event(log_path, &LogEvent::Error {
                            message: e.to_string(),
                            retryable: false,
                            attempt: consecutive_errors,
                            timestamp: now_iso(),
                        });

                        tg.notify(&format!(
                            "⚠️ <b>Parse error</b>  ·  <code>{pid}</code>\n\
                             {DIV}\n\
                             {stats}\n\
                             {DIV}\n\
                             {err}",
                            pid = escape_html(project_id),
                            stats = output.telegram_stats(),
                            err = escape_html(&e.to_string()),
                        )).await;
                        state::delete_state(state_path);
                        return Ok(ExitCode::from(1));
                    }
                }
            }
            Err(ref e) if matches!(e, RexError::Killed) => {
                return Ok(handle_kill(tg, project_id, log_path, state_path).await);
            }
            Err(RexError::AuthExpired(_)) if !auth_refreshed => {
                warn!("claude auth expired, attempting refresh");
                match attempt_auth_refresh(tg, project_id, project_dir, log_path).await {
                    Ok(true) => {
                        auth_refreshed = true;
                        state::delete_state(state_path);
                        continue;
                    }
                    Ok(false) => {
                        state::delete_state(state_path);
                        return Ok(ExitCode::from(1));
                    }
                    Err(RexError::Killed) => {
                        return Ok(handle_kill(tg, project_id, log_path, state_path).await);
                    }
                    Err(e) => {
                        error!("auth refresh failed: {e}");
                        tg.notify(&format!(
                            "⚠️ <b>Auth refresh failed</b>  ·  <code>{pid}</code>\n{DIV}\n{err}",
                            pid = escape_html(project_id),
                            err = escape_html(&e.to_string()),
                        )).await;
                        state::delete_state(state_path);
                        return Ok(ExitCode::from(1));
                    }
                }
            }
            Err(e) => {
                let err_str = e.to_string();
                consecutive_errors += 1;

                log_event(log_path, &LogEvent::Error {
                    message: err_str.clone(),
                    retryable: is_retryable(&err_str),
                    attempt: consecutive_errors,
                    timestamp: now_iso(),
                });

                if is_retryable(&err_str) && consecutive_errors <= args.max_retries {
                    let delay = retry_delay(consecutive_errors);
                    warn!(
                        attempt = consecutive_errors,
                        max = args.max_retries,
                        delay_secs = delay,
                        "retryable error, backing off"
                    );
                    tg.notify(&format!(
                        "⚠️ <b>Error</b>  ·  <code>{pid}</code>\n{DIV}\n{err}\n\n🔄 Retrying in {delay}s (attempt {consecutive_errors}/{max})",
                        pid = escape_html(project_id),
                        err = escape_html(&err_str),
                        max = args.max_retries,
                    ))
                    .await;
                    state::delete_state(state_path);
                    tokio::time::sleep(Duration::from_secs(delay)).await;
                    continue;
                }

                error!("non-retryable or max retries exhausted: {err_str}");
                tg.notify(&format!(
                    "🚨 <b>Fatal error</b>  ·  <code>{pid}</code>\n{DIV}\n{err}",
                    pid = escape_html(project_id),
                    err = escape_html(&err_str),
                )).await;
                state::delete_state(state_path);

                if consecutive_errors > args.max_retries {
                    return Ok(ExitCode::from(3));
                }
                return Ok(ExitCode::from(1));
            }
        }
    }
}

/// RAII guard that deregisters from the autorun registry on drop.
/// Ensures cleanup on all exit paths (including early returns and panics).
struct RegistryGuard<'a> {
    root_dir: &'a Path,
    project_id: &'a str,
}

impl Drop for RegistryGuard<'_> {
    fn drop(&mut self) {
        inbox::deregister_autorun(self.root_dir, self.project_id);
    }
}

/// Retry backoff: 30 * 2^(attempt-1) seconds, capped at 480.
fn retry_delay(attempt: u32) -> u64 {
    (30u64 * 2u64.pow(attempt.saturating_sub(1))).min(480)
}

/// Current time as ISO 8601.
fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

/// Append a JSONL event to the log file.
fn log_event(log_path: &Path, event: &LogEvent) {
    let Ok(line) = serde_json::to_string(event) else {
        return;
    };
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
    else {
        return;
    };
    let _ = writeln!(file, "{line}");
}

/// Format the optional item tag for Telegram messages.
/// Returns `"  ·  <code>{item}</code>"` when non-empty, or `""`.
fn item_tag(item: &str) -> String {
    if item.is_empty() {
        String::new()
    } else {
        format!("  ·  <code>{}</code>", escape_html(item))
    }
}

/// Pick a random acknowledgment response using subsecond nanos as entropy.
fn pick_ack_response() -> &'static str {
    let idx = Utc::now().timestamp_subsec_nanos() as usize % ACK_RESPONSES.len();
    ACK_RESPONSES[idx]
}

/// Send an acknowledgment message to confirm receipt of a reply.
async fn send_ack(tg: &TelegramClient, project_id: &str) {
    let ack = pick_ack_response();
    tg.notify(&format!(
        "👍 <b>Received</b>  ·  <code>{pid}</code>\n{ack}",
        pid = escape_html(project_id),
    ))
    .await;
}

/// Handle a /kill command — log, notify, clean up, return exit code 6.
async fn handle_kill(
    tg: &TelegramClient,
    project_id: &str,
    log_path: &Path,
    state_path: &Path,
) -> ExitCode {
    info!("killed by user via /kill command");
    log_event(log_path, &LogEvent::KilledByUser {
        project_id: project_id.to_string(),
        timestamp: now_iso(),
    });
    tg.notify(&format!(
        "🛑 <b>Killed</b>  ·  <code>{pid}</code>\n{DIV}\nStopped by /kill command",
        pid = escape_html(project_id),
    ))
    .await;
    state::delete_state(state_path);
    ExitCode::from(6)
}

/// Extract the first `https://` URL from text.
fn extract_url(text: &str) -> Option<String> {
    let start = text.find("https://")?;
    let rest = &text[start..];
    let end = rest
        .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == '>' || c == '<')
        .unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

/// Read stdout from the auth process looking for a URL (up to 15s).
async fn read_auth_url(child: &mut tokio::process::Child) -> Option<String> {
    use tokio::io::AsyncReadExt;

    let mut stdout = child.stdout.take()?;
    let mut buf = vec![0u8; 8192];
    let mut text = String::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);

    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_secs(3), stdout.read(&mut buf)).await {
            Ok(Ok(0)) => break,
            Ok(Ok(n)) => {
                text.push_str(&String::from_utf8_lossy(&buf[..n]));
                if let Some(url) = extract_url(&text) {
                    return Some(url);
                }
            }
            Ok(Err(_)) => break,
            Err(_) => continue,
        }
    }

    // Also try stderr as fallback
    if let Some(mut stderr) = child.stderr.take() {
        let mut ebuf = vec![0u8; 8192];
        if let Ok(Ok(n)) =
            tokio::time::timeout(Duration::from_secs(5), stderr.read(&mut ebuf)).await
        {
            if n > 0 {
                let etext = String::from_utf8_lossy(&ebuf[..n]);
                return extract_url(&etext);
            }
        }
    }

    None
}

/// Attempt to refresh Claude auth by running `claude auth login`.
///
/// Spawns the auth process, parses the URL from its output, sends it
/// to the user via Telegram, then waits for the user to confirm auth.
///
/// Returns `Ok(true)` if user confirmed, `Ok(false)` if timed out.
/// Returns `Err(RexError::Killed)` if user sent /kill.
async fn attempt_auth_refresh(
    tg: &mut TelegramClient,
    project_id: &str,
    project_dir: &Path,
    log_path: &Path,
) -> RexResult<bool> {
    info!("attempting auth refresh via `claude auth login`");

    // Spawn claude auth login
    let mut child = tokio::process::Command::new("claude")
        .arg("auth")
        .arg("login")
        .current_dir(project_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| {
            RexError::ClaudeProcess(format!("failed to spawn claude auth login: {e}"))
        })?;

    // Read output looking for an auth URL
    let auth_url = read_auth_url(&mut child).await;

    // Build message with URL or fallback to manual instruction
    let msg = match &auth_url {
        Some(url) => format!(
            "🔑 <b>Auth expired</b>  ·  <code>{pid}</code>\n\
             {DIV}\n\
             Your Claude token has expired.\n\n\
             Please visit this URL to re-authorize:\n{url}\n\n\
             <i>Reply when authorization is complete</i>",
            pid = escape_html(project_id),
        ),
        None => format!(
            "🔑 <b>Auth expired</b>  ·  <code>{pid}</code>\n\
             {DIV}\n\
             Your Claude token has expired.\n\n\
             Please run <code>claude auth login</code> on the server, then reply here when done.\n\n\
             <i>Reply when authorization is complete</i>",
            pid = escape_html(project_id),
        ),
    };

    let msg_id = tg.send_question(&msg).await?;
    inbox::update_expected_message_id(project_dir, project_id, Some(msg_id));

    log_event(
        log_path,
        &LogEvent::AuthRefresh {
            project_id: project_id.to_string(),
            timestamp: now_iso(),
        },
    );

    // Wait for user to confirm they've authorized (10 min timeout)
    let auth_timeout = Duration::from_secs(600);
    let auth_query = format!(
        "🔑 <b>Auth refresh</b>  ·  <code>{pid}</code>\n{DIV}\nWaiting for auth refresh — no stats available yet.",
        pid = escape_html(project_id),
    );
    let result = tg
        .wait_for_reply(msg_id, project_id, auth_timeout, &auth_query)
        .await;

    // Clean up auth process
    let _ = child.kill().await;
    let _ = child.wait().await;

    match result {
        Ok(TelegramPollResult::Reply(_)) => {
            send_ack(tg, project_id).await;
            info!("auth refresh confirmed by user");
            Ok(true)
        }
        Ok(TelegramPollResult::Kill) => Err(RexError::Killed),
        Err(e) => {
            warn!("auth refresh timed out: {e}");
            tg.notify(&format!(
                "⏰ <b>Auth timed out</b>  ·  <code>{pid}</code>\n{DIV}\nShutting down.",
                pid = escape_html(project_id),
            ))
            .await;
            Ok(false)
        }
    }
}

/// Build the response for a `/query` command.
///
/// Includes current project stats and scans for other running autoruns.
fn build_query_response(
    project_id: &str,
    stats: &RunStats,
    _invocation_count: u32,
) -> String {
    let uptime = format_duration_since(&stats.started_at);

    let ctx_last = stats
        .last_context_percent()
        .map(|p| format!("{p:.1}%"))
        .unwrap_or_else(|| "—".to_string());
    let ctx_avg = stats
        .avg_context_percent()
        .map(|p| format!("{p:.1}%"))
        .unwrap_or_else(|| "—".to_string());
    let dur_last = stats
        .last_session_duration_ms()
        .map(format_duration_ms)
        .unwrap_or_else(|| "—".to_string());
    let dur_avg = stats
        .avg_session_duration_ms()
        .map(format_duration_ms)
        .unwrap_or_else(|| "—".to_string());

    let mut msg = format!(
        "📊 <b>Autorun Status</b>  ·  <code>{pid}</code>\n\
         {DIV}\n\
         ⏱ <b>Total uptime:</b> <code>{uptime}</code>\n\
         📊 <b>Context:</b> <code>{ctx_last}</code> last  ·  <code>{ctx_avg}</code> avg\n\
         🕐 <b>Session:</b> <code>{dur_last}</code> last  ·  <code>{dur_avg}</code> avg\n\
         💰 <b>Cost:</b> <code>${cost:.2}</code>",
        pid = escape_html(project_id),
        cost = stats.total_cost_usd,
    );

    // Scan for other running autoruns via ProjectRegistry
    if let Ok(registry) = crate::models::project::ProjectRegistry::load() {
        let mut others = Vec::new();

        // Check all projects (active + inactive) for state files
        let all_projects = registry
            .active
            .iter()
            .chain(registry.inactive.iter())
            .filter(|p| p.id != project_id);

        for proj in all_projects {
            let state_file =
                std::path::Path::new(&proj.directory).join(".rex-autorun.json");
            if let Some(state) = super::state::read_state(&state_file) {
                let dur = format_duration_since(&state.stats.started_at);
                others.push(format!(
                    "  <b>{}</b>  ·  ⏱ {dur}  ·  🔄 #{} inv  ·  💰 ${:.2}",
                    escape_html(&proj.id),
                    state.invocation_count,
                    state.stats.total_cost_usd,
                ));
            }
        }

        if others.is_empty() {
            msg.push_str("\n\n<i>No other autoruns detected.</i>");
        } else {
            msg.push_str("\n\n<b>Other autoruns:</b>\n");
            msg.push_str(&others.join("\n"));
        }
    }

    msg
}

/// Format a duration in milliseconds as a human-readable string.
fn format_duration_ms(ms: u64) -> String {
    let secs = ms / 1000;
    let mins = secs / 60;
    let remaining_secs = secs % 60;
    if mins > 0 {
        format!("{mins}m {remaining_secs}s")
    } else {
        format!("{secs}s")
    }
}

/// Format a human-readable duration from an ISO 8601 start time to now.
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
