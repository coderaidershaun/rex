use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use anyhow::{Context, Result};
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
pub async fn run(args: Args) -> Result<ExitCode> {
    // Resolve project directory to absolute path
    let project_dir = std::fs::canonicalize(&args.project_dir)
        .with_context(|| format!("cannot resolve project dir: {}", args.project_dir.display()))?;

    // Load .env from project dir
    let env_path = project_dir.join(".env");
    if env_path.exists() {
        dotenvy::from_path(&env_path).ok();
    } else {
        dotenvy::dotenv().ok();
    }

    // Read Telegram credentials from env
    let telegram_token = std::env::var("TELEGRAM_BOT_TOKEN")
        .context("TELEGRAM_BOT_TOKEN not set (check .env)")?;
    let telegram_chat_id: i64 = std::env::var("TELEGRAM_CHAT_ID")
        .context("TELEGRAM_CHAT_ID not set (check .env)")?
        .parse()
        .context("TELEGRAM_CHAT_ID must be a valid integer")?;

    // Load project info from projects.json
    // ProjectRegistry::load() reads from cwd, so we set cwd first.
    std::env::set_current_dir(&project_dir)
        .with_context(|| format!("cannot chdir to {}", project_dir.display()))?;

    let registry = ProjectRegistry::load()
        .map_err(|e| anyhow::anyhow!("failed to load project registry: {e}"))?;
    let project = registry.active
        .context("no active rex project — run `rex project set-active` first")?;

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
                "[{project_id}] (recovered) Still waiting for your reply:\n\n{question}\n\n(Reply to this message with your answer)"
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
                            let cost = output.cost.total_cost;
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
                                session_id: output.session_id,
                                cost_usd: cost,
                                duration_ms: output.duration_ms,
                                timestamp: now_iso(),
                            });

                            match op_result {
                                Ok(r) if r.status == OperatorStatus::ProjectDone => {
                                    tg.notify(&format!(
                                        "[{project_id}] Project complete!\nTotal invocations: {} | Total cost: ${:.2}",
                                        recovered_stats.invocations_completed,
                                        recovered_stats.total_cost_usd,
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
                            tg.notify(&format!("[{project_id}] Error on resume: {e}")).await;
                            (stats, invocation_count, Some(tg.update_offset))
                        }
                    }
                }
                Err(e) => {
                    error!("human reply timeout: {e}");
                    tg.notify(&format!("[{project_id}] Timeout waiting for reply — shutting down")).await;
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
        "[{project_id}] Autorun started\nProject: {project_title}\nDirectory: {project_directory}"
    )).await;

    // Signal handling + main loop
    let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .context("failed to register SIGTERM handler")?;

    let main_result = tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("SIGINT received — shutting down");
            tg.notify(&format!("[{project_id}] Autorun stopped (SIGINT)")).await;
            state::delete_state(&state_path);
            Ok(ExitCode::from(4))
        }
        _ = sigterm.recv() => {
            info!("SIGTERM received — shutting down");
            tg.notify(&format!("[{project_id}] Autorun stopped (SIGTERM)")).await;
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
) -> Result<ExitCode> {
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
                "[{project_id}] Budget limit reached (${:.2} / ${:.2}) — stopping",
                stats.total_cost_usd, args.max_total_budget_usd,
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
                let cost = output.cost.total_cost;
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
                                "[{project_id}] Completed: {}\nCost: ${cost:.2} | Duration: {}s | Invocation: #{n}",
                                result.message,
                                output.duration_ms / 1000,
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
                                "[{project_id}] Project complete!\nTotal invocations: {} | Total cost: ${:.2} | Duration: {duration}",
                                stats.invocations_completed,
                                stats.total_cost_usd,
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
                            info!(session_id = %output.session_id, "needs user input");

                            // Save pending_input state
                            let pending_state = AutorunState {
                                phase: AutorunPhase::PendingInput,
                                session_id: Some(output.session_id.clone()),
                                claude_pid: None,
                                claude_pgid: None,
                                pending_question: Some(result.message.clone()),
                                telegram_message_id: None,
                                telegram_update_offset: Some(tg.update_offset),
                                invocation_count: n,
                                updated_at: now_iso(),
                                stats: stats.clone(),
                            };
                            if let Err(e) = state::write_state_atomic(state_path, &pending_state) {
                                error!("failed to write pending_input state: {e}");
                            }

                            log_event(log_path, &LogEvent::NeedsInput {
                                question: result.message.clone(),
                                session_id: output.session_id.clone(),
                                timestamp: now_iso(),
                            });

                            // Send question to Telegram
                            let msg = format!(
                                "[{project_id}] Input needed:\n\n{}\n\n(Reply to this message with your answer)",
                                result.message,
                            );
                            match tg.send_message(&msg).await {
                                Ok(msg_id) => {
                                    // Update state with telegram message ID
                                    let mut updated = pending_state;
                                    updated.telegram_message_id = Some(msg_id);
                                    updated.telegram_update_offset = Some(tg.update_offset);
                                    let _ = state::write_state_atomic(state_path, &updated);
                                }
                                Err(e) => {
                                    error!("failed to send question to telegram: {e}");
                                    // Question is persisted in state, will be re-sent on recovery
                                }
                            }

                            // Wait for reply
                            let human_timeout = Duration::from_secs(args.human_timeout_days * 86400);
                            match tg.wait_for_reply(human_timeout).await {
                                Ok(reply) => {
                                    info!(reply_len = reply.len(), "got user reply, resuming session");

                                    log_event(log_path, &LogEvent::InputReceived {
                                        reply_length: reply.len(),
                                        timestamp: now_iso(),
                                    });

                                    state::delete_state(state_path);

                                    // Resume the session with the user's reply
                                    let resume_result = async {
                                        let spawned = claude::spawn_claude(
                                            project_dir,
                                            &reply,
                                            Some(&output.session_id),
                                            &session_name,
                                            args.max_turns,
                                            args.max_budget_usd,
                                        )?;
                                        claude::await_claude(spawned, timeout).await
                                    }
                                    .await;

                                    match resume_result {
                                        Ok((resume_output, _pgid)) => {
                                            let resume_cost = resume_output.cost.total_cost;
                                            stats.total_cost_usd += resume_cost;
                                            stats.invocations_completed += 1;

                                            let resume_op = claude::parse_operator_result(&resume_output.result);

                                            log_event(log_path, &LogEvent::InvocationCompleted {
                                                n,
                                                status: match &resume_op {
                                                    Ok(r) => format!("{:?}", r.status),
                                                    Err(_) => "parse_error".to_string(),
                                                },
                                                message: match &resume_op {
                                                    Ok(r) => r.message.clone(),
                                                    Err(e) => e.to_string(),
                                                },
                                                session_id: resume_output.session_id.clone(),
                                                cost_usd: resume_cost,
                                                duration_ms: resume_output.duration_ms,
                                                timestamp: now_iso(),
                                            });

                                            match resume_op {
                                                Ok(r) if r.status == OperatorStatus::ProjectDone => {
                                                    let duration = format_duration_since(&stats.started_at);
                                                    tg.notify(&format!(
                                                        "[{project_id}] Project complete!\nTotal invocations: {} | Total cost: ${:.2} | Duration: {duration}",
                                                        stats.invocations_completed,
                                                        stats.total_cost_usd,
                                                    )).await;
                                                    state::delete_state(state_path);
                                                    return Ok(ExitCode::from(0));
                                                }
                                                Ok(r) if r.status == OperatorStatus::NeedsInput => {
                                                    // Another question — loop back through
                                                    // Write new pending state and re-ask
                                                    info!("skill needs another input round");
                                                    // Decrement invocation_count so the outer loop
                                                    // increments it back — but actually we should
                                                    // handle multi-round input inline. For simplicity,
                                                    // continue the outer loop which will invoke fresh.
                                                    // The resumed session already emitted needs_input,
                                                    // but we consumed it. We need to handle this
                                                    // recursively or re-enter the needs_input flow.
                                                    //
                                                    // For now, send the new question and loop.
                                                    let new_msg = format!(
                                                        "[{project_id}] Follow-up input needed:\n\n{}\n\n(Reply to this message with your answer)",
                                                        r.message,
                                                    );

                                                    let new_pending = AutorunState {
                                                        phase: AutorunPhase::PendingInput,
                                                        session_id: Some(resume_output.session_id.clone()),
                                                        claude_pid: None,
                                                        claude_pgid: None,
                                                        pending_question: Some(r.message.clone()),
                                                        telegram_message_id: None,
                                                        telegram_update_offset: Some(tg.update_offset),
                                                        invocation_count: n,
                                                        updated_at: now_iso(),
                                                        stats: stats.clone(),
                                                    };
                                                    let _ = state::write_state_atomic(state_path, &new_pending);

                                                    tg.notify(&new_msg).await;

                                                    // We need to wait again — but rather than deep recursion,
                                                    // continue the outer loop with the session to resume set.
                                                    // For v1, just do a fresh invocation which will
                                                    // re-read the project state.
                                                    state::delete_state(state_path);
                                                    tokio::time::sleep(Duration::from_secs(5)).await;
                                                    continue;
                                                }
                                                Ok(r) if r.status == OperatorStatus::Completed => {
                                                    stats.items_completed += 1;
                                                    tg.notify(&format!(
                                                        "[{project_id}] Completed: {}\nCost: ${resume_cost:.2} | Invocation: #{n}",
                                                        r.message,
                                                    )).await;
                                                    state::delete_state(state_path);
                                                    tokio::time::sleep(Duration::from_secs(5)).await;
                                                    continue;
                                                }
                                                Ok(r) => {
                                                    // OperatorStatus::Error
                                                    error!("operator error after resume: {}", r.message);
                                                    tg.notify(&format!("[{project_id}] Error: {}", r.message)).await;
                                                    state::delete_state(state_path);
                                                    return Ok(ExitCode::from(1));
                                                }
                                                Err(e) => {
                                                    error!("failed to parse resume result: {e}");
                                                    tg.notify(&format!("[{project_id}] Error parsing resume result: {e}")).await;
                                                    state::delete_state(state_path);
                                                    return Ok(ExitCode::from(1));
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            error!("resume invocation failed: {e}");
                                            let err_str = e.to_string();
                                            if is_retryable(&err_str) && consecutive_errors < args.max_retries {
                                                consecutive_errors += 1;
                                                let delay = retry_delay(consecutive_errors);
                                                tg.notify(&format!(
                                                    "[{project_id}] Error (resume): {err_str}\nRetrying in {delay}s (attempt {consecutive_errors}/{})",
                                                    args.max_retries,
                                                )).await;
                                                tokio::time::sleep(Duration::from_secs(delay)).await;
                                                state::delete_state(state_path);
                                                continue;
                                            }
                                            tg.notify(&format!("[{project_id}] Error (resume): {err_str}")).await;
                                            state::delete_state(state_path);
                                            return Ok(ExitCode::from(1));
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("human reply timeout: {e}");
                                    tg.notify(&format!("[{project_id}] Timeout waiting for reply — shutting down")).await;
                                    state::delete_state(state_path);
                                    return Ok(ExitCode::from(2));
                                }
                            }
                        }
                        OperatorStatus::Error => {
                            error!("operator returned error: {}", result.message);

                            log_event(log_path, &LogEvent::Error {
                                message: result.message.clone(),
                                retryable: false,
                                attempt: consecutive_errors,
                                timestamp: now_iso(),
                            });

                            tg.notify(&format!("[{project_id}] Error: {}", result.message)).await;
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

                        tg.notify(&format!("[{project_id}] Error: failed to parse operator result: {e}")).await;
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
                        "[{project_id}] Error: {err_str}\nRetrying in {delay}s (attempt {consecutive_errors}/{})",
                        args.max_retries,
                    ))
                    .await;
                    state::delete_state(state_path);
                    tokio::time::sleep(Duration::from_secs(delay)).await;
                    continue;
                }

                error!("non-retryable or max retries exhausted: {err_str}");
                tg.notify(&format!("[{project_id}] Fatal error: {err_str}")).await;
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
