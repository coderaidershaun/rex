//! Counter-sync fitness function.
//!
//! The on-disk invariant: after any sequence of `rex project task complete`
//! calls, `project.yaml`'s `tasks_completed` / `chunks_completed` must agree
//! with the count of `Done` tasks / chunks in `schedule.json`.
//!
//! Drift here would silently corrupt progress reporting — the kind of bug
//! that fitness functions exist to make impossible.

use assert_cmd::Command;
use tempfile::TempDir;

use rex_cli::project::{PipelineStep, ProjectId, ProjectStore, ProjectYaml};
use rex_cli::schedule::{Chunk, Phase, Schedule, ScheduleState, Task};

fn rex_cmd() -> Command {
    Command::cargo_bin("rex").expect("rex binary must be built")
}

fn pending_task(id: &str) -> Task {
    Task {
        id: id.to_owned(),
        description: format!("{id} description"),
        state: ScheduleState::Pending,
        skill: None,
        inputs: None,
        outputs: None,
    }
}

fn pending_chunk(id: &str, tasks: Vec<Task>) -> Chunk {
    Chunk {
        id: id.to_owned(),
        description: format!("{id} description"),
        scenarios: vec![],
        spec_refs: vec![],
        blocked_by: vec![],
        state: ScheduleState::Pending,
        tasks,
    }
}

/// Seed a project with three chunks (3, 2, 1 tasks respectively) so the loop
/// exercises chunk promotion at varied positions.
fn seed(dir: &std::path::Path, project_id: &str) {
    let project = ProjectYaml {
        project_id: ProjectId::parse(project_id).unwrap(),
        category: "feature".to_owned(),
        title: Some("Counter Sync".to_owned()),
        subtitle: None,
        description: None,
        complexity: "medium".to_owned(),
        chunks_required: 3,
        chunks_completed: 0,
        tasks_required: 6,
        tasks_completed: 0,
        completed: false,
        is_autopilot: false,
        steps: vec![PipelineStep {
            step: "task-execution".to_owned(),
            required: true,
            skill: None,
            agent: None,
            instructions: None,
            inputs: None,
            outputs: None,
            completed: false,
        }],
    };
    let store = ProjectStore::new(dir);
    store.write_active(&project).unwrap();

    let schedule = Schedule {
        project: ProjectId::parse(project_id).unwrap(),
        phases: vec![Phase {
            id: "phase-one".to_owned(),
            description: "Single phase".to_owned(),
            blocked_by: vec![],
            state: ScheduleState::Pending,
            chunks: vec![
                pending_chunk(
                    "chunk-a",
                    vec![
                        pending_task("a-1"),
                        pending_task("a-2"),
                        pending_task("a-3"),
                    ],
                ),
                pending_chunk("chunk-b", vec![pending_task("b-1"), pending_task("b-2")]),
                pending_chunk("chunk-c", vec![pending_task("c-1")]),
            ],
        }],
    };
    store.write_schedule(&schedule).unwrap();
}

fn done_counts(schedule: &Schedule) -> (usize, usize) {
    let tasks = schedule
        .phases
        .iter()
        .flat_map(|p| p.chunks.iter())
        .flat_map(|c| c.tasks.iter())
        .filter(|t| t.state == ScheduleState::Done)
        .count();
    let chunks = schedule
        .phases
        .iter()
        .flat_map(|p| p.chunks.iter())
        .filter(|c| c.state == ScheduleState::Done)
        .count();
    (tasks, chunks)
}

/// FITNESS: across every step of a full task-complete walk, the counters in
/// `project.yaml` must equal the `Done` counts in `schedule.json`.
#[test]
fn task_complete_keeps_project_and_schedule_counters_in_sync() {
    let dir = TempDir::new().unwrap();
    seed(dir.path(), "counter-sync");
    let store = ProjectStore::new(dir.path());

    // Six tasks total — drive each one and check the invariant after every step.
    for step in 1..=6 {
        rex_cmd()
            .args(["project", "task", "complete"])
            .current_dir(dir.path())
            .assert()
            .success();

        let project = store.read_active().unwrap();
        let schedule = store.read_schedule().unwrap();
        let (done_tasks, done_chunks) = done_counts(&schedule);

        assert_eq!(
            project.tasks_completed as usize, done_tasks,
            "step {step}: project.tasks_completed must equal Done tasks in schedule"
        );
        assert_eq!(
            project.chunks_completed as usize, done_chunks,
            "step {step}: project.chunks_completed must equal Done chunks in schedule"
        );
    }

    // Final state: every task and every chunk Done.
    let project = store.read_active().unwrap();
    assert_eq!(project.tasks_completed, 6);
    assert_eq!(project.chunks_completed, 3);

    // Calling once more must print the no-active-task sentinel and not bump.
    rex_cmd()
        .args(["project", "task", "complete"])
        .current_dir(dir.path())
        .assert()
        .success();

    let project = store.read_active().unwrap();
    assert_eq!(
        project.tasks_completed, 6,
        "no-active-task path must not bump tasks_completed"
    );
    assert_eq!(
        project.chunks_completed, 3,
        "no-active-task path must not bump chunks_completed"
    );
}

/// FITNESS: counters in `project.yaml` must reflect schedule state across all schedule CRUD
/// mutations — replace → adds → moves — not just task-complete.
///
/// Drift here would silently corrupt progress reporting for any agent that checks
/// counters after editing the schedule, not just after task completion.
#[test]
fn schedule_crud_keeps_counters_in_sync() {
    let dir = TempDir::new().unwrap();
    let project_id = "crud-counter-sync";
    seed(dir.path(), project_id);
    let store = ProjectStore::new(dir.path());

    // Replace the entire schedule with a fresh 2-phase, 2-chunk, 3-task layout.
    let new_schedule = Schedule {
        project: ProjectId::parse(project_id).unwrap(),
        phases: vec![
            Phase {
                id: "phase-x".to_owned(),
                description: "Phase X".to_owned(),
                blocked_by: vec![],
                state: ScheduleState::Pending,
                chunks: vec![Chunk {
                    id: "chunk-p".to_owned(),
                    description: "Chunk P".to_owned(),
                    scenarios: vec![],
                    spec_refs: vec![],
                    blocked_by: vec![],
                    state: ScheduleState::Pending,
                    tasks: vec![pending_task("t-p1"), pending_task("t-p2")],
                }],
            },
            Phase {
                id: "phase-y".to_owned(),
                description: "Phase Y".to_owned(),
                blocked_by: vec![],
                state: ScheduleState::Pending,
                chunks: vec![Chunk {
                    id: "chunk-q".to_owned(),
                    description: "Chunk Q".to_owned(),
                    scenarios: vec![],
                    spec_refs: vec![],
                    blocked_by: vec![],
                    state: ScheduleState::Pending,
                    tasks: vec![pending_task("t-q1")],
                }],
            },
        ],
    };

    let file = dir.path().join("new_sched.json");
    std::fs::write(&file, serde_json::to_string_pretty(&new_schedule).unwrap()).unwrap();

    rex_cmd()
        .args([
            "project",
            "schedule",
            "replace",
            "--file",
            file.to_str().unwrap(),
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    let project = store.read_active().unwrap();
    assert_eq!(project.chunks_required, 2, "replace: chunks_required");
    assert_eq!(project.tasks_required, 3, "replace: tasks_required");
    assert_eq!(project.chunks_completed, 0, "replace: chunks_completed");
    assert_eq!(project.tasks_completed, 0, "replace: tasks_completed");

    // Add a third chunk via CLI.
    rex_cmd()
        .args([
            "project",
            "schedule",
            "chunk",
            "add",
            "--phase",
            "1",
            "--description",
            "Extra chunk",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    rex_cmd()
        .args([
            "project",
            "schedule",
            "task",
            "add",
            "--chunk",
            "1.2",
            "--description",
            "Extra task",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    let project = store.read_active().unwrap();
    assert_eq!(project.chunks_required, 3, "after add: chunks_required");
    assert_eq!(project.tasks_required, 4, "after add: tasks_required");

    // Move a task from chunk 1.1 to chunk 1.2.
    rex_cmd()
        .args([
            "project",
            "schedule",
            "task",
            "move",
            "t-p1",
            "--to-chunk",
            "1.2",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    // Counters must be unchanged by move (same totals).
    let project = store.read_active().unwrap();
    assert_eq!(
        project.tasks_required, 4,
        "after move: tasks_required unchanged"
    );

    // Drive every remaining task to done via `task update --state done` and
    // assert the counter invariant after each step. `task complete` already
    // pins the autopilot path; this run pins the CRUD path.
    let task_addrs = ["1.1.1", "1.2.1", "1.2.2", "2.1.1"]; // one per remaining task
    for addr in task_addrs {
        rex_cmd()
            .args([
                "project", "schedule", "task", "update", addr, "--state", "done",
            ])
            .current_dir(dir.path())
            .assert()
            .success();

        let project = store.read_active().unwrap();
        let schedule = store.read_schedule().unwrap();
        let (done_tasks, done_chunks) = done_counts(&schedule);
        assert_eq!(
            project.tasks_completed as usize, done_tasks,
            "after task update {addr}: project.tasks_completed must equal Done tasks"
        );
        assert_eq!(
            project.chunks_completed as usize, done_chunks,
            "after task update {addr}: project.chunks_completed must equal Done chunks"
        );
    }

    // Promote chunks via `chunk update --state done`; counters must follow.
    let chunk_addrs = ["1.1", "1.2", "2.1"];
    for addr in chunk_addrs {
        rex_cmd()
            .args([
                "project", "schedule", "chunk", "update", addr, "--state", "done",
            ])
            .current_dir(dir.path())
            .assert()
            .success();

        let project = store.read_active().unwrap();
        let schedule = store.read_schedule().unwrap();
        let (done_tasks, done_chunks) = done_counts(&schedule);
        assert_eq!(
            project.tasks_completed as usize, done_tasks,
            "after chunk update {addr}: tasks counter still in sync"
        );
        assert_eq!(
            project.chunks_completed as usize, done_chunks,
            "after chunk update {addr}: chunks counter still in sync"
        );
    }

    // Final state: every task and every chunk Done.
    let project = store.read_active().unwrap();
    assert_eq!(project.tasks_completed, 4, "all tasks done");
    assert_eq!(project.chunks_completed, 3, "all chunks done");
}
