//! Address resolution and slug helpers: parse dotted positions, locate items by slug or position.

use crate::error::RexError;

use super::types::{Chunk, Phase, Schedule, Task};

/// Parse a dotted position string (`"1"`, `"1.2"`, `"1.2.3"`) into 1-indexed parts.
fn parse_dotted(addr: &str) -> Option<Vec<usize>> {
    let parts: Vec<usize> = addr
        .split('.')
        .map(|p| p.parse::<usize>().ok())
        .collect::<Option<Vec<_>>>()?;
    if parts.is_empty() || parts.contains(&0) {
        return None;
    }
    Some(parts)
}

/// Find a phase by slug id or 1-indexed position (`"1"`).
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] when no phase matches.
pub fn find_phase<'s>(s: &'s Schedule, addr: &str) -> Result<(usize, &'s Phase), RexError> {
    if let Some(parts) = parse_dotted(addr) {
        let idx = parts[0] - 1;
        return s
            .phases
            .get(idx)
            .map(|p| (idx, p))
            .ok_or_else(|| RexError::ScheduleAddrNotFound {
                addr: addr.to_owned(),
            });
    }
    s.phases
        .iter()
        .enumerate()
        .find(|(_, p)| p.id == addr)
        .ok_or_else(|| RexError::ScheduleAddrNotFound {
            addr: addr.to_owned(),
        })
}

/// Find a chunk by slug id or dotted position (`"1.2"`).
///
/// Bare slug addressing is accepted if and only if the slug is unique across all phases.
///
/// # Errors
/// - [`RexError::AmbiguousAddr`] when the slug exists in multiple phases.
/// - [`RexError::ScheduleAddrNotFound`] when no chunk matches.
pub fn find_chunk<'s>(s: &'s Schedule, addr: &str) -> Result<(usize, usize, &'s Chunk), RexError> {
    if let Some(parts) = parse_dotted(addr) {
        if parts.len() < 2 {
            return Err(RexError::ScheduleAddrNotFound {
                addr: addr.to_owned(),
            });
        }
        let phase_idx = parts[0] - 1;
        let chunk_idx = parts[1] - 1;
        return s
            .phases
            .get(phase_idx)
            .and_then(|p| p.chunks.get(chunk_idx).map(|c| (phase_idx, chunk_idx, c)))
            .ok_or_else(|| RexError::ScheduleAddrNotFound {
                addr: addr.to_owned(),
            });
    }

    // Bare slug: collect all matches, error on ambiguity.
    let matches: Vec<(usize, usize, &Chunk)> = s
        .phases
        .iter()
        .enumerate()
        .flat_map(|(pi, p)| {
            p.chunks
                .iter()
                .enumerate()
                .filter(|(_, c)| c.id == addr)
                .map(move |(ci, c)| (pi, ci, c))
        })
        .collect();

    match matches.len() {
        0 => Err(RexError::ScheduleAddrNotFound {
            addr: addr.to_owned(),
        }),
        1 => Ok(matches.into_iter().next().unwrap()),
        _ => {
            let candidates: Vec<String> = matches
                .iter()
                .map(|(pi, ci, _)| format!("{}.{}", pi + 1, ci + 1))
                .collect();
            Err(RexError::AmbiguousAddr {
                addr: addr.to_owned(),
                candidates: candidates.join(", "),
            })
        }
    }
}

/// Find a task by slug id or dotted position (`"1.2.3"`).
///
/// Bare slug addressing is accepted if and only if the slug is unique across all chunks.
///
/// # Errors
/// - [`RexError::AmbiguousAddr`] when the slug exists in multiple chunks.
/// - [`RexError::ScheduleAddrNotFound`] when no task matches.
pub fn find_task<'s>(
    s: &'s Schedule,
    addr: &str,
) -> Result<(usize, usize, usize, &'s Task), RexError> {
    if let Some(parts) = parse_dotted(addr) {
        if parts.len() < 3 {
            return Err(RexError::ScheduleAddrNotFound {
                addr: addr.to_owned(),
            });
        }
        let phase_idx = parts[0] - 1;
        let chunk_idx = parts[1] - 1;
        let task_idx = parts[2] - 1;
        return s
            .phases
            .get(phase_idx)
            .and_then(|p| p.chunks.get(chunk_idx))
            .and_then(|c| {
                c.tasks
                    .get(task_idx)
                    .map(|t| (phase_idx, chunk_idx, task_idx, t))
            })
            .ok_or_else(|| RexError::ScheduleAddrNotFound {
                addr: addr.to_owned(),
            });
    }

    // Bare slug: collect all matches, error on ambiguity.
    let mut matches: Vec<(usize, usize, usize, &Task)> = Vec::new();
    for (pi, phase) in s.phases.iter().enumerate() {
        for (ci, chunk) in phase.chunks.iter().enumerate() {
            for (ti, task) in chunk.tasks.iter().enumerate() {
                if task.id == addr {
                    matches.push((pi, ci, ti, task));
                }
            }
        }
    }

    match matches.len() {
        0 => Err(RexError::ScheduleAddrNotFound {
            addr: addr.to_owned(),
        }),
        1 => Ok(matches.into_iter().next().unwrap()),
        _ => {
            let candidates: Vec<String> = matches
                .iter()
                .map(|(pi, ci, ti, _)| format!("{}.{}.{}", pi + 1, ci + 1, ti + 1))
                .collect();
            Err(RexError::AmbiguousAddr {
                addr: addr.to_owned(),
                candidates: candidates.join(", "),
            })
        }
    }
}

/// Return a slug for `desired` that does not collide with `existing`.
///
/// If `desired` is unique, returns it unchanged. Otherwise appends `-2`, `-3`, …
/// skipping any already taken.
pub fn unique_slug(existing: &[&str], desired: &str) -> String {
    if !existing.contains(&desired) {
        return desired.to_owned();
    }
    let mut n: u32 = 2;
    loop {
        let candidate = format!("{desired}-{n}");
        if !existing.contains(&candidate.as_str()) {
            return candidate;
        }
        n += 1;
    }
}

/// Derive a slug from a description and ensure it is unique within `existing`.
pub fn slug_from_description(existing: &[&str], description: &str) -> String {
    let base = slug::slugify(description);
    unique_slug(existing, &base)
}

/// Rewrite every `blocked_by` entry at the phase level.
///
/// When `new_id` is `Some`, replaces all occurrences of `old` with `new_id`.
/// When `new_id` is `None`, drops `old` from every `blocked_by` list
/// (used on phase remove).
pub fn rewrite_blocked_by_at_phase_level(s: &mut Schedule, old: &str, new_id: Option<&str>) {
    for phase in &mut s.phases {
        phase.blocked_by = phase
            .blocked_by
            .drain(..)
            .filter_map(|dep| {
                if dep == old {
                    new_id.map(str::to_owned)
                } else {
                    Some(dep)
                }
            })
            .collect();
    }
}

/// Rewrite every `blocked_by` entry at the chunk level within a single phase.
///
/// `new_id = None` drops occurrences (used on chunk remove).
pub fn rewrite_blocked_by_at_chunk_level(
    s: &mut Schedule,
    phase_idx: usize,
    old: &str,
    new_id: Option<&str>,
) {
    if let Some(phase) = s.phases.get_mut(phase_idx) {
        for chunk in &mut phase.chunks {
            chunk.blocked_by = chunk
                .blocked_by
                .drain(..)
                .filter_map(|dep| {
                    if dep == old {
                        new_id.map(str::to_owned)
                    } else {
                        Some(dep)
                    }
                })
                .collect();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::ProjectId;
    use crate::schedule::types::{Chunk, Phase, Schedule, ScheduleState, Task};

    fn task(id: &str, state: ScheduleState) -> Task {
        Task { id: id.to_owned(), description: id.to_owned(), state, skill: None, inputs: None, outputs: None }
    }

    fn chunk(id: &str, blocked_by: Vec<String>, state: ScheduleState, tasks: Vec<Task>) -> Chunk {
        Chunk { id: id.to_owned(), description: id.to_owned(), scenarios: vec![], spec_refs: vec![], blocked_by, state, tasks }
    }

    fn make_schedule() -> Schedule {
        Schedule {
            project: ProjectId::parse("test-project").unwrap(),
            phases: vec![
                Phase {
                    id: "phase-a".to_owned(),
                    description: "Phase A".to_owned(),
                    blocked_by: vec![],
                    state: ScheduleState::Pending,
                    chunks: vec![
                        chunk("chunk-1", vec![], ScheduleState::Pending, vec![
                            task("task-a", ScheduleState::Pending),
                            task("task-b", ScheduleState::Done),
                        ]),
                        chunk("chunk-2", vec!["chunk-1".to_owned()], ScheduleState::Done, vec![]),
                    ],
                },
                Phase {
                    id: "phase-b".to_owned(),
                    description: "Phase B".to_owned(),
                    blocked_by: vec!["phase-a".to_owned()],
                    state: ScheduleState::Pending,
                    // same chunk slug as phase-a to exercise ambiguity detection
                    chunks: vec![chunk("chunk-1", vec![], ScheduleState::Pending, vec![
                        task("task-a", ScheduleState::Pending),
                    ])],
                },
            ],
        }
    }

    // ── unique_slug ───────────────────────────────────────────────────────────

    #[test]
    fn unique_slug_returns_input_when_no_collision() {
        let result = unique_slug(&["alpha", "beta"], "gamma");
        assert_eq!(result, "gamma");
    }

    #[test]
    fn unique_slug_appends_numeric_suffix_on_collision() {
        let result = unique_slug(&["alpha", "beta"], "alpha");
        assert_eq!(result, "alpha-2");
    }

    #[test]
    fn unique_slug_skips_existing_suffixes() {
        let result = unique_slug(&["alpha", "alpha-2", "alpha-3"], "alpha");
        assert_eq!(result, "alpha-4");
    }

    // ── find_phase ────────────────────────────────────────────────────────────

    #[test]
    fn find_phase_by_slug() {
        let s = make_schedule();
        let (idx, phase) = find_phase(&s, "phase-a").unwrap();
        assert_eq!(idx, 0);
        assert_eq!(phase.id, "phase-a");
    }

    #[test]
    fn find_phase_by_position() {
        let s = make_schedule();
        let (idx, phase) = find_phase(&s, "2").unwrap();
        assert_eq!(idx, 1);
        assert_eq!(phase.id, "phase-b");
    }

    #[test]
    fn find_phase_not_found_returns_error() {
        let s = make_schedule();
        assert!(matches!(
            find_phase(&s, "nonexistent"),
            Err(RexError::ScheduleAddrNotFound { .. })
        ));
    }

    // ── find_chunk ────────────────────────────────────────────────────────────

    #[test]
    fn find_chunk_by_dotted_position() {
        let s = make_schedule();
        let (pi, ci, chunk) = find_chunk(&s, "1.2").unwrap();
        assert_eq!(pi, 0);
        assert_eq!(ci, 1);
        assert_eq!(chunk.id, "chunk-2");
    }

    #[test]
    fn find_chunk_by_unique_slug() {
        let s = make_schedule();
        let (pi, ci, chunk) = find_chunk(&s, "chunk-2").unwrap();
        assert_eq!(pi, 0);
        assert_eq!(ci, 1);
        assert_eq!(chunk.id, "chunk-2");
    }

    #[test]
    fn find_chunk_ambiguous_slug_returns_error() {
        let s = make_schedule();
        // chunk-1 exists in both phase-a and phase-b
        let err = find_chunk(&s, "chunk-1").unwrap_err();
        match err {
            RexError::AmbiguousAddr { addr, candidates } => {
                assert_eq!(addr, "chunk-1");
                // Both phase-a chunk-1 (1.1) and phase-b chunk-1 (2.1) must appear
                // so the agent can re-call with a dotted address.
                assert!(
                    candidates.contains("1.1"),
                    "candidates missing 1.1: {candidates}"
                );
                assert!(
                    candidates.contains("2.1"),
                    "candidates missing 2.1: {candidates}"
                );
            }
            other => panic!("expected AmbiguousAddr, got {other:?}"),
        }
    }

    // ── find_task ─────────────────────────────────────────────────────────────

    #[test]
    fn find_task_by_dotted_position() {
        let s = make_schedule();
        let (pi, ci, ti, task) = find_task(&s, "1.1.2").unwrap();
        assert_eq!(pi, 0);
        assert_eq!(ci, 0);
        assert_eq!(ti, 1);
        assert_eq!(task.id, "task-b");
    }

    #[test]
    fn find_task_ambiguous_slug_returns_error() {
        let s = make_schedule();
        // task-a exists in phase-a/chunk-1 and phase-b/chunk-1
        let err = find_task(&s, "task-a").unwrap_err();
        match err {
            RexError::AmbiguousAddr { addr, candidates } => {
                assert_eq!(addr, "task-a");
                assert!(
                    candidates.contains("1.1.1"),
                    "candidates missing 1.1.1: {candidates}"
                );
                assert!(
                    candidates.contains("2.1.1"),
                    "candidates missing 2.1.1: {candidates}"
                );
            }
            other => panic!("expected AmbiguousAddr, got {other:?}"),
        }
    }

    // ── rewrite_blocked_by_at_phase_level ─────────────────────────────────────

    #[test]
    fn rewrite_blocked_by_renames_every_match_at_phase_level() {
        let mut s = make_schedule();
        // phase-b blocked_by phase-a
        rewrite_blocked_by_at_phase_level(&mut s, "phase-a", Some("phase-alpha"));
        assert!(s.phases[1].blocked_by.contains(&"phase-alpha".to_owned()));
        assert!(!s.phases[1].blocked_by.contains(&"phase-a".to_owned()));
    }

    #[test]
    fn rewrite_blocked_by_drops_when_target_removed_at_phase_level() {
        let mut s = make_schedule();
        rewrite_blocked_by_at_phase_level(&mut s, "phase-a", None);
        assert!(s.phases[1].blocked_by.is_empty());
    }

    // ── rewrite_blocked_by_at_chunk_level ────────────────────────────────────

    #[test]
    fn rewrite_blocked_by_renames_every_match_at_chunk_level() {
        let mut s = make_schedule();
        // chunk-2 (idx 1) in phase-a (idx 0) is blocked by chunk-1
        rewrite_blocked_by_at_chunk_level(&mut s, 0, "chunk-1", Some("chunk-one"));
        assert!(
            s.phases[0].chunks[1]
                .blocked_by
                .contains(&"chunk-one".to_owned())
        );
    }

    #[test]
    fn rewrite_blocked_by_drops_when_target_removed_at_chunk_level() {
        let mut s = make_schedule();
        rewrite_blocked_by_at_chunk_level(&mut s, 0, "chunk-1", None);
        assert!(s.phases[0].chunks[1].blocked_by.is_empty());
    }

}
