use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};

use crate::error::RexError;
use crate::project::ProjectId;

/// Lifecycle state of a task, chunk, or phase.
///
/// `InProgress` is reserved vocabulary for agents who hand-edit `schedule.json`
/// to mark in-flight work; no CLI path writes it today. `chunk-next` and
/// `task complete` both treat `InProgress` as selectable so a hand-edit cannot
/// hide a chunk from the work queue.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ScheduleState {
    Pending,
    InProgress,
    Done,
    Blocked,
}

impl ScheduleState {
    /// `true` for states that should still be picked up by the work queue
    /// (`Pending` or `InProgress`).
    fn is_open(&self) -> bool {
        matches!(self, Self::Pending | Self::InProgress)
    }

    pub fn is_done(&self) -> bool {
        matches!(self, Self::Done)
    }
}

impl std::str::FromStr for ScheduleState {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "in-progress" => Ok(Self::InProgress),
            "done" => Ok(Self::Done),
            "blocked" => Ok(Self::Blocked),
            other => Err(format!(
                "unknown state '{other}'; expected pending|in-progress|done|blocked"
            )),
        }
    }
}

/// One atomic unit of work within a chunk.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct Task {
    pub id: String,
    pub description: String,
    pub state: ScheduleState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inputs: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<String>,
}

/// A vertical slice of work — one agent session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct Chunk {
    pub id: String,
    pub description: String,
    pub scenarios: Vec<String>,
    pub spec_refs: Vec<String>,
    pub blocked_by: Vec<String>,
    pub state: ScheduleState,
    pub tasks: Vec<Task>,
}

/// A PRD capability milestone — contains one or more chunks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct Phase {
    pub id: String,
    pub description: String,
    pub blocked_by: Vec<String>,
    pub state: ScheduleState,
    pub chunks: Vec<Chunk>,
}

/// The full autopilot work queue for a project.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct Schedule {
    pub project: ProjectId,
    pub phases: Vec<Phase>,
}

/// Result of marking a task as done, including auto-promotion flags.
pub struct TaskCompletion {
    /// The task that was just marked done.
    pub task: Task,
    /// `true` when the parent chunk was auto-promoted to `Done`.
    pub chunk_promoted: bool,
    /// `true` when the parent phase was auto-promoted to `Done`.
    pub phase_promoted: bool,
}

/// Counter values derived from a schedule's current state.
///
/// Mirrors the four counter fields in `project.yaml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleCounters {
    pub chunks_required: u32,
    pub tasks_required: u32,
    pub chunks_completed: u32,
    pub tasks_completed: u32,
}

/// Optional fields for updating a phase.
#[derive(Debug, Default)]
pub struct PhaseEdit {
    pub description: Option<String>,
    pub new_id: Option<String>,
    pub state: Option<ScheduleState>,
    pub blocked_by: Option<Vec<String>>,
}

/// Optional fields for updating a chunk.
#[derive(Debug, Default)]
pub struct ChunkEdit {
    pub description: Option<String>,
    pub new_id: Option<String>,
    pub state: Option<ScheduleState>,
    pub blocked_by: Option<Vec<String>>,
    pub scenarios: Option<Vec<String>>,
    pub spec_refs: Option<Vec<String>>,
}

/// Optional fields for updating a task.
#[derive(Debug, Default)]
pub struct TaskEdit {
    pub description: Option<String>,
    pub new_id: Option<String>,
    pub state: Option<ScheduleState>,
    pub skill: Option<Option<String>>,
    pub inputs: Option<Option<String>>,
    pub outputs: Option<Option<String>>,
}

// ── Addressing ───────────────────────────────────────────────────────────────

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

// ── Slug helpers ─────────────────────────────────────────────────────────────

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

// ── blocked_by maintenance ────────────────────────────────────────────────────

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

// ── Counters ─────────────────────────────────────────────────────────────────

/// Derive the four counter values from the current schedule state.
///
/// Called after every mutation to keep `project.yaml` in sync.
pub fn counters_for(s: &Schedule) -> ScheduleCounters {
    let mut chunks_required: u32 = 0;
    let mut tasks_required: u32 = 0;
    let mut chunks_completed: u32 = 0;
    let mut tasks_completed: u32 = 0;

    for phase in &s.phases {
        for chunk in &phase.chunks {
            chunks_required += 1;
            if chunk.state.is_done() {
                chunks_completed += 1;
            }
            for task in &chunk.tasks {
                tasks_required += 1;
                if task.state.is_done() {
                    tasks_completed += 1;
                }
            }
        }
    }

    ScheduleCounters {
        chunks_required,
        tasks_required,
        chunks_completed,
        tasks_completed,
    }
}

// ── Validation ───────────────────────────────────────────────────────────────

/// Check for cycles in phase-level `blocked_by` references using Kahn's algorithm.
///
/// Returns `Err(BlockedByCycle)` naming one member of the cycle.
pub fn validate_no_phase_cycles(s: &Schedule) -> Result<(), RexError> {
    let ids: Vec<&str> = s.phases.iter().map(|p| p.id.as_str()).collect();
    let id_set: HashSet<&str> = ids.iter().copied().collect();
    let mut in_degree: HashMap<&str, usize> = ids.iter().map(|&id| (id, 0)).collect();
    let mut edges: HashMap<&str, Vec<&str>> = HashMap::new();

    for phase in &s.phases {
        for dep in &phase.blocked_by {
            let dep = dep.as_str();
            if id_set.contains(dep) {
                *in_degree.entry(phase.id.as_str()).or_insert(0) += 1;
                edges.entry(dep).or_default().push(phase.id.as_str());
            }
        }
    }

    let mut queue: VecDeque<&str> = in_degree
        .iter()
        .filter(|(_, deg)| **deg == 0)
        .map(|(id, _)| *id)
        .collect();

    let mut visited = 0usize;
    while let Some(node) = queue.pop_front() {
        visited += 1;
        if let Some(dependents) = edges.get(node) {
            for &dep in dependents {
                let count = in_degree.get_mut(dep).unwrap();
                *count -= 1;
                if *count == 0 {
                    queue.push_back(dep);
                }
            }
        }
    }

    if visited < ids.len() {
        let cycle_member = in_degree
            .iter()
            .find(|(_, deg)| **deg > 0)
            .map(|(id, _)| *id)
            .expect("Kahn's algorithm guarantees a node with non-zero in-degree when visited < ids.len()");
        return Err(RexError::BlockedByCycle {
            addr: cycle_member.to_owned(),
        });
    }
    Ok(())
}

/// Check for cycles in chunk-level `blocked_by` within each phase.
pub fn validate_no_chunk_cycles(s: &Schedule) -> Result<(), RexError> {
    for phase in &s.phases {
        let ids: Vec<&str> = phase.chunks.iter().map(|c| c.id.as_str()).collect();
        let id_set: HashSet<&str> = ids.iter().copied().collect();
        let mut in_degree: HashMap<&str, usize> = ids.iter().map(|&id| (id, 0)).collect();
        let mut edges: HashMap<&str, Vec<&str>> = HashMap::new();

        for chunk in &phase.chunks {
            for dep in &chunk.blocked_by {
                let dep = dep.as_str();
                if id_set.contains(dep) {
                    *in_degree.entry(chunk.id.as_str()).or_insert(0) += 1;
                    edges.entry(dep).or_default().push(chunk.id.as_str());
                }
            }
        }

        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, deg)| **deg == 0)
            .map(|(id, _)| *id)
            .collect();

        let mut visited = 0usize;
        while let Some(node) = queue.pop_front() {
            visited += 1;
            if let Some(dependents) = edges.get(node) {
                for &dep in dependents {
                    let count = in_degree.get_mut(dep).unwrap();
                    *count -= 1;
                    if *count == 0 {
                        queue.push_back(dep);
                    }
                }
            }
        }

        if visited < ids.len() {
            let cycle_member = in_degree
                .iter()
                .find(|(_, deg)| **deg > 0)
                .map(|(id, _)| *id)
                .expect("Kahn's algorithm guarantees a node with non-zero in-degree when visited < ids.len()");
            return Err(RexError::BlockedByCycle {
                addr: cycle_member.to_owned(),
            });
        }
    }
    Ok(())
}

/// Validate that phase slugs are globally unique, chunk slugs are unique within
/// their phase, and task slugs are unique within their chunk.
pub fn validate_slug_uniqueness(s: &Schedule) -> Result<(), RexError> {
    let mut phase_ids = HashSet::new();
    for phase in &s.phases {
        if !phase_ids.insert(phase.id.as_str()) {
            return Err(RexError::DuplicateSlug {
                addr: phase.id.clone(),
            });
        }
        let mut chunk_ids = HashSet::new();
        for chunk in &phase.chunks {
            if !chunk_ids.insert(chunk.id.as_str()) {
                return Err(RexError::DuplicateSlug {
                    addr: chunk.id.clone(),
                });
            }
            let mut task_ids = HashSet::new();
            for task in &chunk.tasks {
                if !task_ids.insert(task.id.as_str()) {
                    return Err(RexError::DuplicateSlug {
                        addr: task.id.clone(),
                    });
                }
            }
        }
    }
    Ok(())
}

/// Check that replacing `old` with `new` would not regress any `done` item to non-done.
///
/// Phase, chunk, and task identities are matched by id within their respective scopes.
pub fn validate_no_state_regression(old: &Schedule, new: &Schedule) -> Result<(), RexError> {
    let mut offenders: Vec<String> = Vec::new();

    // Build maps of existing done ids at each tier.
    let done_phases: HashSet<&str> = old
        .phases
        .iter()
        .filter(|p| p.state.is_done())
        .map(|p| p.id.as_str())
        .collect();
    let done_chunks: HashMap<(&str, &str), ()> = old
        .phases
        .iter()
        .flat_map(|p| {
            p.chunks
                .iter()
                .filter(|c| c.state.is_done())
                .map(move |c| ((p.id.as_str(), c.id.as_str()), ()))
        })
        .collect();
    let done_tasks: HashMap<(&str, &str, &str), ()> = old
        .phases
        .iter()
        .flat_map(|p| {
            p.chunks.iter().flat_map(move |c| {
                c.tasks
                    .iter()
                    .filter(|t| t.state.is_done())
                    .map(move |t| ((p.id.as_str(), c.id.as_str(), t.id.as_str()), ()))
            })
        })
        .collect();

    for phase in &new.phases {
        if done_phases.contains(phase.id.as_str()) && !phase.state.is_done() {
            offenders.push(format!("phase:{}", phase.id));
        }
        for chunk in &phase.chunks {
            let key = (phase.id.as_str(), chunk.id.as_str());
            if done_chunks.contains_key(&key) && !chunk.state.is_done() {
                offenders.push(format!("chunk:{}/{}", phase.id, chunk.id));
            }
            for task in &chunk.tasks {
                let key = (phase.id.as_str(), chunk.id.as_str(), task.id.as_str());
                if done_tasks.contains_key(&key) && !task.state.is_done() {
                    offenders.push(format!("task:{}/{}/{}", phase.id, chunk.id, task.id));
                }
            }
        }
    }

    if offenders.is_empty() {
        Ok(())
    } else {
        Err(RexError::ReplaceWouldRegressState {
            offenders: offenders.join(", "),
        })
    }
}

// ── Phase mutations ───────────────────────────────────────────────────────────

/// Append a new phase, returning the persisted phase.
pub fn add_phase(s: &mut Schedule, phase: Phase) -> Phase {
    s.phases.push(phase.clone());
    phase
}

/// Update a phase identified by `addr`.
///
/// Renames rewrite all phase-level `blocked_by` references across all phases.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] when `addr` does not match any phase.
/// - [`RexError::DuplicateSlug`] when `edit.new_id` collides with an existing phase.
pub fn update_phase(s: &mut Schedule, addr: &str, edit: PhaseEdit) -> Result<Phase, RexError> {
    let (idx, _) = find_phase(s, addr)?;

    if let Some(ref new_id) = edit.new_id {
        let collision = s
            .phases
            .iter()
            .enumerate()
            .any(|(i, p)| i != idx && p.id == *new_id);
        if collision {
            return Err(RexError::DuplicateSlug {
                addr: new_id.clone(),
            });
        }
        let old_id = s.phases[idx].id.clone();
        rewrite_blocked_by_at_phase_level(s, &old_id, Some(new_id));
        s.phases[idx].id = new_id.clone();
    }
    if let Some(desc) = edit.description {
        s.phases[idx].description = desc;
    }
    if let Some(state) = edit.state {
        s.phases[idx].state = state;
    }
    if let Some(blocked_by) = edit.blocked_by {
        s.phases[idx].blocked_by = blocked_by;
    }

    Ok(s.phases[idx].clone())
}

/// Remove a phase identified by `addr`, dropping dangling `blocked_by` references.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] when `addr` does not match any phase.
pub fn remove_phase(s: &mut Schedule, addr: &str) -> Result<Phase, RexError> {
    let (idx, _) = find_phase(s, addr)?;
    let removed = s.phases.remove(idx);
    rewrite_blocked_by_at_phase_level(s, &removed.id, None);
    Ok(removed)
}

/// Move a phase to 1-indexed position `to`.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] when `addr` does not match.
/// - [`RexError::ScheduleAddrNotFound`] when `to` is out of range.
pub fn move_phase(s: &mut Schedule, addr: &str, to: usize) -> Result<Phase, RexError> {
    let (from_idx, _) = find_phase(s, addr)?;
    let target_idx = to.saturating_sub(1);
    if target_idx >= s.phases.len() {
        return Err(RexError::ScheduleAddrNotFound {
            addr: format!("{to}"),
        });
    }
    let phase = s.phases.remove(from_idx);
    s.phases.insert(target_idx, phase);
    Ok(s.phases[target_idx].clone())
}

// ── Chunk mutations ───────────────────────────────────────────────────────────

/// Append a chunk to the phase identified by `phase_addr`.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] when `phase_addr` does not match.
pub fn add_chunk(s: &mut Schedule, phase_addr: &str, chunk: Chunk) -> Result<Chunk, RexError> {
    let (phase_idx, _) = find_phase(s, phase_addr)?;
    s.phases[phase_idx].chunks.push(chunk.clone());
    Ok(chunk)
}

/// Update a chunk identified by `addr`.
///
/// Renames rewrite chunk-level `blocked_by` within the same phase.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] when `addr` does not match.
/// - [`RexError::DuplicateSlug`] when `edit.new_id` collides within the same phase.
/// - [`RexError::AmbiguousAddr`] when `addr` is a bare slug matching multiple phases.
pub fn update_chunk(s: &mut Schedule, addr: &str, edit: ChunkEdit) -> Result<Chunk, RexError> {
    let (phase_idx, chunk_idx, _) = find_chunk(s, addr)?;

    if let Some(ref new_id) = edit.new_id {
        let collision = s.phases[phase_idx]
            .chunks
            .iter()
            .enumerate()
            .any(|(i, c)| i != chunk_idx && c.id == *new_id);
        if collision {
            return Err(RexError::DuplicateSlug {
                addr: new_id.clone(),
            });
        }
        let old_id = s.phases[phase_idx].chunks[chunk_idx].id.clone();
        rewrite_blocked_by_at_chunk_level(s, phase_idx, &old_id, Some(new_id));
        s.phases[phase_idx].chunks[chunk_idx].id = new_id.clone();
    }
    if let Some(desc) = edit.description {
        s.phases[phase_idx].chunks[chunk_idx].description = desc;
    }
    if let Some(state) = edit.state {
        s.phases[phase_idx].chunks[chunk_idx].state = state;
    }
    if let Some(blocked_by) = edit.blocked_by {
        s.phases[phase_idx].chunks[chunk_idx].blocked_by = blocked_by;
    }
    if let Some(scenarios) = edit.scenarios {
        s.phases[phase_idx].chunks[chunk_idx].scenarios = scenarios;
    }
    if let Some(spec_refs) = edit.spec_refs {
        s.phases[phase_idx].chunks[chunk_idx].spec_refs = spec_refs;
    }

    Ok(s.phases[phase_idx].chunks[chunk_idx].clone())
}

/// Remove a chunk identified by `addr`, dropping dangling `blocked_by` references
/// within its parent phase.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] when `addr` does not match.
/// - [`RexError::AmbiguousAddr`] when `addr` is an ambiguous bare slug.
pub fn remove_chunk(s: &mut Schedule, addr: &str) -> Result<Chunk, RexError> {
    let (phase_idx, chunk_idx, _) = find_chunk(s, addr)?;
    let removed = s.phases[phase_idx].chunks.remove(chunk_idx);
    rewrite_blocked_by_at_chunk_level(s, phase_idx, &removed.id, None);
    Ok(removed)
}

/// Move a chunk to a new position and optionally a new parent phase.
///
/// `to_phase_addr` re-parents; `to` sets 1-indexed position within the destination phase.
/// If neither is supplied the call is a no-op and the current chunk is returned.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] when any address does not match.
/// - [`RexError::AmbiguousAddr`] for ambiguous bare slugs.
pub fn move_chunk(
    s: &mut Schedule,
    addr: &str,
    to_phase_addr: Option<&str>,
    to: Option<usize>,
) -> Result<Chunk, RexError> {
    let (src_phase_idx, src_chunk_idx, _) = find_chunk(s, addr)?;

    // No flags supplied — no-op, return the chunk as-is.
    if to_phase_addr.is_none() && to.is_none() {
        return Ok(s.phases[src_phase_idx].chunks[src_chunk_idx].clone());
    }

    let dst_phase_idx = if let Some(pa) = to_phase_addr {
        find_phase(s, pa)?.0
    } else {
        src_phase_idx
    };

    let chunk = s.phases[src_phase_idx].chunks.remove(src_chunk_idx);

    // Adjust dst_chunk_idx for possible index shift if same phase.
    let dst_len = s.phases[dst_phase_idx].chunks.len();
    let insert_idx = if let Some(pos) = to {
        (pos - 1).min(dst_len)
    } else {
        dst_len
    };

    s.phases[dst_phase_idx].chunks.insert(insert_idx, chunk);
    Ok(s.phases[dst_phase_idx].chunks[insert_idx].clone())
}

// ── Task mutations ────────────────────────────────────────────────────────────

/// Append a task to the chunk identified by `chunk_addr`.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] when `chunk_addr` does not match.
/// - [`RexError::AmbiguousAddr`] for ambiguous bare slugs.
pub fn add_task(s: &mut Schedule, chunk_addr: &str, task: Task) -> Result<Task, RexError> {
    let (phase_idx, chunk_idx, _) = find_chunk(s, chunk_addr)?;
    s.phases[phase_idx].chunks[chunk_idx]
        .tasks
        .push(task.clone());
    Ok(task)
}

/// Update a task identified by `addr`.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] when `addr` does not match.
/// - [`RexError::DuplicateSlug`] when `edit.new_id` collides within the same chunk.
/// - [`RexError::AmbiguousAddr`] for ambiguous bare slugs.
pub fn update_task(s: &mut Schedule, addr: &str, edit: TaskEdit) -> Result<Task, RexError> {
    let (phase_idx, chunk_idx, task_idx, _) = find_task(s, addr)?;

    if let Some(ref new_id) = edit.new_id {
        let collision = s.phases[phase_idx].chunks[chunk_idx]
            .tasks
            .iter()
            .enumerate()
            .any(|(i, t)| i != task_idx && t.id == *new_id);
        if collision {
            return Err(RexError::DuplicateSlug {
                addr: new_id.clone(),
            });
        }
        s.phases[phase_idx].chunks[chunk_idx].tasks[task_idx].id = new_id.clone();
    }
    if let Some(desc) = edit.description {
        s.phases[phase_idx].chunks[chunk_idx].tasks[task_idx].description = desc;
    }
    if let Some(state) = edit.state {
        s.phases[phase_idx].chunks[chunk_idx].tasks[task_idx].state = state;
    }
    if let Some(skill) = edit.skill {
        s.phases[phase_idx].chunks[chunk_idx].tasks[task_idx].skill = skill;
    }
    if let Some(inputs) = edit.inputs {
        s.phases[phase_idx].chunks[chunk_idx].tasks[task_idx].inputs = inputs;
    }
    if let Some(outputs) = edit.outputs {
        s.phases[phase_idx].chunks[chunk_idx].tasks[task_idx].outputs = outputs;
    }

    Ok(s.phases[phase_idx].chunks[chunk_idx].tasks[task_idx].clone())
}

/// Remove a task identified by `addr`.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] when `addr` does not match.
/// - [`RexError::AmbiguousAddr`] for ambiguous bare slugs.
pub fn remove_task(s: &mut Schedule, addr: &str) -> Result<Task, RexError> {
    let (phase_idx, chunk_idx, task_idx, _) = find_task(s, addr)?;
    Ok(s.phases[phase_idx].chunks[chunk_idx].tasks.remove(task_idx))
}

/// Move a task to a new position and optionally a new parent chunk.
///
/// # Errors
/// - [`RexError::ScheduleAddrNotFound`] when any address does not match.
/// - [`RexError::AmbiguousAddr`] for ambiguous bare slugs.
pub fn move_task(
    s: &mut Schedule,
    addr: &str,
    to_chunk_addr: Option<&str>,
    to: Option<usize>,
) -> Result<Task, RexError> {
    let (src_phase_idx, src_chunk_idx, src_task_idx, _) = find_task(s, addr)?;

    // No flags supplied — no-op, return the task as-is.
    if to_chunk_addr.is_none() && to.is_none() {
        return Ok(s.phases[src_phase_idx].chunks[src_chunk_idx].tasks[src_task_idx].clone());
    }

    let (dst_phase_idx, dst_chunk_idx) = if let Some(ca) = to_chunk_addr {
        let (pi, ci, _) = find_chunk(s, ca)?;
        (pi, ci)
    } else {
        (src_phase_idx, src_chunk_idx)
    };

    let task = s.phases[src_phase_idx].chunks[src_chunk_idx]
        .tasks
        .remove(src_task_idx);

    let dst_len = s.phases[dst_phase_idx].chunks[dst_chunk_idx].tasks.len();
    let insert_idx = if let Some(pos) = to {
        (pos - 1).min(dst_len)
    } else {
        dst_len
    };

    s.phases[dst_phase_idx].chunks[dst_chunk_idx]
        .tasks
        .insert(insert_idx, task);

    Ok(s.phases[dst_phase_idx].chunks[dst_chunk_idx].tasks[insert_idx].clone())
}

// ── Existing autopilot helpers (unchanged) ────────────────────────────────────

/// Return the first chunk that is still open — `Pending` or `InProgress` —
/// ignoring `blocked_by` (the agent resolves blocking externally).
///
/// Read-only and idempotent: calling this N times returns the same chunk until
/// [`mark_task_done`] advances state.
pub fn next_pending_chunk(schedule: &Schedule) -> Option<&Chunk> {
    schedule
        .phases
        .iter()
        .flat_map(|p| p.chunks.iter())
        .find(|c| c.state.is_open())
}

/// Return the last chunk with state `Done` across all phases, in phase→chunk order.
pub fn prior_chunk(schedule: &Schedule) -> Option<&Chunk> {
    schedule
        .phases
        .iter()
        .flat_map(|p| p.chunks.iter())
        .rfind(|c| c.state == ScheduleState::Done)
}

/// Mark the current task as `Done`, auto-promoting the parent chunk and phase
/// when their respective children are all `Done`.
///
/// Returns `None` when no pending task exists (schedule exhausted).
/// Returns the completed task plus promotion flags so the caller can update
/// counters in `project.yaml`.
pub fn mark_task_done(schedule: &mut Schedule) -> Option<TaskCompletion> {
    // Indices (not refs) so the post-mutation promotion checks can re-borrow.
    let (phase_idx, chunk_idx, task_idx) = find_current_task_indices(schedule)?;

    schedule.phases[phase_idx].chunks[chunk_idx].tasks[task_idx].state = ScheduleState::Done;
    let task = schedule.phases[phase_idx].chunks[chunk_idx].tasks[task_idx].clone();

    let chunk_promoted = if schedule.phases[phase_idx].chunks[chunk_idx]
        .tasks
        .iter()
        .all(|t| t.state == ScheduleState::Done)
    {
        schedule.phases[phase_idx].chunks[chunk_idx].state = ScheduleState::Done;
        true
    } else {
        false
    };

    let phase_promoted = if chunk_promoted
        && schedule.phases[phase_idx]
            .chunks
            .iter()
            .all(|c| c.state == ScheduleState::Done)
    {
        schedule.phases[phase_idx].state = ScheduleState::Done;
        true
    } else {
        false
    };

    Some(TaskCompletion {
        task,
        chunk_promoted,
        phase_promoted,
    })
}

// Chunk predicate matches `next_pending_chunk`: callers rely on `task complete`
// advancing exactly the chunk `chunk-next` reported.
fn find_current_task_indices(schedule: &Schedule) -> Option<(usize, usize, usize)> {
    for (phase_idx, phase) in schedule.phases.iter().enumerate() {
        for (chunk_idx, chunk) in phase.chunks.iter().enumerate() {
            if !chunk.state.is_open() {
                continue;
            }
            if let Some(task_idx) = chunk
                .tasks
                .iter()
                .position(|t| t.state != ScheduleState::Done)
            {
                return Some((phase_idx, chunk_idx, task_idx));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pid() -> ProjectId {
        ProjectId::parse("test-project").unwrap()
    }

    fn make_schedule() -> Schedule {
        Schedule {
            project: pid(),
            phases: vec![
                Phase {
                    id: "phase-a".to_owned(),
                    description: "Phase A".to_owned(),
                    blocked_by: vec![],
                    state: ScheduleState::Pending,
                    chunks: vec![
                        Chunk {
                            id: "chunk-1".to_owned(),
                            description: "Chunk 1".to_owned(),
                            scenarios: vec![],
                            spec_refs: vec![],
                            blocked_by: vec![],
                            state: ScheduleState::Pending,
                            tasks: vec![
                                Task {
                                    id: "task-a".to_owned(),
                                    description: "Task A".to_owned(),
                                    state: ScheduleState::Pending,
                                    skill: None,
                                    inputs: None,
                                    outputs: None,
                                },
                                Task {
                                    id: "task-b".to_owned(),
                                    description: "Task B".to_owned(),
                                    state: ScheduleState::Done,
                                    skill: None,
                                    inputs: None,
                                    outputs: None,
                                },
                            ],
                        },
                        Chunk {
                            id: "chunk-2".to_owned(),
                            description: "Chunk 2".to_owned(),
                            scenarios: vec![],
                            spec_refs: vec![],
                            blocked_by: vec!["chunk-1".to_owned()],
                            state: ScheduleState::Done,
                            tasks: vec![],
                        },
                    ],
                },
                Phase {
                    id: "phase-b".to_owned(),
                    description: "Phase B".to_owned(),
                    blocked_by: vec!["phase-a".to_owned()],
                    state: ScheduleState::Pending,
                    chunks: vec![Chunk {
                        id: "chunk-1".to_owned(), // same slug as phase-a's chunk-1
                        description: "Chunk 1 in phase B".to_owned(),
                        scenarios: vec![],
                        spec_refs: vec![],
                        blocked_by: vec![],
                        state: ScheduleState::Pending,
                        tasks: vec![Task {
                            id: "task-a".to_owned(),
                            description: "Task A in phase B".to_owned(),
                            state: ScheduleState::Pending,
                            skill: None,
                            inputs: None,
                            outputs: None,
                        }],
                    }],
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

    // ── counters_for ──────────────────────────────────────────────────────────

    #[test]
    fn counters_for_counts_pending_and_done_correctly() {
        let s = make_schedule();
        // phase-a: chunk-1 (pending, 2 tasks: 1 pending + 1 done), chunk-2 (done, 0 tasks)
        // phase-b: chunk-1 (pending, 1 task pending)
        let c = counters_for(&s);
        assert_eq!(c.chunks_required, 3);
        assert_eq!(c.chunks_completed, 1);
        assert_eq!(c.tasks_required, 3);
        assert_eq!(c.tasks_completed, 1);
    }

    // ── phase mutations ───────────────────────────────────────────────────────

    #[test]
    fn add_phase_appends() {
        let mut s = make_schedule();
        let new_phase = Phase {
            id: "phase-c".to_owned(),
            description: "Phase C".to_owned(),
            blocked_by: vec![],
            state: ScheduleState::Pending,
            chunks: vec![],
        };
        add_phase(&mut s, new_phase);
        assert_eq!(s.phases.len(), 3);
        assert_eq!(s.phases[2].id, "phase-c");
    }

    #[test]
    fn update_phase_renames_and_rewrites_refs() {
        let mut s = make_schedule();
        let edit = PhaseEdit {
            new_id: Some("phase-alpha".to_owned()),
            ..Default::default()
        };
        update_phase(&mut s, "phase-a", edit).unwrap();
        assert_eq!(s.phases[0].id, "phase-alpha");
        // phase-b blocked_by must now say phase-alpha
        assert!(s.phases[1].blocked_by.contains(&"phase-alpha".to_owned()));
    }

    #[test]
    fn remove_phase_drops_dangling_refs() {
        let mut s = make_schedule();
        remove_phase(&mut s, "phase-a").unwrap();
        assert_eq!(s.phases.len(), 1);
        assert!(s.phases[0].blocked_by.is_empty());
    }

    #[test]
    fn move_phase_reorders() {
        let mut s = make_schedule();
        move_phase(&mut s, "phase-b", 1).unwrap();
        assert_eq!(s.phases[0].id, "phase-b");
        assert_eq!(s.phases[1].id, "phase-a");
    }

    // ── chunk mutations ───────────────────────────────────────────────────────

    #[test]
    fn add_chunk_under_phase() {
        let mut s = make_schedule();
        let chunk = Chunk {
            id: "chunk-new".to_owned(),
            description: "New chunk".to_owned(),
            scenarios: vec![],
            spec_refs: vec![],
            blocked_by: vec![],
            state: ScheduleState::Pending,
            tasks: vec![],
        };
        add_chunk(&mut s, "phase-a", chunk).unwrap();
        assert_eq!(s.phases[0].chunks.len(), 3);
        assert_eq!(s.phases[0].chunks[2].id, "chunk-new");
    }

    #[test]
    fn remove_chunk_cascades_and_drops_refs() {
        let mut s = make_schedule();
        // chunk-2 blocked_by chunk-1; removing chunk-1 should drop that ref
        remove_chunk(&mut s, "1.1").unwrap();
        assert!(s.phases[0].chunks[0].blocked_by.is_empty());
    }

    #[test]
    fn move_chunk_to_other_phase() {
        let mut s = make_schedule();
        // Move chunk-2 (1.2) to phase-b
        move_chunk(&mut s, "1.2", Some("phase-b"), None).unwrap();
        assert_eq!(s.phases[0].chunks.len(), 1);
        assert_eq!(s.phases[1].chunks.len(), 2);
    }

    // ── task mutations ────────────────────────────────────────────────────────

    #[test]
    fn add_task_under_chunk() {
        let mut s = make_schedule();
        let task = Task {
            id: "task-new".to_owned(),
            description: "New task".to_owned(),
            state: ScheduleState::Pending,
            skill: None,
            inputs: None,
            outputs: None,
        };
        add_task(&mut s, "1.1", task).unwrap();
        assert_eq!(s.phases[0].chunks[0].tasks.len(), 3);
        assert_eq!(s.phases[0].chunks[0].tasks[2].id, "task-new");
    }

    #[test]
    fn update_task_changes_state() {
        let mut s = make_schedule();
        let edit = TaskEdit {
            state: Some(ScheduleState::Done),
            ..Default::default()
        };
        let updated = update_task(&mut s, "1.1.1", edit).unwrap();
        assert_eq!(updated.state, ScheduleState::Done);
    }

    #[test]
    fn remove_task_removes_correct() {
        let mut s = make_schedule();
        let removed = remove_task(&mut s, "1.1.1").unwrap();
        assert_eq!(removed.id, "task-a");
        assert_eq!(s.phases[0].chunks[0].tasks.len(), 1);
    }

    #[test]
    fn move_task_to_other_chunk() {
        let mut s = make_schedule();
        // Move task-a (1.1.1) to chunk 2.1 (phase-b, chunk-1)
        move_task(&mut s, "1.1.1", Some("2.1"), None).unwrap();
        assert_eq!(s.phases[0].chunks[0].tasks.len(), 1);
        assert_eq!(s.phases[1].chunks[0].tasks.len(), 2);
    }

    // ── validate_no_state_regression ─────────────────────────────────────────

    #[test]
    fn validate_no_regression_passes_when_clean() {
        let old = make_schedule();
        let new = make_schedule();
        assert!(validate_no_state_regression(&old, &new).is_ok());
    }

    #[test]
    fn validate_regression_fails_when_done_becomes_pending() {
        let old = make_schedule();
        let mut new = make_schedule();
        // task-b in phase-a/chunk-1 is Done in old; set to Pending in new
        new.phases[0].chunks[0].tasks[1].state = ScheduleState::Pending;
        let err = validate_no_state_regression(&old, &new).unwrap_err();
        assert!(matches!(err, RexError::ReplaceWouldRegressState { .. }));
    }
}
