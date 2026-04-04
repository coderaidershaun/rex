# Telegram Commands

Rex uses two separate Telegram bots — one for chat, one for autorun. Each bot has its own token and handles only its own messages.

## Environment Variables

| Variable | Used by | Description |
|----------|---------|-------------|
| `REX_TELEGRAM_CHAT_ID` | Both | Your Telegram chat ID (shared across both bots). |
| `REX_AUTOCHAT_TELEGRAM_BOT_TOKEN` | `rex-chat` | Bot token for the chat bot. |
| `REX_AUTORUN_TELEGRAM_BOT_TOKEN` | `rex-autorun` | Bot token for the autorun bot. |

---

## Chat Bot (`rex-chat`)

The chat bot provides a project dashboard and Claude-powered chat sessions.

### Text Commands

| Command | Description |
|---------|-------------|
| `/start` | Show the project dashboard with inline buttons. |
| `/menu` | Same as `/start`. |
| Any other text | Shows the project dashboard. |

### Inline Buttons

These appear on messages sent by the bot. Tap to interact.

| Button | Description |
|--------|-------------|
| **Chat: \<project\>** | Start a chat session for a project. Sends a ForceReply prompt — type your message below it. |
| **Start: \<project\>** | Start an autorun for a project (runs `rex-autorun` in the background). |
| **Status: \<project\>** | Show autorun stats (uptime, cost, invocations). |
| **Stop: \<project\>** | Stop a running autorun (sends SIGTERM). |
| **Reply** | Continue a chat conversation. Sends a new ForceReply prompt. |
| **Menu** | Return to the project dashboard. |

### Reply-to Routing

Reply directly to any chat response message to continue the conversation with that project. The bot routes your reply to the correct Claude session automatically.

---

## Autorun Bot (`rex-autorun`)

The autorun bot relays status updates and questions from the headless autopilot.

### Text Commands

| Command | Description |
|---------|-------------|
| `/kill <project-id>` | Terminate the autorun for the specified project. |
| `/kill` | Kill the autorun that is currently polling (when only one is running). |
| `/query <project-id>` | Show live stats for the specified project (uptime, cost, context usage, other running autoruns). |
| `/query` | Show stats for the autorun that is currently polling. |

### Inline Buttons

These appear on messages sent by the autorun bot (completion notifications, questions, etc.).

| Button | Description |
|--------|-------------|
| **Reply** | Send a reply to a pending question. Sends a ForceReply prompt. |
| **Stats** | Show live autorun stats. |
| **Kill** | Stop this autorun. |

### Reply-to Routing

When the autorun asks a question (e.g. onboarding input, design acceptance), it sends a ForceReply message. Reply directly to that message to provide your answer. The autorun resumes with your reply.

### Unprocessed Messages

If you send a bare message (not a reply, not a command), the bot responds with:

```
Unprocessed — please reply to a specific message, or use:
  /kill <project-id>
  /query <project-id>

Active autoruns:
  project-a
  project-b
```

---

## Multi-Autorun Behaviour

Multiple autoruns can share the same bot token. A cooperative triage system ensures messages reach the correct instance:

- **File lock**: Only one autorun polls Telegram at a time. Others check their inbox.
- **Registry**: Each autorun registers its PID and expected reply message. The polling autorun routes cross-project messages via per-project inbox files.
- **Reply routing**: When you reply to a specific autorun's question, the polling instance checks the registry and forwards the reply to the correct autorun's inbox.
- **Commands with project-id**: `/kill my-project` and `/query my-project` are routed to the named project regardless of which autorun is polling.
