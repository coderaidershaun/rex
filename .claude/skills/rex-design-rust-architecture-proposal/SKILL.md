---
name: rex-design-rust-architecture-proposal
description: Synthesize all design documents into a polished architecture proposal — a comprehensive markdown document with mermaid diagrams, module breakdowns, error strategy, integration test plan, and critique findings, plus an HTML viewer for easy reading. Use this skill when the rex design process reaches the "proposal" step, when the user needs a single document summarizing the entire design, or when the user says things like "write the proposal", "create the architecture document", "package this up for review", "summarize the design", "give me the full picture", or "I need to see everything in one place." This is the final design deliverable — the document the user reads to approve or reject the design before implementation begins.
disable-model-invocation: false
user-invocable: false
---

# Design: Architecture Proposal

You produce the definitive design document for a Rust project — one document that synthesizes everything the design phase produced into a coherent, polished proposal the user can read, understand, and approve.

This is the moment where all the specialist work becomes visible. The error plan, the library review, the module layout, the architecture, the integration tests, the critique findings — they were written for agents. This document is written for a human. It needs to be clear, complete, and compelling.

You'll be told where to write the output (a file path like `design/proposal.md`) and given all design documents as inputs. Read every one. Then write a proposal that leaves no stone unturned, and generate an HTML viewer alongside it for a polished reading experience.

---

## Reading the inputs

Read every input document completely. You're synthesizing, not summarizing — you need to understand the details to present them well.

For each document, extract:
- **Key decisions** — what was decided and why
- **Diagrams** — mermaid diagrams to include (recreate or improve them for the proposal; use the /mermaid-diagrams skill for guidance on diagram types)
- **Risks and concerns** — anything flagged as risky, uncertain, or needing attention
- **Open questions** — anything unresolved that the user needs to decide

Pay special attention to the **critique** document if present — its findings and changes need prominent placement in the proposal, since they represent the final quality check.

---

## Writing for a human

The design documents were written for other agents — dense, technical, optimized for machine consumption. The proposal is written for the user. Different rules apply:

**Lead with the big picture.** Start with what the system does, why it's designed this way, and what the user needs to know before diving into details. The user should understand the system's shape within the first two minutes of reading.

**Use diagrams liberally.** A mermaid diagram is worth a thousand words of prose. Every major concept — module structure, data flow, type relationships, error propagation, test coverage — gets a diagram. Use the /mermaid-diagrams skill to create clear, well-labeled diagrams appropriate to what you're illustrating.

**Explain the "why" behind decisions.** The module plan says "domain-first organization." The proposal says "We organize by domain (orders, pricing, inventory) rather than by technical layer (models, services, handlers) because it keeps related code together — when you need to change how orders work, everything is in one place."

**Flag what needs the user's attention.** Open questions, trade-offs that could go either way, risks that need human judgment. Make these visually distinct and easy to find.

**Use progressive disclosure.** Put the overview and key decisions at the top. Put the detailed module specs, type definitions, and test plans further down. The user who reads the first three sections should have enough to make a go/no-go decision. The user who reads everything should have the full technical picture.

---

## Proposal structure

Write the proposal as markdown with this structure. Every section is important — don't skip sections, but scale the detail to what the inputs provide.

### 1. Executive Summary
2-3 paragraphs answering: What are we building? Why? How is it structured? What are the key risks? Is the design ready for implementation?

### 2. Project Context
- Goal (from onboarding)
- Scope (what's in, what's out)
- Key constraints (performance, compatibility, timeline)
- Success measures (how we know it worked)

### 3. System Overview
The big picture — a high-level diagram showing the system's major components and how data flows between them. This should be the first mermaid diagram in the document and should be understandable by someone who hasn't read anything else.

Use a flowchart or C4-style diagram showing:
- Entry points (where data comes in)
- Major processing stages
- Output destinations (where results go)
- External dependencies

### 4. Module Layout
- The complete directory tree
- For each module: what it does, what it produces, estimated size
- A module dependency diagram showing which modules depend on which
- Conventions for adding new modules

### 5. Architecture
The type-level design:
- Core types (structs, enums, newtypes) with their purpose and key fields
- Traits and what they abstract
- Key function signatures at module boundaries
- A class diagram showing type relationships
- A data flow diagram showing how data transforms through the types
- DRY analysis — what abstractions prevent duplication

### 6. Error Handling
- Error strategy (thiserror, hierarchy approach)
- Error types with their variants
- Propagation paths
- A diagram showing where errors originate and how they bubble up

### 7. Dependencies
- Version table (crate, version, purpose)
- For unfamiliar crates: key API patterns and usage examples
- Feature flags and configuration
- Integration notes between crates

### 8. Integration Test Strategy
- Summary of failure modes analyzed
- CRITICAL tests (detailed)
- IMPORTANT tests (summarized)
- NICE-TO-HAVE tests (listed)
- Coverage map showing which failure modes are tested
- Uncovered risks and mitigation strategies

### 9. Existing Code Analysis (if applicable)
- What exists and its current state
- Critical logic and invariants that must be preserved
- Migration strategy

### 10. Critique Findings
- Changes made to modules and architecture (with before/after)
- Outstanding findings that need attention
- Cross-reference verification results
- Overall design health assessment

### 11. Open Questions
Decisions that need human input before implementation can proceed. For each:
- The question
- The options
- Trade-offs of each option
- A recommendation (if you have one)

### 12. Implementation Roadmap
Suggested order of implementation:
1. What to build first and why
2. Dependencies between implementation tasks
3. Where integration tests should be written relative to the code they test

---

## Creating the diagrams

Use the /mermaid-diagrams skill for guidance on syntax and best practices. Create diagrams that are:

- **Self-explanatory** — clear labels, meaningful names, no abbreviations without context
- **Focused** — one concept per diagram. Don't cram everything into one diagram
- **Consistent** — same naming conventions across all diagrams
- **Appropriate** — use the right diagram type for the concept:
  - **Flowchart** — for system overview, module dependencies, data flow
  - **Class diagram** — for type relationships, struct fields, trait implementations
  - **Sequence diagram** — for request/response flows, multi-step processes
  - **State diagram** — for state machines, lifecycle states
  - **ER diagram** — for data models, storage schemas

Aim for 5-8 diagrams in a typical proposal. More is fine if each one earns its place; fewer is fine if the system is simple.

---

## Generating the HTML viewer

After writing the markdown proposal, generate an HTML file that renders it beautifully. Use the template at `assets/proposal-viewer.html` in this skill's directory.

Steps:
1. Read the template from `assets/proposal-viewer.html`
2. Replace the placeholders:
   - `__PROJECT_TITLE__` → the project's title (from onboarding goal or project metadata)
   - `__DATE__` → today's date
   - `__MARKDOWN_FILENAME__` → the filename of the markdown proposal (just the filename, not the full path — e.g., `proposal.md`)
3. Write the HTML file alongside the markdown file (same directory, same name but `.html` extension)

The HTML viewer:
- Fetches and renders the markdown file client-side using marked.js
- Renders mermaid diagrams inline
- Has a collapsible table of contents built from headings
- Supports dark mode automatically
- Is print-friendly
- Works offline after first load (CDN scripts are cached)

Tell the user both files exist and suggest they open the HTML file in a browser for the best reading experience. Alternatively, the markdown file works in any markdown viewer (GitHub, VS Code, Obsidian) with mermaid support.

---

## Quality checklist

Before writing the output, verify:

- [ ] Every input document is represented in the proposal
- [ ] Every mermaid diagram renders correctly (valid syntax, no broken references)
- [ ] No section is empty or placeholder-only
- [ ] Open questions are clearly marked and easy to find
- [ ] The executive summary is accurate and complete
- [ ] The document reads coherently from top to bottom (not just a list of sections pasted together)
- [ ] Technical terms are explained on first use
- [ ] File paths and type names are consistent across sections
- [ ] The critique findings are prominently featured, not buried
- [ ] The HTML viewer is generated with correct placeholders replaced

Write both files (markdown and HTML) to the output directory you were given.
