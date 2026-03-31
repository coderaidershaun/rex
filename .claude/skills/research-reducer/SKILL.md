---
name: research:reducer
description: Fetch a URL, deeply analyze its content, and distill it into structured markdown deliverables — key-takeaways.md, key-insights.md, important-details.md, executive-summary.md, and math-notation.md (when math is present). Use this skill whenever the user shares a URL and wants it broken down, summarized, distilled, reduced, analyzed, or extracted into structured notes. Also trigger when the user says "reduce this", "break this down", "summarize this URL", "extract the key points", "research this link", "distill this article", or passes a URL with intent to understand or document its contents. This skill produces files inside the active harnessx project directory.
disable-model-invocation: false
user-invocable: true
---

# Research Reducer

You take a URL, fetch its content, and distill it into a set of structured markdown files that capture everything worth knowing — organized so someone can get the gist in 60 seconds or go deep when they need to.

The output lives in the active harnessx project directory at `harnessx/<project-id>/<slug>/`, where `<slug>` is a short, descriptive kebab-case name derived from the content itself.

---

## Process

### Step 1: Get the active project

Run:
```bash
harnessx project active
```

Extract the `id` field from the response. If no active project exists, tell the user they need to create one first (`harnessx project create <id>`), and stop.

### Step 2: Fetch the URL

Use the **WebFetch** tool to retrieve the content at the URL the user provided. If the fetch fails (404, timeout, paywall), tell the user what happened and ask if they have an alternative source or can paste the content directly.

Read the full content carefully. Understand the subject matter, the author's argument or purpose, the key data points, and the structure of the piece.

### Step 3: Generate the slug

Create a short kebab-case slug (2-4 words) that captures what the content is about. The slug should be meaningful enough that someone browsing the project directory would immediately know what this research covers.

Examples:
- An article about Solana MEV strategies → `solana-mev-strategies`
- A paper on transformer attention mechanisms → `transformer-attention`
- Documentation for a Rust async runtime → `rust-async-runtime`
- A blog post about order book matching engines → `orderbook-matching-engines`

### Step 4: Create the output directory

```bash
mkdir -p harnessx/<project-id>/<slug>
```

### Step 5: Write the deliverables

Write each file using the **Write** tool. Every file should be self-contained — a reader should be able to open any single file and get value from it without needing the others.

---

## Deliverable Specifications

### 1. `executive-summary.md`

The 60-second overview. Someone reading only this file should walk away understanding what the source material covers, why it matters, and the bottom line.

```markdown
# Executive Summary

**Source:** [Title or description](original-url)
**Date reviewed:** YYYY-MM-DD

## What this is about
[2-3 sentences: what the content covers and its purpose]

## Why it matters
[2-3 sentences: the significance, implications, or relevance]

## Bottom line
[1-2 sentences: the single most important thing to take away]
```

Keep it tight. If you can say it in fewer words, do.

### 2. `key-takeaways.md`

The actionable items. These are the things someone would want to remember, reference, or act on. Each takeaway should stand alone as a useful piece of knowledge.

```markdown
# Key Takeaways

**Source:** [Title or description](original-url)

## Takeaways

- **[Short label]** — [Concise explanation of the takeaway and why it matters. Include specific numbers, names, or claims where relevant.]

- **[Short label]** — [...]
```

Aim for 5-12 takeaways depending on the density of the source material. Prioritize ruthlessly — not everything is a takeaway. If the source is a 2,000-word blog post with one real insight, write one takeaway, not twelve padded ones.

### 3. `key-insights.md`

The deeper analysis. Where takeaways are about *what*, insights are about *why* and *so what*. This is where you connect ideas, identify patterns, surface implications the author may not have stated explicitly, and note what's novel or contrarian about the material.

```markdown
# Key Insights

**Source:** [Title or description](original-url)

## Insights

### [Insight title]
[2-4 sentences explaining the insight, its significance, and how it connects to broader context. This is where your analytical depth shows — don't just restate what the source says, interpret it.]

### [Insight title]
[...]
```

Aim for 3-7 insights. These should feel like "aha" moments, not summaries.

### 4. `important-details.md`

The reference material. Specific facts, figures, data points, technical details, names, dates, methodologies, configurations, code snippets, formulas, benchmarks — anything someone might need to look up later. This file is the one you open when you need the specifics.

```markdown
# Important Details

**Source:** [Title or description](original-url)

## [Category]

| Detail | Value |
|--------|-------|
| [specific item] | [specific value] |

## [Another category]

- [Specific detail with full context]
- [...]
```

Organize by category (e.g., "Performance Benchmarks", "API Endpoints", "Configuration", "Architecture Decisions"). Use tables for structured data, bullets for narrative details. Include exact quotes when precision matters.

### 5. `math-notation.md` (conditional)

**Only create this file if the source material contains mathematical content** — formulas, equations, proofs, statistical methods, algorithmic complexity, financial models, or quantitative analysis that benefits from formal notation.

If the source is a blog post about project management with no math, skip this file entirely.

```markdown
# Mathematical Notation

**Source:** [Title or description](original-url)

## [Section name]

[Brief context for what this math describes]

$$
[LaTeX equation]
$$

Where:
- $variable$ — [what it represents]
- $variable$ — [what it represents]

[Explanation of what the equation means in plain language]
```

Guidelines for the math file:
- Use LaTeX notation inside `$$` blocks for display equations and `$` for inline math
- Always define every variable after each equation
- Follow each equation with a plain-language explanation — the math should be accessible to someone who can read LaTeX but wants to quickly verify their understanding
- Group equations by topic or derivation flow
- If there's a derivation, show the steps with brief annotations between them
- Include any stated assumptions or boundary conditions

---

## Quality Standards

**Be honest about depth.** If the source material is shallow, your output should reflect that — a thin article produces thin deliverables, and that's fine. Don't inflate a 500-word opinion piece into 5 pages of notes.

**Preserve specificity.** Numbers, names, dates, versions, benchmarks — these are what make notes useful months later. "Performance improved significantly" is worthless. "Latency dropped from 340ms to 12ms (p99) after switching from mutex to lock-free queue" is gold.

**Attribute claims.** When the source makes a claim, make it clear it's the source's claim, not established fact. "The authors report..." or "According to the benchmarks..." — this matters for research notes.

**Flag gaps.** If the source material is missing something important (no methodology described, no benchmarks, claims without evidence), note it. A good research reduction doesn't just capture what's there — it flags what's missing.
