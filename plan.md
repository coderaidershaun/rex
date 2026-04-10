# Rex: Claude Code to Cursor Migration Plan

## Executive Summary

Rex is a Rust CLI (`rex-cli`) that orchestrates AI-agent-driven project development through four phases: onboarding, design, planning, and execution. It currently spawns `claude` CLI processes headlessly and parses their JSON output. This plan covers every change needed to make rex work with Cursor's agent CLI instead.

The codebase has **three binaries** (`rex`, `rex-autorun`, `rex-chat`) and a **library** (`rex_cli`) with six modules: `autorun`, `chat`, `commands`, `errors`, `models`, `ui`. The Claude-specific code is concentrated in the `autorun` and `chat` modules, plus `commands/init.rs`.

---

## 1. Fundamental Platform Differences

Before diving into code changes, these architectural differences between Claude Code CLI and Cursor CLI drive the migration:

| Concern | Claude Code | Cursor |
|---------|-------------|--------|
| CLI command | `claude -p "<prompt>"` | `cursor agent -p "<prompt>"` |
| File edit permission | `--dangerously-skip-permissions` | `--force` / `--yolo` / `--trust` |
| Output format | `--output-format json` | `--output-format json` (compatible) |
| Model selection | `--model sonnet[1m]` | `--model <model-name>` (e.g. `claude-4.6-sonnet`) |
| Effort/thinking | `--effort high` | No equivalent -- use rules/AGENTS.md |
| Session resume | `--resume <session-id>` | **Not supported** -- sessions are stateless |
| Session naming | `--name <name>` | Not supported |
| System prompt append | `--append-system-prompt "<text>"` | **Not supported** -- use AGENTS.md/rules |
| Budget per invocation | `--max-budget-usd <amount>` | Not applicable (subscription-based) |
| Max turns | `--max-turns <n>` | Needs verification -- may not be supported |
| Auth | `claude auth login` / OAuth | `cursor auth login` / `CURSOR_API_KEY` env |
| Config directory | `.claude/` | `.cursor/` |
| Root instructions file | `CLAUDE.md` | `AGENTS.md` |
| Hook config | `.claude/settings.json` (nested JSON) | `.cursor/hooks.json` (flat JSON) |
| Skills | `.claude/skills/SKILL.md` | `.cursor/skills/SKILL.md` (same format) |
| Cost tracking | JSON output includes `total_cost_usd`, token counts | No cost data in output (subscription model) |
| Context window info | `modelUsage` with token counts and window size | Not available in CLI output |

---

## 2. Source Code Changes

### 2.1 Rename `autorun/claude.rs` to `autorun/cursor_agent.rs`

**File:** `src/autorun/claude.rs` (rename to `src/autorun/cursor_agent.rs`)

This is the most critical file. It spawns the AI process and parses output.

**Changes to `spawn_claude` (rename to `spawn_cursor_agent`):**

Current Claude invocation:
```rust
cmd = tokio::process::Command::new("claude");
cmd.arg("-p").arg(prompt);
cmd.arg("--output-format").arg("json");
cmd.arg("--model").arg("sonnet[1m]");
cmd.arg("--effort").arg("high");
cmd.arg("--dangerously-skip-permissions");
cmd.arg("--max-turns").arg(max_turns.to_string());
cmd.arg("--max-budget-usd").arg(format!("{max_budget_usd:.2}"));
cmd.arg("--append-system-prompt").arg(AUTORUN_SYSTEM_PROMPT);
// Session resume:
if let Some(sid) = session_id_to_resume {
    cmd.arg("--resume").arg(sid);
}
cmd.arg("--name").arg(session_name);
```

New Cursor invocation:
```rust
cmd = tokio::process::Command::new("cursor");
cmd.arg("agent");
cmd.arg("-p").arg(prompt);
cmd.arg("--force");  // replaces --dangerously-skip-permissions
cmd.arg("--output-format").arg("json");
cmd.arg("--model").arg(model);  // passed as parameter, not hardcoded
// No --effort, --max-budget-usd, --append-system-prompt, --resume, --name
```

**Key design decisions:**
- **No session resume**: The multi-round `needs_input` flow must change. Instead of resuming a session, start a fresh Cursor agent with the user's reply prepended to the operator prompt, plus context from the previous invocation written to a temp file.
- **No system prompt append**: Move the `AUTORUN_SYSTEM_PROMPT` content into a `.cursor/rules/` file that is always loaded, or prepend it to the prompt itself.
- **No budget tracking**: Remove per-invocation `max_budget_usd` from the spawn function. Replace cost tracking with invocation counting and time-based limits only.
- **Model parameter**: Accept a `model: &str` parameter instead of hardcoding. Map from rex's `opus`/`sonnet`/`haiku` to Cursor model identifiers.

**Changes to `await_claude` (rename to `await_cursor_agent`):**
- Same structure (timeout + read stdout/stderr + parse JSON)
- Update error detection strings for Cursor-specific errors

**Changes to `parse_operator_result`:**
- No change needed -- this parses rex's own JSON protocol embedded in the result text, not Claude-specific output

**Changes to `is_auth_error` and `is_retryable`:**
- Update error strings to match Cursor CLI error messages (e.g., "CURSOR_API_KEY" instead of "oauth token has expired")

### 2.2 Update `autorun/types.rs`

**File:** `src/autorun/types.rs`

**Rename `ClaudeOutput` to `CursorOutput`:**

The JSON output structure from `cursor agent -p --output-format json` will differ from Claude's. The current `ClaudeOutput` expects:
```json
{
    "result": "...",
    "session_id": "...",
    "cost": { ... },
    "total_cost_usd": 0.0,
    "duration_ms": 0,
    "usage": { ... },
    "modelUsage": { ... }
}
```

Cursor's JSON output structure needs to be determined empirically (run `cursor agent -p --output-format json "hello"` and inspect). The struct must be updated to match. At minimum:
- Remove `cost`, `total_cost_usd` fields (or make them default to 0)
- Remove `modelUsage` / `usage` if not present
- Keep `result` and adapt `session_id` (may not exist -- generate a UUID instead)
- Remove `fast_mode_state`

**Rename `AutorunState` fields:**
- `claude_pid` -> `agent_pid`
- `claude_pgid` -> `agent_pgid`

**Update `RunStats`:**
- Remove `total_cost_usd` (or keep as 0.0 placeholder)
- Remove `context_percents` (no context window data from Cursor)
- Keep `session_durations_ms` for time tracking

### 2.3 Update `autorun/runner.rs`

**File:** `src/autorun/runner.rs`

**Remove budget-related logic:**
- Remove `max_budget_usd` from `Args` (or keep but mark as no-op/future)
- Remove `max_total_budget_usd` from `Args` and the budget check in the main loop
- Remove cost accumulation: `stats.total_cost_usd += cost`

**Redesign the `needs_input` multi-round flow:**

Current flow: spawn claude -> returns needs_input -> save session_id -> wait for Telegram reply -> resume session with `--resume <session_id>`.

New flow (no session resume available):
1. Spawn cursor agent -> returns needs_input -> save the question and context
2. Wait for Telegram reply
3. Start a **fresh** cursor agent with a prompt like: "The user was asked: `<question>`. Their reply: `<reply>`. Continue the operator from where it left off by running `/rex-operator`."
4. The fresh agent reads AGENTS.md and skills, picks up the project state from `project-status.json`, and continues

This means the `session_id` field in `AutorunState` and `RecoveryAction::ResumePendingInput` can be simplified.

**Remove Claude auth refresh logic:**
- Replace `attempt_auth_refresh` (which runs `claude auth login`) with Cursor auth refresh (either `cursor auth login` or instruct user to set `CURSOR_API_KEY`)
- Update error variant from `AuthExpired` to a generic `AuthError`

**Update Telegram stat messages:**
- Remove cost display (`$X.XX`)
- Remove context percentage display
- Keep time-based stats (uptime, session duration, invocation count)

**Rename all `claude::` references to `cursor_agent::`.**

### 2.4 Update `autorun/state.rs`

**File:** `src/autorun/state.rs`

- Rename references from `claude_pid`/`claude_pgid` to `agent_pid`/`agent_pgid`
- Update recovery logic comments

### 2.5 Update `autorun/mod.rs`

**File:** `src/autorun/mod.rs`

```rust
// Was:
pub mod claude;
// Now:
pub mod cursor_agent;
```

### 2.6 Update `chat/sessions.rs`

**File:** `src/chat/sessions.rs`

Same pattern as autorun -- replace `claude` command with `cursor agent`:

Current:
```rust
cmd = tokio::process::Command::new("claude");
cmd.arg("-p").arg(prompt);
cmd.arg("--model").arg("sonnet[1m]");
cmd.arg("--effort").arg("high");
cmd.arg("--dangerously-skip-permissions");
```

New:
```rust
cmd = tokio::process::Command::new("cursor");
cmd.arg("agent");
cmd.arg("-p").arg(prompt);
cmd.arg("--force");
cmd.arg("--output-format").arg("json");
cmd.arg("--model").arg(model);
```

Remove session resume (`--resume`, `--name`) from chat sessions too. Chat sessions will be stateless -- each message starts a fresh agent invocation with context from the skill.

Update `CHAT_SYSTEM_PROMPT` -- either embed it in the prompt directly or move to a `.cursor/rules/` file.

### 2.7 Update `commands/init.rs`

**File:** `src/commands/init.rs`

This is the harness initialization command. Currently scaffolds `.claude/` directory structure.

**Changes:**
- Change `SKILLS_DIR` from `include_dir!("$CARGO_MANIFEST_DIR/.claude/skills")` to `include_dir!("$CARGO_MANIFEST_DIR/.cursor/skills")`
- Change `HOOKS_DIR` from `include_dir!("$CARGO_MANIFEST_DIR/.claude/hooks")` to `include_dir!("$CARGO_MANIFEST_DIR/.cursor/hooks")`
- Remove `CLAUDE_SETTINGS_JSON` (`.claude/settings.json`) -- replace with Cursor hooks.json format
- Change target directories:
  - `.claude/` -> `.cursor/`
  - `.claude/skills/` -> `.cursor/skills/`
  - `.claude/hooks/` -> `.cursor/hooks/`
- Change root file from `CLAUDE.md` to `AGENTS.md`
- Update `ROOT_FILE_CONTENT` to reference `AGENTS.md` instead of `CLAUDE.md`
- Replace `write_claude_settings()` with `write_cursor_hooks()` that generates `.cursor/hooks.json`
- Update all print messages from "Claude Code" to "Cursor"

**New hooks.json format** (replace the Claude settings.json merging logic):
```json
{
    "version": 1,
    "hooks": {
        "stop": [{ "command": ".cursor/hooks/commit-and-push.sh" }]
    }
}
```

**Update the commit-and-push.sh hook:**
- Change `$CLAUDE_PROJECT_DIR` to `$CURSOR_PROJECT_DIR` (already done in the workspace copy)

### 2.8 Update `errors.rs`

**File:** `src/errors.rs`

Rename error variants:
- `ClaudeProcess(String)` -> `AgentProcess(String)`
- `AuthExpired(String)` -> `AuthError(String)` (Cursor auth errors are different)

Update all error messages that reference "claude".

### 2.9 Update `commands/mono.rs`

**File:** `src/commands/mono.rs`

Minimal changes -- just runs `rex init` which handles the details. No direct Claude references.

### 2.10 Update `Cargo.toml`

**File:** `Cargo.toml`

- Update package name/description if desired
- The `exclude` list references `.claude/settings.local.json` and `CLAUDE.md` -- update to `.cursor/` paths and `AGENTS.md`
- All dependencies remain the same

### 2.11 Update `COMMANDS_HELP` in `src/bin/main.rs`

**File:** `src/bin/main.rs`

- Replace all references to "Claude" with "Cursor" in the help text
- Remove `--max-budget-usd` from autorun options (or mark as deprecated)
- Update `--process-timeout-mins` description from "Max minutes per Claude process" to "Max minutes per agent process"

---

## 3. Autorun System Prompt Strategy

Since Cursor has no `--append-system-prompt`, the `AUTORUN_SYSTEM_PROMPT` must be delivered differently.

**Option A (Recommended): Prepend to prompt**

Embed the system prompt directly in the `-p` prompt text:
```rust
let full_prompt = format!("{}\n\n{}", AUTORUN_SYSTEM_PROMPT, prompt);
cmd.arg("-p").arg(&full_prompt);
```

This is the simplest approach and guaranteed to work.

**Option B: Use a Cursor rule file**

Create `.cursor/rules/rex-autorun.md` with the autorun instructions. Cursor automatically loads rules from this directory. However, this only works if the rule is always applicable (it should only apply during autorun, not interactive use).

**Recommendation**: Use Option A for the autorun system prompt (it's invocation-specific), and use `.cursor/rules/` for general project instructions that always apply.

---

## 4. Model Routing Updates

### 4.1 Model name mapping

The rex model router currently uses `opus`, `sonnet`, `haiku`. These need to map to Cursor model identifiers.

Create a mapping table:

| Rex model | Cursor model identifier | Notes |
|-----------|------------------------|-------|
| `haiku` | `fast` (Cursor's fast model) | Cheapest/fastest available |
| `sonnet` | Default / `auto` | The default Cursor agent model |
| `opus` | Best available (e.g., `claude-4.6-opus`) | Most capable model |

The exact model identifiers can be discovered via `cursor agent --list-models`. The mapping should be configurable (not hardcoded) -- store it in a config file or environment variables.

### 4.2 Multi-model advantage

Cursor uniquely offers models from multiple providers. Enhance the model router to optionally select:
- Claude models for Rust-specific reasoning
- Gemini models for large context windows
- GPT models as fallback/alternative

This is an enhancement beyond the basic port.

### 4.3 Update `rex-model-router` skill

**File:** `.cursor/skills/rex-model-router/SKILL.md`

Update the model names in routing tables from `haiku`/`sonnet`/`opus` to Cursor model identifiers. Update the model selection rationale section.

### 4.4 Update `rex-operator` skill

**File:** `.cursor/skills/rex-operator/SKILL.md`

- Update references to "Claude" processes
- Update the agent dispatch section -- Cursor uses the Task tool (subagents), not a separate CLI invocation for sub-agents within a session
- The operator skill itself doesn't need fundamental changes since it runs *inside* the agent session

---

## 5. Session Resume Redesign

This is the most significant architectural change. Claude Code supports `--resume <session-id>` to continue a conversation. Cursor CLI does not.

### 5.1 Impact on `needs_input` flow

The autorun `needs_input` flow currently:
1. Claude returns `{"status": "needs_input", "message": "..."}`
2. Autorun saves `session_id` and sends question to Telegram
3. User replies via Telegram
4. Autorun spawns `claude --resume <session_id> -p "<reply>"`
5. Claude continues the conversation with full context

Without resume, the new flow:
1. Cursor agent returns `{"status": "needs_input", "message": "..."}`
2. Autorun saves the question and writes context to `rex/<project-id>/user-support/autorun-context.md`
3. Sends question to Telegram
4. User replies
5. Autorun spawns a **fresh** Cursor agent with prompt:
   ```
   You are continuing an autorun session. The previous invocation asked the user:
   "<question>"
   
   The user replied:
   "<reply>"
   
   Read rex/<project-id>/user-support/autorun-context.md for context on what was being done.
   
   Now continue by running /rex-operator.
   ```
6. The fresh agent reads the project state and continues

### 5.2 Impact on chat sessions

Chat sessions also use `--resume` to maintain conversation context. Without it:
- Each chat message starts a fresh agent invocation
- Context is maintained via the `rex-chat` skill which reads project state
- This is acceptable since chat is for quick queries, not long conversations

### 5.3 Simplification

Removing session resume actually **simplifies** the codebase:
- Remove `session_id` tracking from `AutorunState`
- Remove `session_id` from `CursorOutput` (generate UUID for logging)
- Remove `RecoveryAction::ResumePendingInput` session_id field
- The `pending_input` recovery just re-asks the question and starts fresh

---

## 6. Hooks Configuration

### 6.1 Current Cursor hooks format (already in workspace)

The workspace already uses Cursor's hooks format at `.cursor/hooks.json`:
```json
{
    "version": 1,
    "hooks": {
        "stop": [{ "command": ".cursor/hooks/commit-and-push.sh" }]
    }
}
```

### 6.2 Changes to `init.rs`

Replace the Claude `settings.json` merge logic with Cursor `hooks.json` merge logic. The Cursor format is simpler (flat hook arrays per event) vs Claude's nested matcher/hooks structure.

### 6.3 Available Cursor hook events

Cursor supports more hook events than Claude:
- `sessionStart`, `sessionEnd`, `stop`
- `beforeShellExecution`, `afterShellExecution`
- `beforeMCPExecution`, `afterMCPExecution`
- `beforeReadFile`, `afterFileEdit`
- `beforeSubmitPrompt`, `preCompact`
- `afterAgentResponse`, `afterAgentThought`

Rex could leverage additional hooks:
- `afterFileEdit` -- run formatters or linters
- `afterAgentResponse` -- log agent activity for monitoring

---

## 7. Skills Compatibility

Skills use the same `SKILL.md` format for both Claude Code and Cursor. The workspace already has all 28+ rex skills under `.cursor/skills/`. **No structural changes needed to the skills format itself.**

However, **6 skill files contain Claude-specific content** that must be updated. The remaining ~22 skills are platform-agnostic and need no changes.

### 7.1 `rex-model-router/SKILL.md` -- Heavy changes

The entire routing table uses `haiku`/`sonnet`/`opus` model names (40+ occurrences). Every row in the Task Classification Table and Pipeline Phase Routing Table must be updated to use Cursor model identifiers.

Changes:
- Replace `haiku` with `fast` (or Cursor equivalent) in all routing tables
- Replace `sonnet` with the default Cursor model identifier in all routing tables
- Replace `opus` with the most capable Cursor model identifier in all routing tables
- Update Step 4 ("Spawn the agent") which says `Use the Agent tool` -- in Cursor this is the `Task tool`
- Update the `model` parameter values: `"haiku"`, `"sonnet"`, `"opus"` in the spawn table
- Update the "Model selection rationale" section: rename `Haiku 4.5`, `Sonnet 4.6`, `Opus 4.6` to Cursor equivalents
- Update the YAML frontmatter `description` which mentions "Haiku/Sonnet/Opus"
- Update all 4 worked examples that reference specific model names

### 7.2 `rex-operator/SKILL.md` -- Heavy changes

References Claude model names throughout the dispatch logic.

Changes:
- Step 3 example JSON: `"model": "opus"` -- update to Cursor model name
- Step 3c example JSON: `"model": "opus"` -- same
- Step 6 sub-agent dispatch table: `"opus"` → opus, `"sonnet"` → sonnet, `"haiku"` → haiku -- replace with Cursor model mapping
- Step 7 sub-agent dispatch: `Spawn one agent using the Agent tool` -- Cursor uses the `Task tool` (with `subagent_type` parameter)
- Step 7 multi-agent dispatch: same `Agent tool` → `Task tool` rename
- Step 7 `model` parameter mapping table: update all three model entries
- Step 11: `default to sonnet` -- update to Cursor default model
- Rules section: `agent.model` mapping table references opus/sonnet/haiku
- All references to "Agent tool" → "Task tool"

### 7.3 `rex-planning-tasks/SKILL.md` -- Moderate changes

Contains many example `rex task upsert` CLI commands with hardcoded model names.

Changes:
- All `--agent-model sonnet` in example commands -- update to Cursor model name
- All `--agent-model opus` in example commands -- update to Cursor model name
- The example text `"sonnet/high"` and `"opus/max"` descriptors -- update model portion
- Reference to `rex-model-router/SKILL.md` Task Classification Table -- ensure consistency after router updates
- Line mentioning `default to sonnet/high` for custom skills -- update model name

### 7.4 `rex-chat/SKILL.md` -- Minor changes

References the autorun state file field name.

Changes:
- Replace `claude_pgid` with `agent_pgid` (matching the types.rs rename)
- Any other references to Claude process naming

### 7.5 `rex-publish-to-git/SKILL.md` -- Minor changes

Uses Claude as the co-author in commit templates.

Changes:
- Replace `Co-Authored-By: Claude <noreply@anthropic.com>` with `Co-Authored-By: Cursor <noreply@cursor.com>` (or remove the co-author line entirely, or make it configurable)
- Two occurrences of this line in commit message templates

### 7.6 `find-skills/SKILL.md` -- Minor changes

References a Claude-specific skills repository.

Changes:
- Replace `ComposioHQ/awesome-claude-skills` with a Cursor-relevant skills source (e.g., `vercel-labs/skills` which is already referenced elsewhere, or remove the Claude-specific reference)

---

## 8. Init Command: Embedded Assets

The `init.rs` uses `include_dir!` to embed skills and hooks at compile time:

```rust
static SKILLS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/.claude/skills");
static HOOKS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/.claude/hooks");
```

These paths must change to `.cursor/skills` and `.cursor/hooks`. This means the **rex repository itself** must have its skills and hooks under `.cursor/` (which the workspace already does).

---

## 9. Environment Variables

### 9.1 Rename/add

| Current | New | Notes |
|---------|-----|-------|
| `REX_AUTORUN=1` | `REX_AUTORUN=1` | Keep as-is (rex-specific, not Claude-specific) |
| `REX_AUTORUN_TELEGRAM_BOT_TOKEN` | Keep as-is | Telegram integration unchanged |
| `REX_TELEGRAM_CHAT_ID` | Keep as-is | Telegram integration unchanged |
| `REX_AUTOCHAT_TELEGRAM_BOT_TOKEN` | Keep as-is | Telegram integration unchanged |
| (none) | `CURSOR_API_KEY` | For headless Cursor auth |
| (none) | `REX_CURSOR_MODEL` | Optional: override default model |

### 9.2 Auth for headless

Cursor headless requires either:
- `CURSOR_API_KEY` environment variable, or
- Prior `cursor auth login`

The autorun auth refresh flow should be updated to handle `CURSOR_API_KEY` and/or `cursor auth login`.

---

## 10. Testing Strategy

1. **Unit tests**: Update `tests/` to test new output parsing
2. **Manual smoke test**: Run `cursor agent -p --output-format json "hello"` to capture actual output format, then adapt `CursorOutput` struct
3. **Autorun integration test**: Run `rex-autorun` against a simple project and verify the full loop works
4. **Chat integration test**: Send a message via Telegram and verify rex-chat responds

---

## 11. File-by-File Change Summary

| File | Change type | Description |
|------|-------------|-------------|
| `src/autorun/claude.rs` | **Rename + Rewrite** | Rename to `cursor_agent.rs`, change CLI command, remove Claude-specific flags |
| `src/autorun/mod.rs` | Edit | `pub mod claude` -> `pub mod cursor_agent` |
| `src/autorun/types.rs` | Edit | Rename structs, remove cost/context fields, update output format |
| `src/autorun/runner.rs` | Edit | Remove budget logic, redesign needs_input flow, rename references |
| `src/autorun/state.rs` | Edit | Rename `claude_pid`/`claude_pgid` fields |
| `src/autorun/telegram.rs` | Edit | Remove cost display from messages |
| `src/chat/sessions.rs` | Edit | Replace Claude CLI with Cursor CLI, remove session resume |
| `src/chat/daemon.rs` | Edit | Rename references |
| `src/commands/init.rs` | **Rewrite** | Change from `.claude/` to `.cursor/`, CLAUDE.md to AGENTS.md, new hooks format |
| `src/commands/mono.rs` | Minor edit | Update print messages |
| `src/errors.rs` | Edit | Rename error variants |
| `src/bin/main.rs` | Edit | Update COMMANDS_HELP text |
| `src/bin/autorun.rs` | No change | Just calls runner::run |
| `src/bin/chat.rs` | No change | Just calls daemon::run |
| `Cargo.toml` | Edit | Update exclude list, optional name/description |
| `.cursor/skills/rex-model-router/SKILL.md` | **Heavy edit** | Replace all haiku/sonnet/opus model names, update Agent tool -> Task tool, update rationale |
| `.cursor/skills/rex-operator/SKILL.md` | **Heavy edit** | Replace model names, Agent tool -> Task tool, update dispatch tables |
| `.cursor/skills/rex-planning-tasks/SKILL.md` | Edit | Replace `--agent-model sonnet`/`opus` in all example CLI commands |
| `.cursor/skills/rex-chat/SKILL.md` | Minor edit | Replace `claude_pgid` with `agent_pgid` |
| `.cursor/skills/rex-publish-to-git/SKILL.md` | Minor edit | Replace `Co-Authored-By: Claude` in commit templates |
| `.cursor/skills/find-skills/SKILL.md` | Minor edit | Replace `awesome-claude-skills` reference |
| `.cursor/hooks/commit-and-push.sh` | Already done | Uses `$CURSOR_PROJECT_DIR` |
| `.cursor/hooks.json` | Already done | Uses Cursor format |
| `AGENTS.md` | Already done | Replaces `CLAUDE.md` |

---

## 12. Implementation Order

1. **Audit Cursor CLI output** -- run `cursor agent -p --output-format json "hello"` to capture actual JSON structure
2. **Update types** -- adapt `CursorOutput` struct to match real output
3. **Rewrite cursor_agent.rs** -- the core spawn/await module
4. **Update runner.rs** -- remove budget logic, redesign needs_input flow
5. **Update chat/sessions.rs** -- same CLI changes
6. **Rewrite init.rs** -- `.claude/` to `.cursor/` scaffolding
7. **Update errors.rs** -- rename variants
8. **Update state.rs** -- rename fields
9. **Update skills** -- model router and operator
10. **Update help text** -- main.rs COMMANDS_HELP
11. **Integration test** -- full autorun loop
