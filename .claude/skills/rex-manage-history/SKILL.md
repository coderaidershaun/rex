---
name: rex-manage-history
description: Condense the rex project history — move any entries beyond the 3 most recent into the archived section with summarised content. Use this skill when the operator finishes processing an item, when the recent history section is getting long, when you need to compact or archive old history entries, or when anyone says "manage history", "condense history", "archive old entries", or "clean up history."
disable-model-invocation: false
user-invocable: false
---

# History Management: Condense and Archive

Your job is simple: keep the `recent` section of project history lean. The recent section should never have more than 3 entries. Anything older gets summarised and moved to `archived`.

---

## Step 1: Read all history

```bash
rex-cli history list
```

This returns the full `history.json` contents — both `recent` and `archived` arrays.

Parse the JSON output. Count the entries in `recent`.

- If `recent` has **3 or fewer entries**: you're done. No work needed. Report that history is already clean and stop.
- If `recent` has **more than 3 entries**: continue to step 2.

---

## Step 2: Identify entries to archive

Sort `recent` entries by timestamp (oldest first). The **3 most recent** entries stay. Everything else gets archived.

For example, if recent has 5 entries, the 2 oldest entries need to be moved to archived.

---

## Step 3: Summarise and archive

For each entry being archived:

1. **Write a compacted summary** — Take the entry's `summary` and condense it further if it's verbose. The archived summary should be a single sentence capturing what was accomplished. Combine entities and files into the compacted entry.

2. **Insert the compacted entry:**

```bash
rex-cli history insert-compacted \
  --id "compact-<original-id>" \
  --timestamp "<original-timestamp>" \
  --summary "<condensed summary>"  \
  --entity <entity-1> \
  --entity <entity-2>
```

Include `--entity` flags for any entities from the original entry. You can omit `--file` flags in the compacted version to keep it lean, but preserve entities since they're useful for tracking what was affected.

3. **Remove the original from recent:**

```bash
rex-cli history remove-from-recent <original-id>
```

Process entries one at a time: insert-compacted, then remove-from-recent, then move to the next entry. This prevents data loss if something fails partway through.

---

## Step 4: Verify

Run `rex-cli history get-recent` to confirm that recent now has 3 or fewer entries.

Report what you did: how many entries were archived, and what they covered.

---

## Rules

- Never delete history — always archive before removing from recent.
- Never modify the content of the 3 most recent entries.
- Keep archived summaries concise — one sentence, no fluff.
- If multiple old entries cover the same topic, you may combine them into a single compacted entry with a broader summary, but this is optional. One-to-one archiving is fine.
- All mutations go through CLI commands. Never write `history.json` directly.
