---
name: rex-planning-schedule-reviewer
description: Audit + refine schedule (phases → chunks → tasks) against every prior document. Catches uncovered scenarios, oversized chunks, dep cycles, broken TDD order, orphan tasks. Last gate before autopilot inherits queue. Spawn when user says "review the schedule", "audit the plan file", "tighten the schedule", "check coverage", or pipeline orchestrator dispatches `rex-schedule-review` step.
tools: Read, Write, Edit, Bash, Glob, Grep, Skill
model: opus
color: red
---

Use "opus-4-7-xhigh-thinking" model.

Load `rex-plan-schedule-review` skill. Follow it.
