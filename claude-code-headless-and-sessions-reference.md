# Claude Code: Headless Mode, Session Management & Persistent CLI Reference

> Compiled from official Anthropic documentation and community guides (2026-04-03)

---

## Table of Contents

1. [Programmatic / Headless Usage (`-p` flag)](#1-programmatic--headless-usage--p-flag)
2. [Bare Mode (`--bare`)](#2-bare-mode---bare)
3. [Session Management](#3-session-management)
4. [Session Naming & Tagging](#4-session-naming--tagging)
5. [Continuing & Resuming Sessions](#5-continuing--resuming-sessions)
6. [Forking Sessions](#6-forking-sessions)
7. [Output Formats & Streaming](#7-output-formats--streaming)
8. [Tool Permissions in Headless Mode](#8-tool-permissions-in-headless-mode)
9. [System Prompt Customization](#9-system-prompt-customization)
10. [Git Worktrees for Parallel Sessions](#10-git-worktrees-for-parallel-sessions)
11. [Remote Control (Mobile/Browser Access)](#11-remote-control)
12. [Agent SDK (Python & TypeScript)](#12-agent-sdk-python--typescript)
13. [Auto-Resume Pattern (Community)](#13-auto-resume-pattern-community)
14. [CI/CD Integration Patterns](#14-cicd-integration-patterns)
15. [Complete CLI Flags Reference](#15-complete-cli-flags-reference)
16. [Environment Variables](#16-environment-variables)

---

## 1. Programmatic / Headless Usage (`-p` flag)

The `-p` (or `--print`) flag runs Claude Code non-interactively. It processes the prompt, executes tools, and exits. This is the foundation of all headless/scripted usage.

```bash
# Basic headless query
claude -p "What does the auth module do?"

# Process piped content
cat logs.txt | claude -p "explain these errors"

# Pipe a PR diff for review
gh pr diff 42 | claude -p "Review this PR for security issues"
```

**Key behavior:**
- Claude runs the full agent loop (reading files, running commands, editing) then exits
- All CLI options work with `-p`
- User-invoked skills (`/commit`, `/review-pr`) are NOT available — describe the task instead
- Sessions created by `-p` don't appear in the `/resume` interactive picker, but CAN be resumed by session ID

---

## 2. Bare Mode (`--bare`)

Bare mode skips auto-discovery of hooks, skills, plugins, MCP servers, auto memory, and CLAUDE.md. This makes startup faster and ensures reproducible results across machines.

```bash
claude --bare -p "Summarize this file" --allowedTools "Read"
```

**What bare mode provides:**
- Access to Bash, file read, and file edit tools
- No hooks, plugins, skills, MCP, auto memory, or CLAUDE.md loaded
- Skips OAuth and keychain reads (use `ANTHROPIC_API_KEY` or `apiKeyHelper` in `--settings`)

**Loading context explicitly in bare mode:**

| To load                  | Use                                                  |
|--------------------------|------------------------------------------------------|
| System prompt additions  | `--append-system-prompt`, `--append-system-prompt-file` |
| Settings                 | `--settings <file-or-json>`                          |
| MCP servers              | `--mcp-config <file-or-json>`                        |
| Custom agents            | `--agents <json>`                                    |
| A plugin directory       | `--plugin-dir <path>`                                |

**Recommended:** `--bare` is the recommended mode for scripted and SDK calls, and will become the default for `-p` in a future release.

---

## 3. Session Management

A session is the full conversation history accumulated while Claude works — prompts, tool calls, tool results, and responses. Sessions are written to disk automatically so you can return to them later.

**Sessions persist the conversation, not the filesystem.** For file snapshots, use file checkpointing.

**Storage location:** `~/.claude/projects/<encoded-cwd>/*.jsonl` where `<encoded-cwd>` is the absolute working directory with every non-alphanumeric character replaced by `-`.

### Approaches by Use Case

| Use case | What to use |
|----------|------------|
| One-shot task, no follow-up | Single `query()` call or `claude -p` |
| Multi-turn chat in one process | `ClaudeSDKClient` (Python) or `continue: true` (TypeScript) |
| Pick up after process restart | `--continue` (resumes most recent session in directory) |
| Resume a specific past session | Capture session ID, use `--resume <id>` |
| Try alternative approach without losing original | Fork the session |
| Stateless, nothing written to disk (TS only) | `persistSession: false` |

---

## 4. Session Naming & Tagging

Name sessions to find them later. This is essential when managing multiple sessions.

### At startup:
```bash
claude -n "auth-refactor"
# or
claude --name "auth-refactor"
```

### During a session:
```
/rename auth-refactor
```

### From the `/resume` picker:
Navigate to a session and press `R` to rename it.

The name appears in:
- The `/resume` picker
- The terminal title bar
- Resume commands: `claude --resume auth-refactor`

---

## 5. Continuing & Resuming Sessions

### Continue (most recent session)

```bash
# Interactive: continue most recent conversation in current directory
claude -c
claude --continue

# Headless: continue most recent and send new prompt
claude -c -p "Now focus on the database queries"
```

### Resume (specific session by ID or name)

```bash
# By session name
claude -r "auth-refactor"
claude --resume "auth-refactor"

# By session ID
claude --resume "550e8400-e29b-41d4-a716-446655440000"

# Interactive picker (shows all sessions)
claude --resume

# Inside an active session
/resume
/resume auth-refactor
```

### Multi-step headless workflows

```bash
# Step 1: Initial task
claude -p "Review this codebase for performance issues"

# Step 2: Continue with follow-up
claude -p "Now focus on the database queries" --continue

# Step 3: Continue again
claude -p "Generate a summary of all issues found" --continue
```

### Capture session ID for later resume

```bash
session_id=$(claude -p "Start a review" --output-format json | jq -r '.session_id')
claude -p "Continue that review" --resume "$session_id"
```

### Explicit session ID

```bash
# Use a specific UUID as session ID
claude --session-id "550e8400-e29b-41d4-a716-446655440000"
```

### Resume from PR

```bash
# Resume sessions linked to a specific GitHub PR
claude --from-pr 123
claude --from-pr https://github.com/org/repo/pull/123
```

### Fork on resume (create new branch of conversation)

```bash
claude --resume abc123 --fork-session
```

### Disable session persistence

```bash
# Don't save session to disk (print mode only)
claude -p --no-session-persistence "query"
```

**Important:** If resume returns a fresh session instead of expected history, the most common cause is a **mismatched `cwd`** — sessions are stored per directory.

---

## 6. Forking Sessions

Forking creates a NEW session that starts with a copy of the original's history but diverges from that point. The original stays unchanged.

**Use fork to:**
- Try a different approach without losing the original thread
- Explore alternative solutions from a common analysis point

**Important:** Forking branches the conversation history, NOT the filesystem. File changes are real and visible to all sessions in the same directory.

### CLI fork:
```bash
claude --resume <session-id> --fork-session
```

### Python SDK fork:
```python
async for message in query(
    prompt="Try the OAuth2 approach instead",
    options=ClaudeAgentOptions(
        resume=original_session_id,
        fork=True,
        allowed_tools=["Read", "Edit", "Bash"],
    ),
):
    if isinstance(message, ResultMessage):
        forked_id = message.session_id
```

### TypeScript SDK fork:
```typescript
for await (const message of query({
  prompt: "Try the OAuth2 approach instead",
  options: {
    resume: originalSessionId,
    fork: true,
    allowedTools: ["Read", "Edit", "Bash"],
  }
})) {
  if (message.type === "result") {
    forkedId = message.sessionId;
  }
}
```

---

## 7. Output Formats & Streaming

### Output format options (`--output-format`)

| Format | Description |
|--------|------------|
| `text` (default) | Plain text output |
| `json` | Structured JSON with result, session ID, and metadata |
| `stream-json` | Newline-delimited JSON for real-time streaming |

### JSON output
```bash
claude -p "Summarize this project" --output-format json
# Result is in the `result` field
claude -p "Summarize" --output-format json | jq -r '.result'
```

### Structured output with JSON Schema
```bash
claude -p "Extract function names from auth.py" \
  --output-format json \
  --json-schema '{"type":"object","properties":{"functions":{"type":"array","items":{"type":"string"}}},"required":["functions"]}'
# Result is in the `structured_output` field
```

### Streaming
```bash
# Full streaming with partial messages
claude -p "Explain recursion" --output-format stream-json --verbose --include-partial-messages

# Filter for just text deltas
claude -p "Write a poem" --output-format stream-json --verbose --include-partial-messages | \
  jq -rj 'select(.type == "stream_event" and .event.delta.type? == "text_delta") | .event.delta.text'
```

### Input format for bidirectional streaming
```bash
claude -p --input-format stream-json --output-format stream-json
```

### API retry events in stream
When an API request fails with a retryable error, Claude emits a `system/api_retry` event:

| Field | Type | Description |
|-------|------|-------------|
| `type` | `"system"` | message type |
| `subtype` | `"api_retry"` | retry event identifier |
| `attempt` | integer | current attempt number (starts at 1) |
| `max_retries` | integer | total retries permitted |
| `retry_delay_ms` | integer | ms until next attempt |
| `error_status` | int/null | HTTP status code |
| `error` | string | error category |

---

## 8. Tool Permissions in Headless Mode

### Allow specific tools (`--allowedTools`)
```bash
claude -p "Run tests and fix failures" \
  --allowedTools "Bash,Read,Edit"
```

### Permission rule syntax with prefix matching
```bash
# The trailing ` *` enables prefix matching
# Space before * is important!
claude -p "Create a commit" \
  --allowedTools "Bash(git diff *),Bash(git log *),Bash(git status *),Bash(git commit *)"
```

### Disallow tools (`--disallowedTools`)
```bash
claude -p "Analyze code" --disallowedTools "Edit,Bash"
```

### Restrict available tools (`--tools`)
```bash
# Only allow Bash, Edit, Read
claude --tools "Bash,Edit,Read"

# Disable all tools
claude --tools ""
```

### Permission prompt tool (MCP-based approval)
```bash
claude -p --permission-prompt-tool mcp_auth_tool "query"
```

### Skip all permission prompts (dangerous)
```bash
claude --dangerously-skip-permissions
# or
claude --permission-mode bypassPermissions
```

---

## 9. System Prompt Customization

### Append to default prompt (recommended)
```bash
claude --append-system-prompt "Always use TypeScript"
claude --append-system-prompt-file ./style-rules.txt
```

### Replace entire prompt
```bash
claude --system-prompt "You are a Python expert"
claude --system-prompt-file ./custom-prompt.txt
```

### Combine with piped input
```bash
gh pr diff "$1" | claude -p \
  --append-system-prompt "You are a security engineer. Review for vulnerabilities." \
  --output-format json
```

**Note:** `--system-prompt` and `--system-prompt-file` are mutually exclusive. Append flags can be combined with either.

---

## 10. Git Worktrees for Parallel Sessions

Worktrees give each Claude session its own copy of the codebase so changes don't collide.

### Create and start in a worktree
```bash
# Named worktree
claude --worktree feature-auth
claude -w feature-auth

# Auto-generated name
claude --worktree
claude -w

# With tmux pane
claude -w feature-auth --tmux
```

### Worktree details
- Created at `<repo>/.claude/worktrees/<name>`
- Branches from `origin/HEAD` (the remote's default branch)
- Branch named `worktree-<name>`
- Add `.claude/worktrees/` to `.gitignore`

### Re-sync base branch
```bash
git remote set-head origin -a
```

### Subagent worktrees
Configure `isolation: worktree` in agent frontmatter for automatic parallel isolation.

### Worktree cleanup
- **No changes:** auto-removed on exit
- **Changes exist:** Claude prompts to keep or remove
- Orphaned subagent worktrees are auto-cleaned after `cleanupPeriodDays`

### Copy gitignored files (`.worktreeinclude`)
Create `.worktreeinclude` in project root using `.gitignore` syntax:
```
.env
.env.local
```

---

## 11. Remote Control

Control a local Claude Code session from your phone, tablet, or any browser.

### Start Remote Control

```bash
# Server mode (no local interactive session)
claude remote-control
claude remote-control --name "My Project"
claude remote-control --spawn worktree --capacity 32

# Interactive session with remote access
claude --remote-control
claude --rc "My Project"

# From existing session
/remote-control
/rc My Project
```

### Server mode flags

| Flag | Description |
|------|-------------|
| `--name "My Project"` | Custom session title |
| `--spawn <mode>` | `same-dir` (default) or `worktree` for concurrent sessions |
| `--capacity <N>` | Max concurrent sessions (default: 32) |
| `--verbose` | Show detailed logs |
| `--sandbox` / `--no-sandbox` | Enable/disable sandboxing |

### Enable for all sessions
Use `/config` → "Enable Remote Control for all sessions" → `true`

---

## 12. Agent SDK (Python & TypeScript)

The Agent SDK gives programmatic control with the same tools and agent loop as Claude Code.

### Python: Multi-turn with ClaudeSDKClient
```python
import asyncio
from claude_agent_sdk import ClaudeSDKClient, ClaudeAgentOptions

async def main():
    options = ClaudeAgentOptions(
        allowed_tools=["Read", "Edit", "Glob", "Grep"],
    )
    async with ClaudeSDKClient(options=options) as client:
        # First query - client captures session ID internally
        await client.query("Analyze the auth module")
        async for message in client.receive_response():
            print_response(message)

        # Second query - automatically continues same session
        await client.query("Now refactor it to use JWT")
        async for message in client.receive_response():
            print_response(message)

asyncio.run(main())
```

### Python: Capture session ID
```python
from claude_agent_sdk import query, ClaudeAgentOptions, ResultMessage

async def main():
    session_id = None
    async for message in query(
        prompt="Analyze the auth module",
        options=ClaudeAgentOptions(allowed_tools=["Read", "Glob", "Grep"]),
    ):
        if isinstance(message, ResultMessage):
            session_id = message.session_id
    return session_id
```

### Python: Resume by ID
```python
async for message in query(
    prompt="Now implement the refactoring you suggested",
    options=ClaudeAgentOptions(
        resume=session_id,
        allowed_tools=["Read", "Edit", "Write", "Glob", "Grep"],
    ),
):
    ...
```

### TypeScript: Continue sessions
```typescript
import { query } from "@anthropic-ai/claude-agent-sdk";

// First query - creates new session
for await (const message of query({
  prompt: "Analyze the auth module",
  options: { allowedTools: ["Read", "Glob", "Grep"] }
})) { ... }

// Second query - continue: true resumes most recent session
for await (const message of query({
  prompt: "Now refactor it to use JWT",
  options: {
    continue: true,
    allowedTools: ["Read", "Edit", "Write", "Glob", "Grep"]
  }
})) { ... }
```

### Install
```bash
# Python
pip install claude-agent-sdk

# TypeScript
npm install @anthropic-ai/claude-agent-sdk
```

### Authentication
```bash
export ANTHROPIC_API_KEY=your-api-key

# Or third-party providers:
export CLAUDE_CODE_USE_BEDROCK=1   # Amazon Bedrock
export CLAUDE_CODE_USE_VERTEX=1    # Google Vertex AI
export CLAUDE_CODE_USE_FOUNDRY=1   # Microsoft Azure
```

---

## 13. Auto-Resume Pattern (Community)

A community pattern for automatically resuming sessions using hooks and shell functions.

### Step 1: SessionEnd Hook (`~/.claude/settings.json`)
```json
{
  "hooks": {
    "SessionEnd": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "jq -r '\"echo \\\"claude --resume \" + .session_id + \"\\\" > \" + .cwd + \"/.claude_session\"' | sh"
          }
        ]
      }
    ]
  }
}
```

### Step 2: Shell function (`~/.zshrc` or `~/.bashrc`)
```bash
c() {
  if [ -f .claude_session ]; then
    local cmd
    cmd=$(cat .claude_session)
    rm -f .claude_session
    eval "$cmd"
  else
    claude "$@"
  fi
}
```

### How it works:
1. Exit Claude Code → hook saves session ID to `.claude_session`
2. Run `c` from same directory → resumes where you left off
3. Run `c` again (or with args) → starts fresh session (file was consumed)
4. Multiple projects = multiple sessions, no conflicts

---

## 14. CI/CD Integration Patterns

### Basic CI/CD usage
```bash
# Code review in CI
claude --bare -p "Review this PR for issues" \
  --allowedTools "Read,Glob,Grep" \
  --output-format json \
  --max-turns 10
```

### With budget limits
```bash
claude -p "Fix all lint errors" \
  --max-budget-usd 5.00 \
  --max-turns 20 \
  --allowedTools "Bash,Read,Edit"
```

### With model selection and fallback
```bash
claude -p "Generate API docs" \
  --model sonnet \
  --fallback-model sonnet \
  --output-format json
```

### Effort level control
```bash
claude --effort low -p "Quick lint check"
claude --effort high -p "Deep security review"
claude --effort max -p "Full architectural analysis"  # Opus only
```

### GitHub Actions pattern
```yaml
- name: Claude Code Review
  run: |
    claude --bare -p "Review the changes in this PR" \
      --allowedTools "Read,Glob,Grep,Bash(git diff *)" \
      --output-format json \
      --max-budget-usd 2.00
  env:
    ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
```

---

## 15. Complete CLI Flags Reference

### Session & Conversation Flags

| Flag | Description |
|------|-------------|
| `-p`, `--print` | Non-interactive mode (headless) |
| `-c`, `--continue` | Continue most recent conversation |
| `-r`, `--resume` | Resume specific session by ID or name |
| `-n`, `--name` | Set session display name |
| `--session-id` | Use specific UUID as session ID |
| `--fork-session` | Create new session from resumed conversation |
| `--from-pr` | Resume sessions linked to a GitHub PR |
| `--no-session-persistence` | Don't save session to disk |

### Output & Format Flags

| Flag | Description |
|------|-------------|
| `--output-format` | `text`, `json`, or `stream-json` |
| `--input-format` | `text` or `stream-json` |
| `--json-schema` | JSON Schema for structured output |
| `--verbose` | Full turn-by-turn output |
| `--include-partial-messages` | Include streaming partials |
| `--include-hook-events` | Include hook lifecycle events |
| `--replay-user-messages` | Re-emit user messages on stdout |

### Tool & Permission Flags

| Flag | Description |
|------|-------------|
| `--allowedTools` | Tools that execute without prompting |
| `--disallowedTools` | Tools removed from context |
| `--tools` | Restrict available tools |
| `--permission-mode` | `default`, `acceptEdits`, `plan`, `auto`, `dontAsk`, `bypassPermissions` |
| `--dangerously-skip-permissions` | Skip all permission prompts |
| `--permission-prompt-tool` | MCP tool for permission handling |

### Model & Performance Flags

| Flag | Description |
|------|-------------|
| `--model` | Set model (`sonnet`, `opus`, or full name) |
| `--fallback-model` | Fallback model when overloaded |
| `--effort` | `low`, `medium`, `high`, `max` |
| `--max-turns` | Limit agentic turns |
| `--max-budget-usd` | Maximum dollar spend |

### Environment & Config Flags

| Flag | Description |
|------|-------------|
| `--bare` | Skip all auto-discovery |
| `--settings` | Load settings from JSON file/string |
| `--setting-sources` | Which sources to load (`user,project,local`) |
| `--mcp-config` | Load MCP servers from JSON |
| `--strict-mcp-config` | Only use specified MCP config |
| `--agents` | Define subagents via JSON |
| `--agent` | Specify agent for session |
| `--plugin-dir` | Load plugins from directory |
| `--add-dir` | Additional working directories |

### System Prompt Flags

| Flag | Description |
|------|-------------|
| `--system-prompt` | Replace entire system prompt |
| `--system-prompt-file` | Replace with file contents |
| `--append-system-prompt` | Append to default prompt |
| `--append-system-prompt-file` | Append file contents |

### Worktree & Parallel Flags

| Flag | Description |
|------|-------------|
| `-w`, `--worktree` | Start in isolated git worktree |
| `--tmux` | Create tmux session for worktree |
| `--teammate-mode` | `auto`, `in-process`, or `tmux` |

### Remote & Web Flags

| Flag | Description |
|------|-------------|
| `--remote` | Create web session on claude.ai |
| `--remote-control`, `--rc` | Interactive session with remote access |
| `--teleport` | Resume web session locally |

### Debug Flags

| Flag | Description |
|------|-------------|
| `--debug` | Enable debug mode with category filtering |
| `--debug-file` | Write debug logs to file |

---

## 16. Environment Variables

Key environment variables for headless/programmatic usage:

| Variable | Description |
|----------|-------------|
| `ANTHROPIC_API_KEY` | API key for authentication |
| `CLAUDE_CODE_USE_BEDROCK` | Use Amazon Bedrock |
| `CLAUDE_CODE_USE_VERTEX` | Use Google Vertex AI |
| `CLAUDE_CODE_USE_FOUNDRY` | Use Microsoft Azure |
| `CLAUDE_CODE_SIMPLE` | Set by `--bare` mode |
| `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS` | Enable agent teams |
| `CLAUDE_CODE_DEBUG_LOGS_DIR` | Directory for debug logs |

---

## Quick Reference: Common Patterns

```bash
# One-shot headless query
claude -p "Fix the bug in auth.py" --allowedTools "Read,Edit,Bash"

# Headless with JSON output
claude -p "Summarize project" --output-format json | jq -r '.result'

# Multi-step headless conversation
claude -p "Analyze code" --output-format json > /tmp/step1.json
session_id=$(jq -r '.session_id' /tmp/step1.json)
claude -p "Fix the issues you found" --resume "$session_id" --allowedTools "Edit,Bash"

# Named session for later resume
claude -n "feature-auth" "Start working on auth refactor"
# ... later ...
claude --resume "feature-auth" "Continue where we left off"

# Parallel sessions with worktrees
claude -w feature-auth "Implement auth" &
claude -w bugfix-123 "Fix the login bug" &
wait

# Bare mode for CI/CD
claude --bare -p "Review code" --allowedTools "Read,Grep" --max-budget-usd 2.00

# Remote control from phone
claude remote-control --name "My Project" --spawn worktree

# Fork to explore alternative
session_id=$(claude -p "Analyze options" --output-format json | jq -r '.session_id')
claude -p "Try approach A" --resume "$session_id" --fork-session
claude -p "Try approach B" --resume "$session_id" --fork-session
```

---

*Sources: code.claude.com/docs, platform.claude.com/docs, community guides*
