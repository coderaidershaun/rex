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
use super::state::{self, RecoveryAction};
use super::telegram::TelegramClient;
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
    #[arg(long, default_value = "7")]
    pub human_timeout_days: u64,

    /// Log file path (default: <project-dir>/.rex-autorun.log)
    #[arg(long)]
    pub log_file: Option<PathBuf>,
}

/// Main entry point — the core state machine.
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
    let telegram_token = std::env::var("TELEGRAM_BOT_TOKEN")
        .map_err(|_| RexError::EnvVar { name: "TELEGRAM_BOT_TOKEN".into(), detail: "check .env".into() })?;
    let telegram_chat_id: i64 = std::env::var("TELEGRAM_CHAT_ID")
        .map_err(|_| RexError::EnvVar { name: "TELEGRAM_CHAT_ID".into(), detail: "check .env".into() })?
        .parse()
        .map_err(|e| RexError::EnvVar { name: "TELEGRAM_CHAT_ID".into(), detail: format!("must be a valid integer: {e}") })?;

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
            );

            let msg = format!(
                "<b>[{pid}] Awaiting reply (recovered)</b>\n━━━━━━━━━━━━━━━━━━━━\n{q}\n\n<i>Reply to this message with your answer</i>",
                pid = escape_html(&project_id),
                q = escape_html(&question),
            );
            tg.notify(&msg).await;

            // Wait for reply
            let human_timeout = Duration::from_secs(args.human_timeout_days * 86400);
            match tg.wait_for_reply(human_timeout).await {
                Ok(reply) => {
                    log_event(&log_path, &LogEvent::InputReceived {
                        reply_length: reply.len(),
                        timestamp: now_iso(),
                    });

                    state::delete_state(&state_path);

                    // Resume the claude session with the user's reply
                    let timeout = Duration::from_secs(args.process_timeout_mins * 60);
                    let session_name = format!("rex-autorun-{project_id}-{invocation_count}");

                    let spawn_result = claude::spawn_claude(
                        &project_dir,
                        &reply,
                        Some(&session_id),
                        &session_name,
                        args.max_turns,
                        args.max_budget_usd,
                    );

                    match async {
                        let spawned = spawn_result?;
                        claude::await_claude(spawned, timeout).await
                    }.await
                    {
                        Ok((output, _pgid)) => {
                            let cost = output.effective_cost();
                            let mut recovered_stats = stats;
                            recovered_stats.total_cost_usd += cost;
                            recovered_stats.invocations_completed += 1;

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
                                        "{header}\n━━━━━━━━━━━━━━━━━━━━\n<b>[{pid}] Project complete!</b>\nInvocations: {inv}  |  Cost: ${cost:.2}",
                                        header = output.telegram_header(),
                                        pid = escape_html(&project_id),
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
                        Err(e) => {
                            error!("resume invocation failed: {e}");
                            tg.notify(&format!(
                                "<b>[{pid}] Error on resume</b>\n{err}",
                                pid = escape_html(&project_id),
                                err = escape_html(&e.to_string()),
                            )).await;
                            (stats, invocation_count, Some(tg.update_offset))
                        }
                    }
                }
                Err(e) => {
                    error!("human reply timeout: {e}");
                    tg.notify(&format!(
                        "<b>[{pid}] Timeout waiting for reply</b>\nShutting down",
                        pid = escape_html(&project_id),
                    )).await;
                    state::delete_state(&state_path);
                    return Ok(ExitCode::from(2));
                }
            }
        }
    };

    // Create Telegram client for main loop
    let mut tg = TelegramClient::new(telegram_token, telegram_chat_id, telegram_offset);

    // Log + notify start
    log_event(&log_path, &LogEvent::Started {
        project_id: project_id.clone(),
        timestamp: now_iso(),
    });
    tg.notify(&format!(
        "<b>[{pid}] Autorun started</b>\n<b>Project:</b> {pt}\n<b>Directory:</b> {pd}",
        pid = escape_html(&project_id),
        pt = escape_html(&project_title),
        pd = escape_html(&project_directory),
    )).await;

    // Signal handling + main loop
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;

    let main_result = tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("SIGINT received — shutting down");
            tg.notify(&format!("<b>[{pid}] Autorun stopped</b> (SIGINT)", pid = escape_html(&project_id))).await;
            state::delete_state(&state_path);
            Ok(ExitCode::from(4))
        }
        _ = sigterm.recv() => {
            info!("SIGTERM received — shutting down");
            tg.notify(&format!("<b>[{pid}] Autorun stopped</b> (SIGTERM)", pid = escape_html(&project_id))).await;
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

    main_result
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

    loop {
        // Budget check
        if stats.total_cost_usd >= args.max_total_budget_usd {
            warn!(
                total = stats.total_cost_usd,
                limit = args.max_total_budget_usd,
                "total budget exceeded"
            );
            tg.notify(&format!(
                "<b>[{pid}] Budget limit reached</b>\n${used:.2} / ${limit:.2} — stopping",
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
        let invoke_result = async {
            let spawned = claude::spawn_claude(
                project_dir,
                "/rex-operator",
                None,
                &session_name,
                args.max_turns,
                args.max_budget_usd,
            )?;

            // Update state with PID/PGID so crash recovery can kill orphans
            let mut with_pid = running_state.clone();
            with_pid.claude_pid = Some(spawned.pid);
            with_pid.claude_pgid = Some(spawned.pgid);
            let _ = state::write_state_atomic(state_path, &with_pid);

            claude::await_claude(spawned, timeout).await
        }
        .await;

        match invoke_result {
            Ok((output, _pgid)) => {
                consecutive_errors = 0;
                let cost = output.effective_cost();
                stats.total_cost_usd += cost;
                stats.invocations_completed += 1;

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
                            tg.notify(&format!(
                                "{header}\n━━━━━━━━━━━━━━━━━━━━\n<b>[{pid}] Completed #{n}</b>\n{msg}\n\n<b>Cost:</b> ${cost:.2}  |  <b>Duration:</b> {dur}s",
                                header = output.telegram_header(),
                                pid = escape_html(project_id),
                                msg = escape_html(&result.message),
                                cost = cost,
                                dur = output.duration_ms / 1000,
                            ))
                            .await;

                            state::delete_state(state_path);
                            info!(n, "invocation completed, cooling down 5s");
                            tokio::time::sleep(Duration::from_secs(5)).await;
                            continue;
                        }
                        OperatorStatus::ProjectDone => {
                            let duration = format_duration_since(&stats.started_at);
                            tg.notify(&format!(
                                "{header}\n━━━━━━━━━━━━━━━━━━━━\n<b>[{pid}] Project complete!</b>\nInvocations: {inv}  |  Cost: ${cost:.2}  |  Duration: {duration}",
                                header = output.telegram_header(),
                                pid = escape_html(project_id),
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

                                // Send question to Telegram
                                let msg = format!(
                                    "{header}\n━━━━━━━━━━━━━━━━━━━━\n<b>[{pid}] Input needed</b>\n\n{q}\n\n<i>Reply to this message with your answer</i>",
                                    header = output.telegram_header(),
                                    pid = escape_html(project_id),
                                    q = escape_html(&current_question),
                                );
                                if let Ok(msg_id) = tg.send_message(&msg).await {
                                    let mut updated = pending_state;
                                    updated.telegram_message_id = Some(msg_id);
                                    updated.telegram_update_offset = Some(tg.update_offset);
                                    let _ = state::write_state_atomic(state_path, &updated);
                                }

                                // Wait for reply
                                let human_timeout = Duration::from_secs(args.human_timeout_days * 86400);
                                let reply = match tg.wait_for_reply(human_timeout).await {
                                    Ok(r) => r,
                                    Err(e) => {
                                        error!("human reply timeout: {e}");
                                        tg.notify(&format!(
                                            "<b>[{pid}] Timeout waiting for reply</b>\nShutting down",
                                            pid = escape_html(project_id),
                                        )).await;
                                        state::delete_state(state_path);
                                        return Ok(ExitCode::from(2));
                                    }
                                };

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
                                            "<b>[{pid}] Error spawning resume</b>\n{err}",
                                            pid = escape_html(project_id),
                                            err = escape_html(&e.to_string()),
                                        )).await;
                                        return Ok(ExitCode::from(1));
                                    }
                                };

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

                                let resume_output = match claude::await_claude(spawned, timeout).await {
                                    Ok((output, _pgid)) => output,
                                    Err(e) => {
                                        let err_str = e.to_string();
                                        if is_retryable(&err_str) && consecutive_errors < args.max_retries {
                                            consecutive_errors += 1;
                                            let delay = retry_delay(consecutive_errors);
                                            tg.notify(&format!(
                                                "<b>[{pid}] Error (resume)</b>\n{err}\n\nRetrying in {delay}s (attempt {consecutive_errors}/{max})",
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
                                            "<b>[{pid}] Error (resume)</b>\n{err}",
                                            pid = escape_html(project_id),
                                            err = escape_html(&err_str),
                                        )).await;
                                        state::delete_state(state_path);
                                        return Ok(ExitCode::from(1));
                                    }
                                };

                                stats.total_cost_usd += resume_output.effective_cost();
                                stats.invocations_completed += 1;

                                let resume_op = match claude::parse_operator_result(&resume_output.result) {
                                    Ok(r) => r,
                                    Err(e) => {
                                        error!("failed to parse resume result: {e}");
                                        tg.notify(&format!(
                                            "<b>[{pid}] Error parsing resume</b>\n{err}",
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
                                        tg.notify(&format!(
                                            "{header}\n━━━━━━━━━━━━━━━━━━━━\n<b>[{pid}] Completed #{n}</b>\n{msg}\n\n<b>Cost:</b> ${cost:.2}  |  <b>Duration:</b> {dur}s",
                                            header = resume_output.telegram_header(),
                                            pid = escape_html(project_id),
                                            msg = escape_html(&resume_op.message),
                                            cost = resume_output.effective_cost(),
                                            dur = resume_output.duration_ms / 1000,
                                        )).await;
                                        state::delete_state(state_path);
                                        info!(n, "invocation completed, cooling down 5s");
                                        tokio::time::sleep(Duration::from_secs(5)).await;
                                        break; // back to outer loop
                                    }
                                    OperatorStatus::ProjectDone => {
                                        let duration = format_duration_since(&stats.started_at);
                                        tg.notify(&format!(
                                            "{header}\n━━━━━━━━━━━━━━━━━━━━\n<b>[{pid}] Project complete!</b>\nInvocations: {inv}  |  Cost: ${cost:.2}  |  Duration: {duration}",
                                            header = resume_output.telegram_header(),
                                            pid = escape_html(project_id),
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
                                            "{header}\n━━━━━━━━━━━━━━━━━━━━\n<b>[{pid}] Error</b>\n{msg}",
                                            header = resume_output.telegram_header(),
                                            pid = escape_html(project_id),
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
                                "{header}\n━━━━━━━━━━━━━━━━━━━━\n<b>[{pid}] Error</b>\n{msg}",
                                header = output.telegram_header(),
                                pid = escape_html(project_id),
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
                            "{header}\n━━━━━━━━━━━━━━━━━━━━\n<b>[{pid}] Parse error</b>\n{err}",
                            header = output.telegram_header(),
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
                        "<b>[{pid}] Error</b>\n{err}\n\nRetrying in {delay}s (attempt {consecutive_errors}/{max})",
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
                    "<b>[{pid}] Fatal error</b>\n{err}",
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
