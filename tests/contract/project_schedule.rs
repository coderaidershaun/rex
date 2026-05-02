use tempfile::TempDir;

use rex_cli::project::{ProjectId, ProjectStore, ProjectYaml};
use rex_cli::schedule::{
    Chunk, Phase, Schedule, ScheduleState, Task, counters_for, mark_task_done,
    rewrite_blocked_by_at_chunk_level, rewrite_blocked_by_at_phase_level, unique_slug,
};

fn make_active_project(dir: &std::path::Path, project_id: &str) {
    let project = ProjectYaml {
        project_id: ProjectId::parse(project_id).unwrap(),
        category: "feature".to_owned(),
        title: Some("Test".to_owned()),
        subtitle: None,
        description: None,
        complexity: "medium".to_owned(),
        chunks_required: 2,
        chunks_completed: 0,
        tasks_required: 2,
        tasks_completed: 0,
        completed: false,
        is_autopilot: false,
        steps: vec![],
    };
    ProjectStore::new(dir).write_active(&project).unwrap();
}

fn make_schedule(project_id: &str) -> Schedule {
    Schedule {
        project: ProjectId::parse(project_id).unwrap(),
        phases: vec![Phase {
            id: "phase-one".to_owned(),
            description: "First phase".to_owned(),
            blocked_by: vec![],
            state: ScheduleState::Pending,
            chunks: vec![
                Chunk {
                    id: "chunk-alpha".to_owned(),
                    description: "Alpha chunk".to_owned(),
                    scenarios: vec!["scenario one".to_owned()],
                    spec_refs: vec![],
                    blocked_by: vec![],
                    state: ScheduleState::Pending,
                    tasks: vec![
                        Task {
                            id: "task-one".to_owned(),
                            description: "First task".to_owned(),
                            state: ScheduleState::Pending,
                            skill: None,
                            inputs: None,
                            outputs: None,
                        },
                        Task {
                            id: "task-two".to_owned(),
                            description: "Second task".to_owned(),
                            state: ScheduleState::Pending,
                            skill: None,
                            inputs: None,
                            outputs: None,
                        },
                    ],
                },
                Chunk {
                    id: "chunk-beta".to_owned(),
                    description: "Beta chunk".to_owned(),
                    scenarios: vec![],
                    spec_refs: vec![],
                    blocked_by: vec![],
                    state: ScheduleState::Pending,
                    tasks: vec![Task {
                        id: "task-three".to_owned(),
                        description: "Third task".to_owned(),
                        state: ScheduleState::Pending,
                        skill: None,
                        inputs: None,
                        outputs: None,
                    }],
                },
            ],
        }],
    }
}

/// CONTRACT: Schedule written via ProjectStore survives a JSON roundtrip.
#[test]
fn schedule_roundtrip_via_store() {
    let dir = TempDir::new().unwrap();
    make_active_project(dir.path(), "roundtrip-project");

    let store = ProjectStore::new(dir.path());
    let original = make_schedule("roundtrip-project");
    store.write_schedule(&original).unwrap();

    let loaded = store.read_schedule().unwrap();
    assert_eq!(loaded.project, original.project);
    assert_eq!(loaded.phases.len(), original.phases.len());
    assert_eq!(loaded.phases[0].chunks[0].tasks[0].id, "task-one");
    assert_eq!(
        loaded.phases[0].chunks[0].tasks[0].state,
        ScheduleState::Pending
    );
}

/// CONTRACT: ScheduleState::InProgress serializes as "in-progress" (kebab-case).
#[test]
fn schedule_state_serializes_kebab_case() {
    let state = ScheduleState::InProgress;
    let json = serde_json::to_string(&state).unwrap();
    assert_eq!(
        json, r#""in-progress""#,
        "InProgress must serialize as \"in-progress\""
    );
}

/// CONTRACT: mark_task_done promotes chunk to Done when it was the last task.
#[test]
fn mark_task_done_promotes_chunk_when_last() {
    let mut schedule = make_schedule("promote-project");

    // Complete task-one (not the last in chunk-alpha).
    let completion = mark_task_done(&mut schedule).unwrap();
    assert_eq!(completion.task.id, "task-one");
    assert!(
        !completion.chunk_promoted,
        "chunk must not promote until last task done"
    );
    assert_eq!(schedule.phases[0].chunks[0].state, ScheduleState::Pending);

    // Complete task-two (last in chunk-alpha) — chunk should auto-promote.
    let completion = mark_task_done(&mut schedule).unwrap();
    assert_eq!(completion.task.id, "task-two");
    assert!(
        completion.chunk_promoted,
        "chunk must promote when last task done"
    );
    assert_eq!(
        schedule.phases[0].chunks[0].state,
        ScheduleState::Done,
        "chunk-alpha must be Done after last task completes"
    );
}

/// CONTRACT: mark_task_done promotes phase to Done when its last chunk completes.
#[test]
fn mark_task_done_promotes_phase_when_last_chunk_done() {
    let mut schedule = make_schedule("phase-promote-project");

    // Drive every task to Done — three tasks across two chunks in one phase.
    let _ = mark_task_done(&mut schedule).unwrap(); // task-one
    let _ = mark_task_done(&mut schedule).unwrap(); // task-two → chunk-alpha promotes

    let final_completion = mark_task_done(&mut schedule).unwrap(); // task-three
    assert_eq!(final_completion.task.id, "task-three");
    assert!(
        final_completion.chunk_promoted,
        "chunk-beta must promote when its sole task completes"
    );
    assert!(
        final_completion.phase_promoted,
        "phase must promote when its last chunk completes"
    );
    assert_eq!(
        schedule.phases[0].state,
        ScheduleState::Done,
        "phase-one must be Done after all chunks complete"
    );
}

// ── New pure-helper contract tests ────────────────────────────────────────────

fn two_phase_schedule() -> Schedule {
    Schedule {
        project: ProjectId::parse("pure-test").unwrap(),
        phases: vec![
            Phase {
                id: "phase-a".to_owned(),
                description: "Phase A".to_owned(),
                blocked_by: vec![],
                state: ScheduleState::Pending,
                chunks: vec![
                    Chunk {
                        id: "chunk-one".to_owned(),
                        description: "Chunk One".to_owned(),
                        scenarios: vec![],
                        spec_refs: vec![],
                        blocked_by: vec![],
                        state: ScheduleState::Done,
                        tasks: vec![Task {
                            id: "task-a".to_owned(),
                            description: "Task A".to_owned(),
                            state: ScheduleState::Done,
                            skill: None,
                            inputs: None,
                            outputs: None,
                        }],
                    },
                    Chunk {
                        id: "chunk-two".to_owned(),
                        description: "Chunk Two".to_owned(),
                        scenarios: vec![],
                        spec_refs: vec![],
                        blocked_by: vec!["chunk-one".to_owned()],
                        state: ScheduleState::Pending,
                        tasks: vec![Task {
                            id: "task-b".to_owned(),
                            description: "Task B".to_owned(),
                            state: ScheduleState::Pending,
                            skill: None,
                            inputs: None,
                            outputs: None,
                        }],
                    },
                ],
            },
            Phase {
                id: "phase-b".to_owned(),
                description: "Phase B".to_owned(),
                blocked_by: vec!["phase-a".to_owned()],
                state: ScheduleState::Pending,
                chunks: vec![],
            },
        ],
    }
}

/// CONTRACT: `unique_slug` returns the desired slug unchanged when no collision.
#[test]
fn unique_slug_returns_input_when_no_collision() {
    let result = unique_slug(&["alpha", "beta"], "gamma");
    assert_eq!(result, "gamma");
}

/// CONTRACT: `unique_slug` appends `-2` on first collision.
#[test]
fn unique_slug_appends_numeric_suffix_on_collision() {
    let result = unique_slug(&["alpha", "beta"], "alpha");
    assert_eq!(result, "alpha-2");
}

/// CONTRACT: `unique_slug` skips already-taken suffixes.
#[test]
fn unique_slug_skips_existing_suffixes() {
    let result = unique_slug(&["alpha", "alpha-2", "alpha-3"], "alpha");
    assert_eq!(result, "alpha-4");
}

/// CONTRACT: `rewrite_blocked_by_at_phase_level` rewrites every matching entry.
#[test]
fn rewrite_blocked_by_renames_every_match_at_level() {
    let mut s = two_phase_schedule();
    rewrite_blocked_by_at_phase_level(&mut s, "phase-a", Some("phase-alpha"));
    assert!(s.phases[1].blocked_by.contains(&"phase-alpha".to_owned()));
    assert!(!s.phases[1].blocked_by.contains(&"phase-a".to_owned()));
}

/// CONTRACT: `rewrite_blocked_by_at_phase_level` with `None` drops the reference.
#[test]
fn rewrite_blocked_by_drops_when_target_removed() {
    let mut s = two_phase_schedule();
    rewrite_blocked_by_at_phase_level(&mut s, "phase-a", None);
    assert!(s.phases[1].blocked_by.is_empty());
}

/// CONTRACT: `rewrite_blocked_by_at_chunk_level` drops chunk ref on remove.
#[test]
fn rewrite_blocked_by_at_chunk_level_drops_on_remove() {
    let mut s = two_phase_schedule();
    rewrite_blocked_by_at_chunk_level(&mut s, 0, "chunk-one", None);
    assert!(s.phases[0].chunks[1].blocked_by.is_empty());
}

/// CONTRACT: `counters_for` correctly counts pending and done items.
#[test]
fn counters_for_counts_pending_and_done_correctly() {
    let s = two_phase_schedule();
    // phase-a: chunk-one (Done, 1 done task) + chunk-two (Pending, 1 pending task)
    // phase-b: no chunks
    let c = counters_for(&s);
    assert_eq!(c.chunks_required, 2, "two chunks total");
    assert_eq!(c.chunks_completed, 1, "one done chunk");
    assert_eq!(c.tasks_required, 2, "two tasks total");
    assert_eq!(c.tasks_completed, 1, "one done task");
}

/// CONTRACT: `write_schedule_with_counters` updates project.yaml counters from schedule state.
#[test]
fn write_schedule_with_counters_syncs_project_yaml() {
    let dir = TempDir::new().unwrap();
    make_active_project(dir.path(), "counter-sync-test");

    let store = ProjectStore::new(dir.path());
    let schedule = two_phase_schedule();

    store.write_schedule_with_counters(&schedule).unwrap();

    let project = store.read_active().unwrap();
    assert_eq!(project.chunks_required, 2);
    assert_eq!(project.chunks_completed, 1);
    assert_eq!(project.tasks_required, 2);
    assert_eq!(project.tasks_completed, 1);
}
