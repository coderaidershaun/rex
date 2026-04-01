# Rex Operator

The operator is the heartbeat of the rex harness. It processes one work item per invocation, dispatching the right agent(s) and recording what happened.

## Sequence of Operations

```
1. Get active project        →  rex project get-active
2. Check lock status          →  If locked: true, STOP immediately
3. Get next work item         →  rex project next-item
4. Mark item in-progress      →  rex project update-status <item> in-progress
5. Get recent history         →  rex history get-recent
6. Prepare agent dispatch     →  Build prompt from item config
7. Dispatch agent(s)          →  Agent tool (BLOCKING — never background)
8. Check agent response       →  Respect "do not mark complete" signals
9. Record history             →  rex history insert-recent ...
10. Mark item complete        →  rex project update-status <item> completed
11. Manage history            →  Dispatch agent with rex-manage-history skill
12. Stop                      →  Report what was done and terminate
```

## CLI Commands Used

| Step | Command | Purpose |
|------|---------|---------|
| 1 | `rex project get-active` | Get active project fields |
| 3 | `rex project next-item` | Get next actionable item from project-status.json |
| 4, 10 | `rex project update-status <item> <status>` | Mark items in-progress or completed |
| 5 | `rex history get-recent` | Get recent history for agent context |
| 9 | `rex history insert-recent --id ... --timestamp ... --summary ... --entity ... --file ...` | Record what was done |

## Agent Dispatch

The work item's `agent` object controls how agents are dispatched:

```json
{
  "count": 1,          // Number of agents to spawn
  "effort": "high",    // Reasoning depth
  "model": "opus",     // Model to use
  "skills": ["..."]    // Skill(s) agents should invoke
}
```

### Model Mapping

| Item value | Agent model param |
|------------|-------------------|
| `"opus"` | `opus` |
| `"sonnet"` | `sonnet` |
| `"haiku"` | `haiku` |

### Effort Mapping

| Item value | Instruction to agent |
|------------|---------------------|
| `"medium"` | Apply moderate reasoning depth |
| `"high"` | Think carefully and thoroughly |
| `"max"` | Think very deeply, consider all angles |
| `"ultrathink"` | Think extremely deeply, exhaust every consideration |

### Single vs Multi-Agent

- **count = 1**: One agent, one dispatch. Straightforward.
- **count > 1**: Spawn N worker agents in parallel, each focusing on a different portion. Then one coordinator agent synthesizes their work into the final output(s).

**All dispatches are blocking.** The operator waits for every agent to complete before proceeding. This is critical for headless operation.

## History Management

After each item completes, the operator invokes an agent with the `rex-manage-history` skill to keep the recent history section at 3 entries or fewer. Older entries are summarised and moved to the archived section.

## Rules

- One item per invocation — no looping
- CLI only for data mutations — never write JSON files directly
- Blocking dispatch — never run agents in background
- Respect the lock — if locked, stop immediately
- Respect agent responses — if an agent says not to mark complete, don't
- Pass full context — agents receive the project object and recent history
