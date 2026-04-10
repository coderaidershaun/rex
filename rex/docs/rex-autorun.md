# Rex Autorun

Rex Autorun is a headless autopilot that drives a rex project to completion unattended. It repeatedly invokes the `/rex-operator` skill via the agent CLI, parses the structured JSON output, and loops. When the operator needs human input (onboarding questions, design acceptance), the question is relayed to Telegram and the binary waits for a reply. One project per instance. One Telegram chat.

---

## Prerequisites

1. **Rex project initialized and active.** Run `rex init`, then `rex project create` and step through the prompts.
2. **Telegram bot.** Create a dedicated autorun bot via [@BotFather](https://t.me/BotFather) on Telegram. You need the bot token and your chat ID.
3. **`.env` file** in the rex project root with:
   ```
   REX_AUTORUN_TELEGRAM_BOT_TOKEN=<your-autorun-bot-token>
   REX_TELEGRAM_CHAT_ID=<your-chat-id>
   ```

To find your chat ID: send any message to your bot, then visit `https://api.telegram.org/bot<token>/getUpdates` and look for `"chat":{"id":<number>}`.

---

## Quick Start

```bash
# From the rex project root (where you ran rex init)
rex-autorun

# Or run in the background with nohup (recommended for unattended runs).
# Always use --project-dir with an absolute path so the process
# finds the correct project regardless of working directory.
nohup rex-autorun --project-dir /absolute/path/to/project > /dev/null 2>&1 &
```

The binary loads the active project from `rex/projects.json`, starts invoking the operator, and sends you Telegram messages for status updates and questions.

---

## Command

```
rex-autorun [OPTIONS]
```

Rex Autorun is a separate binary installed alongside `rex` when you run `cargo install rex-cli`.

---

## Options

| Flag | Default | Description |
|------|---------|-------------|
| `--project-dir <PATH>` | `.` (current directory) | Rex project root directory. Must contain `rex/projects.json` and a skills directory. |
| `--model <MODEL>` | `opus[1m]` (Claude) / `claude-4.6-opus-high` (Cursor) | Model identifier passed to the agent CLI. |
| `--max-budget-usd <AMOUNT>` | `50.0` | Maximum USD spend per single invocation (Claude only; ignored for Cursor). |
| `--max-total-budget-usd <AMOUNT>` | `500.0` | Maximum USD spend across ALL invocations (Claude only). Hard stop — exits with code 5. |
| `--max-turns <N>` | `200` | Maximum agentic turns per invocation (Claude only). |
| `--process-timeout <MINS>` | `60` | Maximum minutes a single agent process may run before being killed. |
| `--max-retries <N>` | `5` | Maximum consecutive retries for transient failures (rate limits, timeouts, connection errors). |
| `--human-timeout <DAYS>` | `1` | Maximum days to wait for a user reply via Telegram before giving up. |
| `--log-file <PATH>` | `.rex-autorun.log` | Path to the JSONL structured log file. |
| `-h`, `--help` | | Print help. |
| `-V`, `--version` | | Print version. |

### Examples

```bash
# Run with defaults from the project root
rex-autorun

# Point at a specific rex project directory
rex-autorun --project-dir /Users/me/Code/my-rex-project

# Lower budget limits for a test run
rex-autorun --max-budget-usd 50.0 --max-total-budget-usd 300.0

# Give the agent more time per invocation and allow more retries
rex-autorun --process-timeout 120 --max-retries 10

# Custom log file location
rex-autorun --log-file /tmp/autorun.log

# Background with nohup — use --project-dir so the process doesn't
# depend on the shell's working directory
nohup rex-autorun --project-dir /Users/me/Code/my-rex-project > /dev/null 2>&1 &
```

---

## Environment Variables

Set these in a `.env` file in the project root directory, or export them in your shell.

| Variable | Required | Description |
|----------|----------|-------------|
| `REX_AUTORUN_TELEGRAM_BOT_TOKEN` | Yes | Telegram Bot API token for the autorun bot (from @BotFather). |
| `REX_TELEGRAM_CHAT_ID` | Yes | Telegram chat ID for the user. This is where questions and notifications are sent. |
| `RUST_LOG` | No | Controls log verbosity to stderr. Default: `info`. Set to `debug` for verbose output, `warn` for quieter. |

---

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Project completed successfully — all items in `project-status.json` are done. |
| `1` | Unrecoverable error (operator error, parse failure, non-retryable CLI failure). |
| `2` | Human reply timeout exceeded (no Telegram reply within `--human-timeout` days). |
| `3` | Maximum retries exhausted for transient failures. |
| `4` | Clean shutdown — SIGTERM or SIGINT (Ctrl+C) received. |
| `5` | Total budget exceeded (`--max-total-budget-usd`). |
| `6` | Killed by `/kill` command via Telegram. |

---

## Telegram Commands

While autorun is running, you can send commands to your Telegram bot at any time. Commands work both when the agent is processing and when the bot is waiting for your reply.

| Command | Description |
|---------|-------------|
| `/kill <project-id>` | Terminate the autorun session immediately. Kills any running agent process, cleans up state, and exits with code 6. `/kill` without a project ID kills the current session. |
| `/query <project-id>` | Show live stats: uptime, context usage, session duration, task progress, cost, and other running autoruns. `/query` without a project ID also works. |
| `/commands` | Show available commands. |
| `/start` | Same as `/commands`. |
| `/menu` | Same as `/commands`. |
| `/clear` | Clear chat history (deletes recent messages). |

See `rex/docs/telegram.md` for the full Telegram reference including inline buttons, reply routing, and multi-autorun triage behaviour.

---

## Reply Matching

Autorun uses Telegram's reply-to mechanism to prevent stray messages from being consumed as answers:

1. Questions are sent with inline **Reply** / **Stats** / **Kill** buttons.
2. Tapping **Reply** sends a ForceReply prompt — your Telegram client will open the reply composer.
3. Only messages with `reply_to_message.message_id` matching the question are accepted.
4. Messages that are not replies to a question trigger an "unprocessed" response listing active autoruns.
5. `/kill` and `/query` commands are always processed regardless of reply-to status.

When multiple autoruns share the same bot token, a cooperative triage system routes messages to the correct instance via a shared registry and per-project inbox files.

---

## Auth Token Refresh

If the agent's auth token expires during operation, autorun handles it automatically:

1. The 401 authentication error is detected from the agent's stderr.
2. For Claude: `claude auth login` is spawned and its output is scanned for an authorization URL. For Cursor: a manual instruction is sent.
3. The URL (or manual instruction) is sent to Telegram with ForceReply.
4. Autorun waits up to 10 minutes for you to authorize and reply.
5. On your confirmation, the invocation is retried.

Auth refresh is attempted at most once per session. If auth fails again after refresh, it's treated as a fatal error.

---

## How It Works

### Core Loop

1. Load the active project from `rex/projects.json`.
2. Invoke the agent CLI with `/rex-operator` as the prompt, JSON output format, and the configured model.
3. While the agent runs, poll Telegram for `/kill` and `/query` commands (1-second polling interval).
4. Parse the JSON output for a status: `completed`, `project_done`, `needs_input`, or `error`.
5. Route:
   - **completed** — notify Telegram with model header and stats, wait 5 seconds, invoke again.
   - **project_done** — notify Telegram, exit 0.
   - **needs_input** — send question to Telegram (ForceReply), wait for reply, send ack, resume the session.
   - **error** — notify Telegram, exit 1.
6. Before each invocation, check total spend against `--max-total-budget-usd`.

### Interactive Items (Needs Input)

When the operator hits an onboarding or user-acceptance item, it cannot proceed without a human. The binary:

1. Saves the session ID and question to `.rex-autorun.json`.
2. Sends the question to Telegram with **ForceReply** markup.
3. Polls for a reply with 1-second intervals (up to `--human-timeout` days).
4. Only accepts messages that are direct replies to the question (reply-to matching).
5. Sends an acknowledgment message on receipt.
6. Resumes the agent session with `--resume <session-id>`.
7. Repeats if the skill needs follow-up questions.

### Session Tagging

Every agent session is named `rex-autorun-<project-id>-<N>` where N is the invocation number (Claude only; Cursor does not support session naming).

---

## Crash Recovery

The binary persists its state to `.rex-autorun.json` using atomic writes (write to temp, fsync, rename). If the binary crashes or is killed:

| State at crash | Recovery on restart |
|---------------|---------------------|
| Agent was running (PID alive) | Kill the orphaned process group, start fresh. |
| Agent was running (PID dead) | Process already exited, start fresh. |
| Waiting for Telegram reply | Re-send the question to Telegram (ForceReply), resume waiting. |
| State file corrupt | Delete and start fresh. |
| No state file | Clean start. |

### Session Leak Prevention

When the binary spawns the agent CLI, it creates a new process group. If the binary is killed, the entire process group (including any sub-agents) is terminated. Four layers of protection:

1. **Process group isolation** — the agent runs in its own process group.
2. **PID tracking** — the state file records the process ID.
3. **Signal handlers** — SIGTERM/SIGINT cleanly kill the process group before exiting.
4. **Startup sweep** — orphaned processes from a previous crash are killed on startup.

---

## Files

| File | Description |
|------|-------------|
| `.rex-autorun.json` | State file for crash recovery. Created during operation, deleted on clean exit. |
| `.rex-autorun.log` | JSONL structured log of all events (invocations, completions, errors, Telegram interactions). |

Both files are created in the project root directory (or at `--log-file` for the log).

### Log Format

Each line in `.rex-autorun.log` is a JSON object with an `event` field:

```jsonl
{"event":"Started","project_id":"my-project","timestamp":"2026-04-03T12:00:00Z"}
{"event":"InvocationStarted","n":1,"timestamp":"2026-04-03T12:00:01Z"}
{"event":"InvocationCompleted","n":1,"status":"Completed","message":"Completed: goal (onboarding)","session_id":"abc-123","cost_usd":1.23,"duration_ms":45000,"timestamp":"2026-04-03T12:00:46Z"}
{"event":"NeedsInput","question":"What is the project goal?","session_id":"def-456","timestamp":"2026-04-03T12:01:00Z"}
{"event":"InputReceived","reply_length":142,"timestamp":"2026-04-03T12:15:00Z"}
{"event":"AuthRefresh","project_id":"my-project","timestamp":"2026-04-03T12:20:00Z"}
{"event":"KilledByUser","project_id":"my-project","timestamp":"2026-04-03T18:00:00Z"}
{"event":"ProjectDone","total_cost_usd":89.45,"total_invocations":27,"total_duration":"6h 12m","timestamp":"2026-04-03T18:12:00Z"}
```

---

## Tips

- **Start small.** Use `--max-total-budget-usd 50.0` for your first run to cap spend while you verify it works.
- **Watch the log.** `tail -f .rex-autorun.log | jq .` gives a live view of what's happening.
- **Run in the background.** `nohup rex-autorun --project-dir /absolute/path/to/project > /dev/null 2>&1 &` — always pass `--project-dir` with an absolute path when using nohup, so the process finds the correct project regardless of working directory. Check `.rex-autorun.log` for progress.
- **Stop cleanly.** Send `/kill <project-id>` via Telegram, or SIGTERM/Ctrl+C locally. The binary cleans up the agent process group and state file before exiting.
- **Check status.** Send `/query <project-id>` via Telegram to see total uptime, context usage, session duration, cost, and whether other autoruns are running.
- **Resume after crash.** Just run `rex-autorun` again. It reads `.rex-autorun.json` and picks up where it left off.
- **One bot per project.** Telegram's `getUpdates` API is exclusive — only one process can poll a given bot. If you need concurrent projects, use separate Telegram bots.
