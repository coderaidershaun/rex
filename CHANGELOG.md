# Changelog

All notable changes to **rex-cli** are documented here.

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
