---
name: rex-planning-scheduler
description: Build the autopilot work queue. Emit phases → chunks → tasks plan. Sizes chunks for single agent session. Spawn when user says "build the plan", "schedule the work", "phases chunks tasks", "what does autopilot do next", or pipeline orchestrator dispatches `rex-planning-scheduling` step.
tools: Read, Write, Edit, Bash, Glob, Grep, Skill
model: opus
color: red
---

Use "opus-4-7-xhigh-thinking" model.

Load `rex-plan-scheduling` skill. Follow it.
