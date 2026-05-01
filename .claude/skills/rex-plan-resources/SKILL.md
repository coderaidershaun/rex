---
name: rex-plan-resources
description: Interview the user about external resources they've handed in alongside the project — credentials, vendor docs, datasets, reference code, mock servers, fixtures, schema registries, anything not in the source. Ask how each relates to the inputs. Output = a resource manifest future agents read so they know what to use, when, and what to avoid. Use when user says "catalogue resources", "what resources do we have", "tell agents about the assets", or pipeline orchestrator dispatches `rex-plan-resources` step.
disable-model-invocation: false
user-invocable: true
---

# rex-plan-resources

Interview the user about the resources they've handed in. Find out what each one is, how it maps to the inputs, when to use it, what gotchas it carries. Output = a manifest that future agents read at orient-time so they don't re-derive any of this.

I/O contract lives in `rex-utils-task-request`. Reads inputs given. Writes resource manifest to output path given.

**Dialogue, not monologue.** One question per turn. Recommend an answer each time. Code lookup > user lookup.

## What good resource cataloguing is

- **Resource ≠ input.** Inputs are the planning docs (specs, scenarios, plan files). Resources are the external assets the user provides for execution — keys, datasets, PDFs, fixtures, mock servers, vendor docs.
- **Mapped to the work.** Each resource has at least one chunk / scenario / context that uses it. If nothing uses it, surface that.
- **Usage-ready.** Future agent reads the manifest and knows exactly when to reach for the resource and how to invoke it.
- **Gotchas surfaced.** Rate limits, freshness windows, format quirks, auth scope — better in the manifest than discovered mid-execution.
- **Living.** New resource handed in mid-project → run this skill again, append.

## Resource categories (to anchor the interview)

| Category | Examples | What to ask |
|----------|----------|-------------|
| Credentials | API keys, OAuth tokens, signing keys, certs | Scope? Rate limit? Rotation cadence? Where stored? |
| Vendor docs | Exchange API spec PDF, RFC, schema registry URL | Which contract / spec section relies on it? Authoritative or just reference? |
| Datasets | Sample order books, historical trades, fixture files | Which scenarios use it? Real data or synthetic? Freshness? |
| Reference code | Internal library, prior project, vendor SDK | Read-only reference, or wrap-and-use? License? Version pinned? |
| Mock servers | Fake exchange, recorded WebSocket replays | Which integration tests rely on it? How invoked? Lifetime per test? |
| Test fixtures | Recorded payloads, golden files, regression inputs | Which test suite consumes it? Regenerate cadence? |
| Internal services | Staging DB, internal API, ops dashboard | Auth path? Rate limit? Persistent state risk? |
| Domain artefacts | Glossaries, runbooks, incident postmortems | Which terms / decisions does it inform? |

Ask only about categories that apply. Skip the rest.

## The interview — what to extract per resource

For each resource the user names:

1. **What is it?** One line. Type + provenance.
2. **Where does it live?** Path / URL / env var / vault entry.
3. **Who provided it?** User / vendor / earlier session — affects refresh cadence.
4. **What does it provide that isn't elsewhere?** If duplicate, why keep both.
5. **Which inputs does it map to?** Scenarios? Specs? Bounded contexts?
6. **When should an agent use it?** Trigger condition. Don't bury this.
7. **How is it invoked?** Exact command / env var / API call.
8. **Gotchas?** Rate limit, expiry, format quirk, scope limit.
9. **Failure mode?** What does the agent do if it's down / expired / missing?

Most resources need 2–4 of these answered. Don't drag the user through all 9 if 4 cover it.

## Interview discipline

- **One question per turn.** Never wall-of-questions.
- **Recommend, don't blank-page.** "I'd recommend treating the Binance PDF as authoritative for trade-message format, since the spec references it — agree?" User confirms or corrects.
- **Code lookup > user lookup.** Don't ask "where's the API key?" — grep `.env.example`, find the var name. Confirm.
- **Update inline.** Each resource → that entry in the manifest written / refined immediately.
- **Stop when the manifest is usable.** Aim for full coverage of named resources, not exhaustive interrogation. ~3–6 questions per resource on average.

## Output shape

```md
# Resources — <project>

## <resource-id>
**What:** <one-line description>
**Where:** `<path / URL / env var>`
**Provided by:** <user / vendor / prior session>
**Maps to:** <inputs that depend on it — scenarios, specs, contexts>

**Use when:** <trigger condition>
**Invoke:** `<exact command / API call>`

**Gotchas:**
- <rate limit / expiry / quirk>

**On failure:** <what the agent does if down / expired / missing>

---

## <next-resource-id>
...
```

If a field doesn't apply, drop the line. Don't pad.

## Process

1. **Orient.** Read inputs. Find resources mentioned but not catalogued (referenced env vars, URLs in specs, fixtures in test dirs, deps in `Cargo.toml` flagged as external).
2. **List candidates.** Pre-fill the manifest with what you can derive without asking.
3. **Interview.** One question at a time, with a recommendation, until each candidate has the fields it needs.
4. **Ask what's missing.** "Anything else handed in we should know about?" — catches resources you couldn't see.
5. **Cross-check coverage.** Every resource maps to at least one input. Every input that needs an external resource has one assigned.
6. **Publish.** Manifest to output path given by task envelope.

## Anti-patterns

| Bad | Why | Fix |
|-----|-----|-----|
| Wall of questions | User freezes | One per turn. Wait |
| Cataloguing everything in `Cargo.toml` | Standard deps aren't resources | Only external assets requiring user knowledge |
| Asking what `.env.example` says | Read it | Confirm only ambiguities |
| Resource without "Maps to" field | Future agent can't tell when to use it | Always link to inputs. If it maps to nothing, surface that |
| Vague "Use when: as needed" | Tells agent nothing | Concrete trigger: "during integration tests against real Binance" |
| Storing actual secrets in the manifest | Manifest is committed. Secrets aren't | Reference name only (`BINANCE_API_KEY`). Vault path if relevant |
| Skipping gotchas because "obvious" | Future agent isn't this user | Surface rate limits, scopes, expiries |
| One mega-resource entry | Agents grep by id. Mega-entries hide info | One entry per resource |
| Pad manifest with empty fields | Reader skims. Pad blocks skim | Drop empty fields |
| Forgetting on-failure behaviour | Agent stuck mid-execution | Always say what to do if resource unreachable |

## Hand-off

Resource manifest feeds:

- **Every later agent** — read at orient-time. Tells them what assets exist + how to use.
- **`rex-plan-scheduling`** — chunks that depend on a resource carry a ref to its id.
- **`rex-clean-uat`** — UAT setup section may depend on a resource being available.
- **`rex-plan-refinement`** — uncatalogued resource referenced in a spec → flagged.
- **Future runs of this skill** — append-only. New resources extend; existing ones get refined.

Without this manifest, agents either guess or stall on missing context. Catalogue once, reuse everywhere.
