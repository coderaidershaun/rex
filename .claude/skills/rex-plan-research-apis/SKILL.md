---
name: rex-plan-research-apis
description: Research external APIs / SDKs / libraries the project depends on. Maps endpoints, auth flow, request/response shapes, rate limits, quirks. Produces a research dossier + minimal spike snippet. Use when user says "research the API", "investigate library", "map the endpoints", "what does X API support", or pipeline orchestrator dispatches `rex-plan-research-apis` step.
disable-model-invocation: false
user-invocable: true
---

# rex-plan-research-apis

Take an API documentation request → produce a research dossier. Maps endpoints, auth, request/response, quirks, rate limits. Includes minimal spike snippet caller can run to confirm.

I/O contract = `rex-utils-task-request`. Read envelope first. Inputs name the API + scope (which endpoints / which capability). Outputs receive the dossier.

**Not a spike.** Don't hit prod. Doc-driven. Spike snippet = code only, not executed. Real probing → `rex-plan-spike`.

## What good API research is

- **Endpoint-complete.** Every endpoint relevant to scope is listed. Nav drawer (left sidebar in most API docs) is the canonical source.
- **Versioned.** API version pinned. URL pattern + version header captured. Breaking changes vs prior version noted.
- **Auth-explicit.** Exact scheme, header name, scope/permission required, rotation cadence, where the secret lives.
- **Shape-precise.** Request fields with types + units + constraints. Response fields the same. Error shape too.
- **Quirk-aware.** Rate limits, idempotency keys, pagination style, retry semantics, sandbox vs prod URL split.
- **Spike-ready.** One concrete request the caller can run to verify the doc. Curl + minimal Rust both fine.
- **Source-cited.** Every claim links to the doc page or section it came from. Future readers verify.

## Tooling — firecrawl preferred

Check first:

```bash
which firecrawl
```

Available → use it. Better than `WebFetch` for nav-driven docs (renders JS sidebars, follows internal links, extracts clean markdown).

| Want | Skill |
|------|-------|
| Map every URL on docs site | `firecrawl-map` |
| Pull one endpoint page | `firecrawl-scrape` |
| Bulk pull `/api/*` section | `firecrawl-crawl` |
| Auth flow behind login wall | `firecrawl-instruct` |
| Structured extract (every endpoint as JSON) | `firecrawl-agent` |

Not available → fall back to `WebFetch` + `WebSearch`. Slower. Some JS-rendered nav drawers won't expose links. Note in dossier when fallback used (research may be incomplete).

## Process

1. **Parse task envelope.** API name + scope (which endpoints / which capability). Confirm scope before research — if vague, ask.
2. **Locate authoritative doc.** Vendor's own docs > community wrappers. Pin version.
3. **Sweep nav drawer.** Most API docs put every endpoint in the left sidebar. `firecrawl-map` against `https://docs.<vendor>/...` → full URL list. List every endpoint, even out-of-scope, so coverage is visible.
4. **Filter to scope.** Keep endpoints in scope. Out-of-scope listed in appendix only.
5. **Per-endpoint extract.** Method, path, auth, request body, response body, error codes, examples. Use `firecrawl-scrape` per page.
6. **Cross-cut concerns.** Auth flow (one place). Rate limits. Pagination. Idempotency. Sandbox vs prod URLs. Retry-After. Versioning.
7. **Write spike snippet.** One request that exercises the most load-bearing endpoint. Curl + Rust. Don't run.
8. **Surface unknowns.** Anything the doc didn't answer → "Open questions" section. Ask caller before publishing if a load-bearing question is open.
9. **Clarify back.** Ambiguous scope, missing auth detail, conflicting doc pages → return question to caller. Don't guess.
10. **Publish.** Dossier to output path from task envelope.

## Output shape

```md
# API research — <vendor> <product> v<N>

## Source
- Doc root: <url>
- Version: <api version + date last verified>
- Tooling: firecrawl | WebFetch (note if fallback)

## Authentication
- Scheme: <Bearer / HMAC / OAuth2 / API key>
- Header: `<name>: <format>`
- Scope/permissions required: <list>
- Where secret lives: <env var name>
- Rotation: <cadence + procedure>

## Base URLs
- Sandbox: <url>
- Prod: <url>

## Endpoints in scope

### <METHOD> <path>
**Purpose:** <one line>
**Auth:** <scheme + scope>
**Request:**
- `field` : <type> — <meaning, units, constraints>
**Response 2xx:**
- `field` : <type> — <meaning>
**Errors:**
- `<code>` — <when + recovery>
**Source:** <doc url>

(repeat per endpoint)

## Cross-cutting

### Rate limits
- <limit + window + Retry-After behavior>

### Pagination
- <cursor / page+limit / link-header>

### Idempotency
- <key header / dedup window>

### Versioning
- <URL / header / sunset policy>

## Spike snippet

```bash
curl -H "Authorization: Bearer $TOKEN" \
  https://<sandbox>/<endpoint>
```

```rust
// minimal, illustrative — not executed
```

## Open questions
- <question caller must answer>

## Out of scope (mapped, not researched)
- <METHOD> <path> — <one-line purpose>
```

Drop sections that don't apply. Don't pad.

## Anti-patterns

| Bad | Why | Fix |
|-----|-----|-----|
| Research without scope | Dossier sprawls, never finishes | Confirm scope from task envelope before searching |
| Skipping nav drawer sweep | Miss endpoints. Caller hits unknown 404s later | Always map full sidebar first, filter after |
| Citing community blog as primary source | Blogs rot, vendor docs win | Vendor docs primary. Community as cross-check only |
| `Authorization: <token>` w/o scheme | Caller will guess wrong | Exact header format incl. prefix (`Bearer `, `HMAC `, etc) |
| `price: number` in shape table | No precision, no unit | Type + unit + constraint per field |
| Running the spike snippet | This is research, not spike | Snippet = artefact. `rex-plan-spike` runs it |
| Silent fallback when firecrawl absent | Caller can't tell research is incomplete | Note tooling fallback in dossier |
| Hand-waving auth ("OAuth-ish") | Caller stuck at first request | Exact flow, scopes, refresh semantics |
| Skipping clarification when scope unclear | Wrong dossier > no dossier | Ask caller. One question. Recommend an answer |
| Out-of-scope endpoints deleted entirely | Future asks re-do the sweep | Keep in appendix as one-liners |

## Hand-off

Dossier feeds:

- **`rex-plan-spike`** — spike snippet is the starting point. Spike runs it for real, captures findings, updates the dossier.
- **`rex-plan-sdd`** — request/response shapes → canonical schemas (`OrderIntent v1` etc).
- **`rex-plan-resources`** — auth secret name + URL → resource manifest entry.
- **`rex-code-tests-integration-testing`** — endpoints + error codes → integration test matrix.
- **ADRs** — version pin + sandbox/prod URL choice often warrant an ADR.

If the dossier is right, the next agent doesn't re-read the docs. If it's wrong, every downstream test inherits the bug.
