# Rex Operator Autopilot Harness — Design Document

## Executive Summary

This document describes a Rust-based autopilot harness that runs Claude Code headlessly via its CLI, orchestrates multi-project autonomous coding sessions, and brings a human into the loop via Telegram only when Claude explicitly requests input. The system is designed to run unattended for days or weeks, surviving transient failures in both the Claude CLI and the Telegram API.

The architecture splits into two binaries: a **broker** (singleton, owns the Telegram bot) and one or more **operators** (one per project, each driving a Claude Code session). Operators communicate with the broker over local IPC. The broker multiplexes Telegram messages to the correct operator using inline keyboards and callback routing.

---

## User Goals and Needs

### Primary Goal
Run a Claude Code custom skill (`/rex-operator`) autonomously across multiple projects. The skill performs a unit of work and exits with a structured status. The harness reacts to that status — either resuming the session, asking the user a question, or terminating.

### Constraints
- **One Telegram account** shared across all projects. Telegram's `getUpdates` API is exclusive — only one process can poll it.
- **Multiple project binaries** running concurrently, each with its own Claude Code session.
- **Long-lived sessions** — the user may take up to a week to respond. The system must survive process restarts during that window.
- **Context window management is handled internally** by the `/rex-operator` skill itself. The harness does not need to track chunk counts or recycle sessions.

### What the User Sees
Every Telegram message is tagged with a human-friendly project name. When Claude needs input, the user sees an inline button labelled with the project. Tapping it activates that project's reply context, and a `ForceReply` ensures the next message is captured cleanly. A `/project <id> <message>` command exists as a manual override. Completion and error notifications are also tagged.

```
[alpha] Need your AWS credentials for the staging deploy.
        [Reply to alpha]

[beta] Should I use PostgreSQL or SQLite for the cache layer?
        [Reply to beta]

[alpha] Done: Deployed v2.3.1 to staging.
```

---

## Architecture

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ rex-operator │     │ rex-operator │     │ rex-operator │
│  project-A   │     │  project-B   │     │  project-C   │
└──────┬───────┘     └──────┬───────┘     └──────┬───────┘
       │ tcp/unix           │                    │
       └────────────┬───────┴────────────────────┘
                    │
             ┌──────┴──────┐
             │  rex-broker  │
             │  (single TG  │
             │   listener)  │
             └──────┬───────┘
                    │
                 Telegram
```

- **rex-broker**: Single process. Owns the Telegram bot token. Runs the `getUpdates` loop. Routes replies to the correct operator via project ID.
- **rex-operator**: One instance per project. Shells out to `claude -p` / `claude --resume`. Communicates with the broker over local IPC (unix socket or TCP). Contains zero Telegram code.

---

## IPC Protocol

Two message types flow between operators and the broker:

```rust
/// Operator -> Broker
#[derive(serde::Serialize, serde::Deserialize)]
enum ToBroker {
    Ask {
        project_id: String,
        project_name: String,
        question: String,
    },
    Notify {
        project_id: String,
        project_name: String,
        message: String,
    },
}

/// Broker -> Operator
#[derive(serde::Serialize, serde::Deserialize)]
enum ToOperator {
    Reply { text: String },
}
```

---

## Rex Operator Skill Contract

The `/rex-operator` skill must emit a structured JSON status as its result text. This is the only interface between Claude's output and the harness logic.

```rust
#[derive(Debug, serde::Deserialize)]
struct OperatorResult {
    status: Status,
    #[serde(default)]
    message: String,
}

#[derive(Debug, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Status {
    Completed,    // work is done
    NeedsInput,   // blocked on human — message contains the question
    Error,        // unrecoverable
}
```

There is no `Continue` or `InProgress` state. `claude -p` runs the full agent loop internally and only exits when it reaches one of these three terminal states.

---

## Recommended Code

### Operator Binary

```rust
use std::path::PathBuf;
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::mpsc;

const HUMAN_TIMEOUT: Duration = Duration::from_secs(7 * 24 * 60 * 60); // 1 week

// ── Skill contract ──────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
struct OperatorResult {
    status: Status,
    #[serde(default)]
    message: String,
}

#[derive(Debug, serde::Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Status {
    Completed,
    NeedsInput,
    Error,
}

// ── Claude invocation output ────────────────────────────────────

struct InvokeOutput {
    session_id: String,
    result_text: String,
}

// ── Broker client ───────────────────────────────────────────────

struct BrokerClient {
    project_id: String,
    project_name: String,
    tx: mpsc::Sender<ToBroker>,
    rx: mpsc::Receiver<ToOperator>,
}

#[derive(serde::Serialize, serde::Deserialize)]
enum ToBroker {
    Ask {
        project_id: String,
        project_name: String,
        question: String,
    },
    Notify {
        project_id: String,
        project_name: String,
        message: String,
    },
}

#[derive(serde::Serialize, serde::Deserialize)]
enum ToOperator {
    Reply { text: String },
}

impl BrokerClient {
    async fn ask(&mut self, question: &str) -> anyhow::Result<String> {
        self.tx
            .send(ToBroker::Ask {
                project_id: self.project_id.clone(),
                project_name: self.project_name.clone(),
                question: question.to_string(),
            })
            .await?;

        let reply = tokio::time::timeout(HUMAN_TIMEOUT, self.rx.recv())
            .await
            .map_err(|_| anyhow::anyhow!("no response in 1 week"))?
            .ok_or_else(|| anyhow::anyhow!("broker disconnected"))?;

        match reply {
            ToOperator::Reply { text } => Ok(text),
        }
    }

    async fn notify(&self, message: &str) -> anyhow::Result<()> {
        self.tx
            .send(ToBroker::Notify {
                project_id: self.project_id.clone(),
                project_name: self.project_name.clone(),
                message: message.to_string(),
            })
            .await?;
        Ok(())
    }
}

// ── Operator ────────────────────────────────────────────────────

struct RexOperator {
    project_dir: PathBuf,
    broker: BrokerClient,
    session_id: Option<String>,
    process_timeout: Duration,
    max_retries: usize,
}

impl RexOperator {
    async fn run(&mut self) -> anyhow::Result<()> {
        // First invocation — start a new session
        let out = self.invoke_with_retry("/rex-operator").await?;
        self.session_id = Some(out.session_id);

        loop {
            let result = self.parse_result(&out.result_text)?;

            match result.status {
                Status::Completed => {
                    self.broker
                        .notify(&format!("Done: {}", result.message))
                        .await?;
                    return Ok(());
                }

                Status::NeedsInput => {
                    let reply = self.broker.ask(&result.message).await?;
                    let out = self.invoke_with_retry(&reply).await?;
                    self.session_id = Some(out.session_id);
                    continue;
                }

                Status::Error => {
                    self.broker
                        .notify(&format!("Error: {}", result.message))
                        .await?;
                    return Err(anyhow::anyhow!("{}", result.message));
                }
            }
        }
    }

    async fn invoke_with_retry(&self, prompt: &str) -> anyhow::Result<InvokeOutput> {
        let mut attempt = 0;
        loop {
            match self.invoke_claude(prompt).await {
                Ok(output) => return Ok(output),
                Err(e) => {
                    attempt += 1;
                    if attempt > self.max_retries || !is_retryable(&e) {
                        return Err(e);
                    }
                    let backoff = Duration::from_secs(30 * (1 << (attempt - 1)));
                    tokio::time::sleep(backoff).await;
                }
            }
        }
    }

    async fn invoke_claude(&self, prompt: &str) -> anyhow::Result<InvokeOutput> {
        let mut cmd = Command::new("claude");
        cmd.current_dir(&self.project_dir);
        cmd.kill_on_drop(true);

        match &self.session_id {
            Some(sid) => {
                cmd.args([
                    "--resume", sid, "-p", prompt,
                    "--output-format", "json",
                ]);
            }
            None => {
                cmd.args(["-p", prompt, "--output-format", "json"]);
            }
        }

        let output = tokio::time::timeout(self.process_timeout, cmd.output())
            .await
            .map_err(|_| {
                anyhow::anyhow!("claude timed out after {:?}", self.process_timeout)
            })??;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("claude exited {}: {stderr}", output.status);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let envelope: serde_json::Value = serde_json::from_str(&stdout)?;

        let session_id = envelope["session_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("no session_id in response"))?
            .to_string();

        let result_text = envelope["result"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(InvokeOutput {
            session_id,
            result_text,
        })
    }

    fn parse_result(&self, result_text: &str) -> anyhow::Result<OperatorResult> {
        let cleaned = result_text
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        serde_json::from_str::<OperatorResult>(cleaned).map_err(|e| {
            anyhow::anyhow!("failed to parse result: {e}\nraw: {result_text}")
        })
    }
}

fn is_retryable(e: &anyhow::Error) -> bool {
    let msg = e.to_string().to_lowercase();
    msg.contains("rate limit")
        || msg.contains("overloaded")
        || msg.contains("connection")
        || msg.contains("timeout")
}
```

### Broker Binary

```rust
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use teloxide::prelude::*;
use teloxide::types::{
    ChatId, ForceReply,
    InlineKeyboardButton, InlineKeyboardMarkup,
    ParseMode,
};

struct BrokerState {
    /// project_id -> sender back to that operator's task
    operators: HashMap<String, mpsc::Sender<String>>,
    /// human-friendly names for display
    project_names: HashMap<String, String>,
    /// which project the user is currently replying to
    active_reply: Option<String>,
}

impl BrokerState {
    fn display_name(&self, project_id: &str) -> &str {
        self.project_names
            .get(project_id)
            .map(|s| s.as_str())
            .unwrap_or(project_id)
    }
}

async fn handle_ask(
    bot: &Bot,
    chat_id: ChatId,
    project_id: &str,
    project_name: &str,
    question: &str,
) -> anyhow::Result<()> {
    let button = InlineKeyboardButton::callback(
        format!("Reply to {project_name}"),
        format!("reply:{project_id}"),
    );
    let keyboard = InlineKeyboardMarkup::new(vec![vec![button]]);

    let mut attempt = 0;
    loop {
        match bot
            .send_message(chat_id, format!("[{project_name}]\n\n{question}"))
            .reply_markup(keyboard.clone())
            .await
        {
            Ok(_) => return Ok(()),
            Err(e) => {
                attempt += 1;
                if attempt > 5 {
                    return Err(anyhow::anyhow!("telegram send failed: {e}"));
                }
                tokio::time::sleep(Duration::from_secs(10 * attempt)).await;
            }
        }
    }
}

async fn handle_notify(
    bot: &Bot,
    chat_id: ChatId,
    project_name: &str,
    message: &str,
) -> anyhow::Result<()> {
    let mut attempt = 0;
    loop {
        match bot
            .send_message(chat_id, format!("[{project_name}] {message}"))
            .await
        {
            Ok(_) => return Ok(()),
            Err(e) => {
                attempt += 1;
                if attempt > 5 {
                    return Err(anyhow::anyhow!("telegram send failed: {e}"));
                }
                tokio::time::sleep(Duration::from_secs(10 * attempt)).await;
            }
        }
    }
}

async fn broker_loop(bot: Bot, chat_id: ChatId, state: Arc<Mutex<BrokerState>>) {
    let mut offset: i32 = 0;
    let mut backoff = Duration::from_secs(1);

    loop {
        let updates = match bot.get_updates().offset(offset).timeout(30).await {
            Ok(u) => {
                backoff = Duration::from_secs(1);
                u
            }
            Err(e) => {
                eprintln!("getUpdates error: {e}, retry in {backoff:?}");
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(Duration::from_secs(120));
                continue;
            }
        };

        for update in &updates {
            offset = update.id.as_offset();
            let mut st = state.lock().await;

            // ── Callback query from inline button ───────────────
            if let Some(cq) = update.callback_query() {
                if let Some(data) = &cq.data {
                    if let Some(project_id) = data.strip_prefix("reply:") {
                        let name = st.display_name(project_id).to_string();
                        st.active_reply = Some(project_id.to_string());

                        bot.answer_callback_query(&cq.id).await.ok();
                        bot.send_message(
                            chat_id,
                            format!("[{name}] Type your response:"),
                        )
                        .reply_markup(ForceReply::new())
                        .await
                        .ok();
                    }
                }
                continue;
            }

            // ── Text message ────────────────────────────────────
            if let Some(msg) = update.regular_message() {
                if msg.chat.id != chat_id {
                    continue;
                }

                if let Some(text) = msg.text() {
                    // /project <id> <message> — manual override
                    if let Some(rest) = text.strip_prefix("/project ") {
                        if let Some((pid, reply)) = rest.split_once(' ') {
                            if let Some(tx) = st.operators.get(pid) {
                                let _ = tx.send(reply.to_string()).await;
                            }
                            st.active_reply = None;
                            continue;
                        }
                    }

                    // /status — list pending questions
                    if text == "/status" {
                        // implementation: iterate operators, report which
                        // have pending Ask requests
                        continue;
                    }

                    // Route to active project
                    if let Some(pid) = st.active_reply.take() {
                        if let Some(tx) = st.operators.get(&pid) {
                            let _ = tx.send(text.to_string()).await;
                        }
                    } else {
                        bot.send_message(
                            chat_id,
                            "No active question. Use /status to see pending projects.",
                        )
                        .await
                        .ok();
                    }
                }
            }
        }
    }
}
```

---

## Persistence Across Restarts

If the operator process is killed while waiting for a human reply, the session and pending question are lost. To survive restarts, persist the state to disk before asking and delete it after receiving the reply.

```rust
#[derive(serde::Serialize, serde::Deserialize)]
struct PendingState {
    session_id: String,
    question: String,
    asked_at: chrono::DateTime<chrono::Utc>,
}

// Write before sending the question
let state = PendingState {
    session_id: self.session_id.clone().unwrap(),
    question: result.message.clone(),
    asked_at: chrono::Utc::now(),
};
std::fs::write(
    self.project_dir.join(".rex-pending.json"),
    serde_json::to_string(&state)?,
)?;

// Delete after receiving the reply
std::fs::remove_file(self.project_dir.join(".rex-pending.json")).ok();
```

On startup, check for this file. If it exists, skip the initial `/rex-operator` invocation and jump straight to waiting for the reply, then resume the session with `--resume <session_id>`.

---

## Robustness Summary

| Failure Mode | Mitigation |
|---|---|
| Claude CLI hangs | `tokio::time::timeout` + `kill_on_drop(true)` on child process |
| Claude CLI rate-limited / overloaded | Exponential backoff retry (30s / 60s / 120s), up to `max_retries` |
| Claude returns unparseable output | Error propagated to broker, user notified via Telegram |
| Telegram API temporarily down (send) | Retry with backoff up to 5 attempts on `send_message` |
| Telegram API temporarily down (recv) | `getUpdates` loop retries with exponential backoff, no message loss — Telegram queues unretrieved updates server-side |
| User takes a long time to reply | 1-week timeout before the operator gives up |
| Stale messages in channel | Channel drained before every question is sent |
| Process restarts during wait | `.rex-pending.json` persists session ID and question, operator resumes on next launch |
| Multiple operators, one Telegram bot | Broker pattern — single `getUpdates` consumer, IPC routing via project ID |
| User sends message to wrong project | Inline keyboard callbacks tag each reply context; `ForceReply` captures the next message unambiguously; `/project` command as manual override |
| Bot token revoked / permanent Telegram failure | 1-week timeout trips, operator exits with error |

---

## Dependencies

```toml
[dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros", "process", "time"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
chrono = { version = "0.4", features = ["serde"] }

# Broker only
teloxide = { version = "0.13", features = ["macros"] }
```
