# Changelog

All notable changes to **rex-cli** are documented here.

## 0.2.6 — 2026-04-05

- **Fix task count path in daemon** — `task_counts()` in rex-chat now correctly looks for planning data in the `rex/<project-id>/` subdirectory instead of the project root, fixing incorrect 0/0 task counts in Telegram status messages.
- **Model routing guidance in init** — Generated CLAUDE.md now includes a section pointing agents to the `rex-model-router` skill for model, effort, and context decisions.

## 0.2.5 — 2026-04-05

- **Refactor project validation** — `project create` with the refactor category now requires the target directory to already exist, instead of scaffolding a new project with `cargo new`.
- **Duplicate project directory guard** — `project create` checks whether `rex/<project-id>/` already exists on disk before proceeding, and errors out with a message to choose a different ID if it does.

## 0.2.4 — 2026-04-05

- **Integration testing in onboarding** — Add `integration-testing` as a new onboarding step and wire it as an input to all design and planning phases, so integration test preferences gathered during onboarding flow through to architecture, modules, errors, and critique design steps.
- **Skill refinements** — Improve instructions for planning-tasks, planning-objectives, planning-review, integration-testing, design-integration-tests, design-user-acceptance, and research-reducer skills.

## 0.2.3 — 2026-04-04

- **Varied thinking messages** — Rex-chat now shows a randomly chosen status message (from 50 variations) while the agent is working, instead of always "Thinking...".

## 0.2.2 — 2026-04-04

- **Fix systemd PATH for claude binary** — Add PATH environment variable to the rex-chat systemd service example so `claude` (typically in `~/.local/bin`) is found when running as a service.

## 0.2.1 — 2026-04-04

- **Smart message routing** — Bare messages now route directly to the active project's Claude agent instead of showing a menu. Priority: last-chatted project > single project > first available. Every non-command message gets an agent response.
- **Text commands** — Full text-based project management: `/start <id>`, `/stop <id>`, `/status [id]`, `/chat <id>`, `/projects`. No buttons required.
- **Project-prefixed chat** — `project-id: message` syntax targets a specific project and switches context to it.
- **Process group cleanup** — Spawned `claude -p` processes now track their PGID. On daemon shutdown, session timeout, or Claude timeout, the entire process group is killed (SIGTERM + SIGKILL) to prevent orphaned processes.

## 0.2.0 — 2026-04-04

- **Project-agnostic rex-chat** — Rex-chat no longer requires `--project-dir`. It auto-discovers all rex projects by scanning the filesystem from `$HOME` (or a custom `--scan-dir` root) for `rex/projects.json` files. No CWD dependency, no single-registry assumption — just run `rex-chat` and it finds everything.
- **Real-time state monitoring** — Rex-chat polls the filesystem every 5 seconds and sends Telegram notifications when projects are created, removed, or when autoruns start or stop. No manual refresh needed.
- **Breaking: `--project-dir` replaced by `--scan-dir`** — The old `--project-dir` flag is removed. Use `--scan-dir` to narrow the scan root, or omit it to scan `$HOME`. The systemd service no longer needs `WorkingDirectory` or project-specific paths.

## 0.1.30 — 2026-04-04

- **Dependency upgrades** — Bump console 0.15→0.16, dialoguer 0.11→0.12, reqwest 0.12→0.13, teloxide 0.13→0.17. Reqwest TLS feature renamed from `rustls-tls` to `rustls` (now uses aws-lc crypto backend). All public APIs remain compatible.
- **Fix `wait_for_reply` visibility** — Make `TelegramClient::wait_for_reply` public so integration tests can call it (was `pub(crate)`).

## 0.1.29 — 2026-04-04

- **Separate chat and autorun Telegram bots** — Each daemon now uses its own dedicated bot token (`REX_AUTOCHAT_TELEGRAM_BOT_TOKEN` for rex-chat, `REX_AUTORUN_TELEGRAM_BOT_TOKEN` for rex-autorun) and `REX_TELEGRAM_CHAT_ID` as the shared chat ID. No more cross-filtering between bots or shared `TELEGRAM_BOT_TOKEN`.
- **Cooperative multi-autorun triage** — When multiple autoruns share the same bot token, a file-lock-based triage system ensures only one polls Telegram at a time. A shared registry tracks each autorun's PID and expected reply message, and cross-project messages are routed via per-project inbox files. A `RegistryGuard` RAII struct guarantees deregistration on all exit paths.
- **Rex-chat simplified** — Remove `/rex-chat` command prefix (replaced by `/menu` and `/start`), remove all inbox IPC and autorun message routing, simplify session manager by removing `autorun_reply_map`. Chat bot now operates fully independently.
- **Inline buttons on questions** — Autorun "input needed" messages now include Reply + Stats + Kill inline buttons instead of ForceReply-only. Tapping Reply sends a ForceReply prompt.
- **Task progress in stats** — `/query` and status messages now show task completion counts (`Tasks: 14/23`) loaded from `planning/planning.json`.
- **`/clear` and `/commands`** — Both bots support `/clear` to delete recent chat history and `/commands` (or `/start`, `/menu`) to show available commands.
- **Duration formatting fix** — Completion messages now use `Xm Ys` format (via `format_duration_ms`) instead of raw seconds.
- **1M context window** — All headless `claude -p` calls now use `sonnet[1m]` to ensure the 1M context window.
- **Blockquote formatting** — Error messages use `<blockquote>` for highlighted error text. Chat response footer text removed (buttons are sufficient).
- **Integration tests** — New `test_telegram_messages` and `test_chat_messages` tests exercise the real `TelegramClient` code, sending every message type to verify formatting and button layout on both bots.

## 0.1.28 — 2026-04-03

- **Interactive Telegram chat sessions** — New `/chat` command lets you ask questions about the running project mid-session. Each chat spawns a parallel Claude instance scoped to the project directory, with inline Reply and Restart buttons for multi-turn conversation — all without interrupting the active autorun work item.
- **Telegram message editing and chat routing** — `TelegramClient` gains `edit_message`, `edit_message_with_chat_buttons`, `send_chat_reply_prompt`, and `send_with_chat_buttons` methods. The main poller now routes callback queries and force-reply messages to the correct chat session via a `ChatManager`.

## 0.1.27 — 2026-04-03

- **Telegram message formatting overhaul** — Align all autorun notifications with the rich formatting from integration tests: emoji-prefixed titles, `⎯` dividers via shared `DIV` constant, `<blockquote>` for questions, and a consistent `EMOJI <b>Title</b>  ·  <code>{pid}</code>` header on every message without exception.
- **Inline Stats/Kill/Reply buttons** — Startup and completion messages now include inline keyboard buttons for 📊 Stats, 🛑 Kill, and 💬 Reply. Both polling functions handle `callback_query` updates alongside text commands. Reply button sends a follow-up `force_reply` prompt.
- **Work item name in notifications** — Add `item` field to `OperatorResult` and operator skill JSON output. Telegram messages now show the current work item (e.g. `goal`, `architecture`, `t-token-endpoint`) after the project ID so you know what topic a question relates to.
- **Fix context percentage calculation** — The old formula summed input + output + cache_read + cache_creation tokens, giving wildly inflated values (100%+). Now correctly uses `(input_tokens + cache_read_input_tokens) / context_window` — excluding output tokens and the cache_creation double-count.

## 0.1.26 — 2026-04-03

- **Fix default branch name** — `git init` in both `rex init` and `rex mono` now uses `-b main` to ensure new repositories start on the `main` branch instead of inheriting the system default (often `master`), preventing branch name mismatches with GitHub remotes.

## 0.1.25 — 2026-04-03

- **Publish-to-git: prefer `gh` CLI** — the `rex-publish-to-git` skill now checks for a working `gh` CLI and uses it for push operations when available, falling back to standard git otherwise.
- **README: autorun in monorepo quickstart** — add `nohup rex-autorun --project-dir` example to the monorepo quickstart section so users know how to run autorun headlessly for a specific project.

## 0.1.24 — 2026-04-03

- **`--with-git-repo` flag** — `rex mono` and `rex project create` now accept `--with-git-repo <public|private>` to create a GitHub repository via the `gh` CLI and add it as the `origin` remote during project/workspace setup.
- **Unified `rex mono` command** — replace separate `rex mono init` and `rex mono empty` subcommands with a single `rex mono --name <name> [--no-harness]` command. The `--no-harness` flag replaces the old `empty` subcommand.
- **Autorun: reply-to matching** — Telegram questions now use `ForceReply` markup and poll for replies that match the specific `message_id`, preventing cross-talk when multiple projects are running.
- **Autorun: `/kill` command** — send `/kill <project-id>` in Telegram to gracefully stop a running autorun session (exit code 6).
- **Autorun: `/stats` command** — send `/stats` in Telegram to get a summary of the current session (invocations, cost, uptime, model).
- **Autorun: auth refresh** — automatically detect expired Claude auth tokens and attempt `claude auth login --api-key` recovery before failing.
- **Autorun: detailed cost tracking** — parse `total_cost_usd`, `modelUsage`, and `usage.speed` from Claude output for richer session reporting.
- **Autorun: documented exit codes** — exit codes 0–6 now have defined meanings (success, fatal, timeout, retries, signal, budget, killed).
- **README quickstart** — new "Quickstart Example — Monorepo with Individual Projects" section showing the full workflow from workspace creation to per-project harness setup.

## 0.1.23 — 2026-04-03

- **`rex mono empty`** — new subcommand to create a bare Cargo workspace (no rex harness, no `.claude/` folder) with git initialized. Useful for monorepos where rex is initialized per-project rather than at the root.
- **Init inside project** — `rex project create` now prompts whether to run `rex init` inside the project directory, creating a fully self-contained project with its own harness, skills, and registry. Defaults to Yes when no outer harness exists.
- **`rex --commands`** — new flag that prints a formatted list of every command and subcommand with descriptions.
- **Documentation overhaul** — README, monorepo, projects, init, and main docs all updated for new commands, missing project commands (lock, unlock, update-category, update-complexity), and the init-inside workflow.

## 0.1.22 — 2026-04-03

- **Error handling overhaul** — new `RexError` enum via `thiserror` replaces `Box<dyn Error>` across all commands, models, autorun, and UI modules. Provides structured error variants (`FileRead`, `NotFound`, `Validation`, `Subprocess`, etc.) with contextual messages.
- **Publish skills** — add `rex-publish-to-git` (commit and push with meaningful messages) and `rex-publish-to-crates-io` (full release workflow: version bump, changelog, commit, publish).

## 0.1.21 — 2026-04-03

- **Fix crates.io keywords** — reduce keywords from 6 to 5 (crates.io maximum).

## 0.1.20 — 2026-04-03

- **Remove Cursor support** — `rex init` now targets Claude Code only. Removed `--cursor` / `--claude` flags, `AgentOs` enum, interactive agent OS prompt, and all Cursor-specific hook configuration (`write_cursor_hooks`, `merge_cursor_hooks`, `CURSOR_HOOKS_JSON`).
- **Add `rex-autorun` binary** — headless autopilot that drives a project to completion via `claude -p`, with Telegram integration for relaying questions.
- **Operator skill updates** — improved dispatch rules and session lifecycle handling.
- **Documentation refresh** — all docs updated to reflect Claude Code-only workflow.

## 0.1.19 — 2026-04-02

- **Execution task limit** — operator processes up to 3 tasks per invocation instead of 1, reducing round-trips during execution phase.

## 0.1.18 — 2026-04-01

- **Planning review skill** — new `rex-planning-review` skill for adversarial review of the planning tree (milestones, objectives, tasks) against design documents.
- **Project status improvements** — additional fields and logic in `project-status.json` for execution phase tracking.

## 0.1.17 — 2026-04-01

- **Project status fix** — minor corrections to `project-status.json` serialisation.

## 0.1.16 — 2026-04-01

- **Architecture proposal improvements** — updated HTML viewer template and skill instructions for `rex-design-rust-architecture-proposal`.
- **User acceptance skill tweak** — minor instruction update for `rex-design-user-acceptance`.
- **Project status refinement** — simplified status model fields.

## 0.1.15 — 2026-04-01

- **Project status patch** — added missing field to project status model.

## 0.1.14 — 2026-04-01

- **Monorepo support** — new `rex mono init --name <name>` command creates a Cargo workspace monorepo with rex harness pre-configured (workspace Cargo.toml, libs/, .gitignore, git init, rex init).
- **Project lock/unlock** — new `rex project lock` / `rex project unlock` commands to prevent the operator from advancing a project.

## 0.1.13 — 2026-04-01

- **Per-task agent config** — tasks can carry `--agent-model`, `--agent-effort`, `--agent-skill`, `--agent-count` fields that override the execution item's default agent config.
- **`rex task next`** — new command returns the highest-priority eligible task with its parent objective and milestone.
- **Operator execution phase** — operator now switches from linear `project-status.json` to the planning tree when it reaches the execution item.
- **Skill building improvements** — updated `rex-onboarding-skill-building` skill.

## 0.1.12 — 2026-04-01

- **`rex project update-category`** — new command to change the active project's category.
- **`rex project update-complexity`** — new command to change the active project's complexity.

## 0.1.11 — 2026-04-01

- **Hybrid dispatch** — operator directly invokes skills for interactive items (goal, scope, uat) and uses sub-agents for the rest.

## 0.1.10 — 2026-04-01

- **Operator contract redesign** — work item is now the single source of truth for agent dispatch (model, effort, skills, count).

## 0.1.9 — 2026-04-01

- **Dynamic skill names** — operator prompt templates reference skill names from the work item instead of hardcoding them.
- **Stronger agent dispatch** — improved model selection, skill invocation, and effort handling in operator.
- **Anti-menu rules** — strengthened across all onboarding skills.

## 0.1.6 — 2026-04-01

- **Binary rename** — renamed from `rex-cli` to `rex`.
- **Stop-on-finish** — operator respects the `stop-on-finish` field and loops when false.

## 0.1.5 — 2026-04-01

- **SendMessage relay** — operator skill now includes instructions for relaying user questions via SendMessage.
- **Open-ended onboarding** — conversational style for onboarding skills instead of menu-driven.

## 0.1.2 — 2026-04-01

- **Initial release** — project CLI with create/active commands, onboarding phase with 14 items, project-status.json manifest, complexity field, category/onboarding selection widget, cargo scaffold integration.
