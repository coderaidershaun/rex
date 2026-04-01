# Objectives

Objectives sit beneath a milestone and represent the strategic outcomes needed to reach it. They answer "what must be true for this milestone to be achieved?" Each objective should be a coherent, scoped goal — not a single action, but not so broad that it could be a milestone itself.

Objectives carry intent and can have success criteria. They decompose the *why* of a milestone into addressable chunks.

**Example:** "Users can securely reset their passwords via email."

## JSON Schema

```json
{
  "id": "o-password-reset",
  "milestone_id": "m-auth-system",
  "title": "Users can securely reset their passwords via email",
  "description": "Token-based reset flow with expiry, rate limiting, and audit logging",
  "status": "not-started",
  "references": ["docs/auth-spec.md#password-reset"],
  "outputs": ["src/auth/reset.rs"],
  "checklist": [
    { "id": "c1", "item": "Token generation works", "done": false },
    { "id": "c2", "item": "Email delivery confirmed", "done": false }
  ],
  "tasks": ["t-token-endpoint", "t-email-template", "t-reset-ui"],
  "upstream": [],
  "downstream": ["o-oauth-setup"]
}
```

## CLI Commands

### Create an objective

```bash
rex objective upsert \
  --id o-password-reset \
  --milestone m-auth-system \
  --title "Users can securely reset their passwords via email" \
  --description "Token-based reset flow with expiry, rate limiting, and audit logging" \
  --add-reference "docs/auth-spec.md#password-reset" \
  --add-checklist "c1:Token generation works" \
  --add-checklist "c2:Email delivery confirmed"
```

The parent milestone's `objectives` list is automatically updated to include `o-password-reset`.

### Update an objective

```bash
# Update status
rex objective upsert --id o-password-reset --status in-progress

# Re-parent to a different milestone
rex objective upsert --id o-password-reset --milestone m-auth-v2

# Add dependency on another objective
rex objective upsert --id o-oauth-setup --add-upstream o-password-reset
```

### Get an objective

```bash
rex objective get o-password-reset
```

### List objectives

```bash
# All objectives
rex objective list

# Filter by milestone
rex objective list --milestone m-auth-system

# Filter by status
rex objective list --status not-started

# Both filters
rex objective list --milestone m-auth-system --status completed
```

### Remove an objective

```bash
rex objective remove o-password-reset
```

Removes the objective from its parent milestone's `objectives` list, cleans up upstream/downstream references in sibling objectives, and warns about any orphaned tasks.

## List Modification Flags

Same flags as milestones — see [milestones.md](milestones.md#list-modification-flags).
