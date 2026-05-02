use assert_cmd::Command;
use rex_cli::project::{PipelineStep, ProjectId, ProjectStore, ProjectYaml};
use rex_cli::schedule::{Chunk, Phase, Schedule, ScheduleState, Task};
use tempfile::TempDir;

fn rex_cmd() -> Command {
    Command::cargo_bin("rex").expect("rex binary must be built")
}

/// Seed an active project and an empty schedule (one phase, no chunks) for tests.
fn setup(dir: &std::path::Path, project_id: &str) {
    let project = ProjectYaml {
        project_id: ProjectId::parse(project_id).unwrap(),
        category: "feature".to_owned(),
        title: Some("Schedule CLI Test".to_owned()),
        subtitle: None,
        description: None,
        complexity: "medium".to_owned(),
        chunks_required: 0,
        chunks_completed: 0,
        tasks_required: 0,
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
        phases: vec![],
    };
    store.write_schedule(&schedule).unwrap();
}

/// Seed project + a schedule with two phases, two chunks in phase-a, one in phase-b,
/// each with tasks, for multi-entity move/cascade tests.
fn setup_rich(dir: &std::path::Path, project_id: &str) {
    let project = ProjectYaml {
        project_id: ProjectId::parse(project_id).unwrap(),
        category: "feature".to_owned(),
        title: Some("Rich Schedule".to_owned()),
        subtitle: None,
        description: None,
        complexity: "medium".to_owned(),
        chunks_required: 3,
        chunks_completed: 0,
        tasks_required: 4,
        tasks_completed: 0,
        completed: false,
        is_autopilot: false,
        steps: vec![],
    };
    let store = ProjectStore::new(dir);
    store.write_active(&project).unwrap();

    let pending_task = |id: &str| Task {
        id: id.to_owned(),
        description: format!("{id} desc"),
        state: ScheduleState::Pending,
        skill: None,
        inputs: None,
        outputs: None,
    };
    let schedule = Schedule {
        project: ProjectId::parse(project_id).unwrap(),
        phases: vec![
            Phase {
                id: "phase-a".to_owned(),
                description: "Phase A".to_owned(),
                blocked_by: vec![],
                state: ScheduleState::Pending,
                chunks: vec![
                    Chunk {
                        id: "chunk-x".to_owned(),
                        description: "Chunk X".to_owned(),
                        scenarios: vec![],
                        spec_refs: vec![],
                        blocked_by: vec![],
                        state: ScheduleState::Pending,
                        tasks: vec![pending_task("task-1"), pending_task("task-2")],
                    },
                    Chunk {
                        id: "chunk-y".to_owned(),
                        description: "Chunk Y".to_owned(),
                        scenarios: vec![],
                        spec_refs: vec![],
                        blocked_by: vec!["chunk-x".to_owned()],
                        state: ScheduleState::Pending,
                        tasks: vec![pending_task("task-3")],
                    },
                ],
            },
            Phase {
                id: "phase-b".to_owned(),
                description: "Phase B".to_owned(),
                blocked_by: vec!["phase-a".to_owned()],
                state: ScheduleState::Pending,
                chunks: vec![Chunk {
                    id: "chunk-z".to_owned(),
                    description: "Chunk Z".to_owned(),
                    scenarios: vec![],
                    spec_refs: vec![],
                    blocked_by: vec![],
                    state: ScheduleState::Pending,
                    tasks: vec![pending_task("task-4")],
                }],
            },
        ],
    };
    store.write_schedule(&schedule).unwrap();
}

fn read_schedule(dir: &std::path::Path) -> Schedule {
    ProjectStore::new(dir).read_schedule().unwrap()
}

fn read_project(dir: &std::path::Path) -> ProjectYaml {
    ProjectStore::new(dir).read_active().unwrap()
}

// ── phase add ─────────────────────────────────────────────────────────────────

#[test]
fn phase_add_appends_and_returns_json() {
    let dir = TempDir::new().unwrap();
    setup(dir.path(), "phase-add-test");

    let output = rex_cmd()
        .args([
            "project",
            "schedule",
            "phase",
            "add",
            "--description",
            "Market data ingestion",
        ])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(
        json.get("description").and_then(|v| v.as_str()),
        Some("Market data ingestion")
    );
    assert!(json.get("id").is_some());

    let schedule = read_schedule(dir.path());
    assert_eq!(schedule.phases.len(), 1);
}

#[test]
fn phase_add_collision_appends_suffix() {
    let dir = TempDir::new().unwrap();
    setup(dir.path(), "phase-collision-test");

    rex_cmd()
        .args([
            "project",
            "schedule",
            "phase",
            "add",
            "--description",
            "Phase one",
            "--id",
            "phase-one",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    let output = rex_cmd()
        .args([
            "project",
            "schedule",
            "phase",
            "add",
            "--description",
            "Also phase one",
            "--id",
            "phase-one",
        ])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(json.get("id").and_then(|v| v.as_str()), Some("phase-one-2"));
}

// ── phase update ──────────────────────────────────────────────────────────────

#[test]
fn phase_update_rename_rewrites_refs() {
    let dir = TempDir::new().unwrap();
    setup_rich(dir.path(), "phase-update-test");

    rex_cmd()
        .args([
            "project",
            "schedule",
            "phase",
            "update",
            "phase-a",
            "--id",
            "phase-alpha",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    let schedule = read_schedule(dir.path());
    assert_eq!(schedule.phases[0].id, "phase-alpha");
    // phase-b blocked_by must be rewritten
    assert!(
        schedule.phases[1]
            .blocked_by
            .contains(&"phase-alpha".to_owned())
    );
    assert!(
        !schedule.phases[1]
            .blocked_by
            .contains(&"phase-a".to_owned())
    );
}

// ── phase remove ──────────────────────────────────────────────────────────────

#[test]
fn phase_remove_drops_dangling_refs() {
    let dir = TempDir::new().unwrap();
    setup_rich(dir.path(), "phase-remove-test");

    rex_cmd()
        .args(["project", "schedule", "phase", "remove", "phase-a"])
        .current_dir(dir.path())
        .assert()
        .success();

    let schedule = read_schedule(dir.path());
    assert_eq!(schedule.phases.len(), 1);
    assert_eq!(schedule.phases[0].id, "phase-b");
    // phase-b's blocked_by["phase-a"] must be dropped
    assert!(schedule.phases[0].blocked_by.is_empty());
}

// ── phase move ────────────────────────────────────────────────────────────────

#[test]
fn phase_move_reorders() {
    let dir = TempDir::new().unwrap();
    setup_rich(dir.path(), "phase-move-test");

    rex_cmd()
        .args([
            "project", "schedule", "phase", "move", "phase-b", "--to", "1",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    let schedule = read_schedule(dir.path());
    assert_eq!(schedule.phases[0].id, "phase-b");
    assert_eq!(schedule.phases[1].id, "phase-a");
}

// ── chunk add ─────────────────────────────────────────────────────────────────

#[test]
fn chunk_add_under_phase() {
    let dir = TempDir::new().unwrap();
    setup(dir.path(), "chunk-add-test");

    rex_cmd()
        .args([
            "project",
            "schedule",
            "phase",
            "add",
            "--description",
            "Phase one",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    let output = rex_cmd()
        .args([
            "project",
            "schedule",
            "chunk",
            "add",
            "--phase",
            "1",
            "--description",
            "Gap detect + recover",
            "--scenario",
            "scenario one",
            "--spec-ref",
            "docs/spec.md",
        ])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(
        json.get("description").and_then(|v| v.as_str()),
        Some("Gap detect + recover")
    );

    let schedule = read_schedule(dir.path());
    assert_eq!(schedule.phases[0].chunks.len(), 1);
    assert_eq!(schedule.phases[0].chunks[0].scenarios.len(), 1);
    assert_eq!(schedule.phases[0].chunks[0].spec_refs.len(), 1);
}

// ── chunk update ──────────────────────────────────────────────────────────────

#[test]
fn chunk_update_appends_scenarios() {
    let dir = TempDir::new().unwrap();
    setup_rich(dir.path(), "chunk-update-test");

    rex_cmd()
        .args([
            "project",
            "schedule",
            "chunk",
            "update",
            "1.1",
            "--scenario",
            "new scenario",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    let schedule = read_schedule(dir.path());
    assert_eq!(schedule.phases[0].chunks[0].scenarios, vec!["new scenario"]);
}

// ── chunk remove ──────────────────────────────────────────────────────────────

#[test]
fn chunk_remove_cascades_to_tasks() {
    let dir = TempDir::new().unwrap();
    setup_rich(dir.path(), "chunk-remove-test");

    rex_cmd()
        .args(["project", "schedule", "chunk", "remove", "1.1"])
        .current_dir(dir.path())
        .assert()
        .success();

    let schedule = read_schedule(dir.path());
    assert_eq!(schedule.phases[0].chunks.len(), 1);
    // chunk-y's blocked_by["chunk-x"] must be dropped
    assert!(schedule.phases[0].chunks[0].blocked_by.is_empty());
}

// ── chunk move ────────────────────────────────────────────────────────────────

#[test]
fn chunk_move_to_other_phase() {
    let dir = TempDir::new().unwrap();
    setup_rich(dir.path(), "chunk-move-test");

    rex_cmd()
        .args([
            "project",
            "schedule",
            "chunk",
            "move",
            "1.2",
            "--to-phase",
            "phase-b",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    let schedule = read_schedule(dir.path());
    assert_eq!(schedule.phases[0].chunks.len(), 1);
    assert_eq!(schedule.phases[1].chunks.len(), 2);
}

// ── task add ──────────────────────────────────────────────────────────────────

#[test]
fn task_add_under_chunk() {
    let dir = TempDir::new().unwrap();
    setup_rich(dir.path(), "task-add-test");

    let output = rex_cmd()
        .args([
            "project",
            "schedule",
            "task",
            "add",
            "--chunk",
            "1.1",
            "--description",
            "Unit tests for gap detection",
            "--skill",
            "rex-code-tests-unit-testing",
        ])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(
        json.get("description").and_then(|v| v.as_str()),
        Some("Unit tests for gap detection")
    );
    assert_eq!(
        json.get("skill").and_then(|v| v.as_str()),
        Some("rex-code-tests-unit-testing")
    );

    let schedule = read_schedule(dir.path());
    assert_eq!(schedule.phases[0].chunks[0].tasks.len(), 3);
}

// ── task update ───────────────────────────────────────────────────────────────

#[test]
fn task_update_changes_state() {
    let dir = TempDir::new().unwrap();
    setup_rich(dir.path(), "task-update-test");

    rex_cmd()
        .args([
            "project", "schedule", "task", "update", "task-1", "--state", "done",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    let schedule = read_schedule(dir.path());
    assert_eq!(
        schedule.phases[0].chunks[0].tasks[0].state,
        ScheduleState::Done
    );
}

// ── task remove ───────────────────────────────────────────────────────────────

#[test]
fn task_remove() {
    let dir = TempDir::new().unwrap();
    setup_rich(dir.path(), "task-remove-test");

    rex_cmd()
        .args(["project", "schedule", "task", "remove", "task-1"])
        .current_dir(dir.path())
        .assert()
        .success();

    let schedule = read_schedule(dir.path());
    assert_eq!(schedule.phases[0].chunks[0].tasks.len(), 1);
    assert_eq!(schedule.phases[0].chunks[0].tasks[0].id, "task-2");
}

// ── task move ─────────────────────────────────────────────────────────────────

#[test]
fn task_move_to_other_chunk() {
    let dir = TempDir::new().unwrap();
    setup_rich(dir.path(), "task-move-test");

    rex_cmd()
        .args([
            "project",
            "schedule",
            "task",
            "move",
            "task-1",
            "--to-chunk",
            "1.2",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    let schedule = read_schedule(dir.path());
    assert_eq!(schedule.phases[0].chunks[0].tasks.len(), 1);
    assert_eq!(schedule.phases[0].chunks[1].tasks.len(), 2);
}

// ── replace ───────────────────────────────────────────────────────────────────

#[test]
fn replace_atomic_swaps_full_schedule() {
    let dir = TempDir::new().unwrap();
    setup(dir.path(), "replace-test");

    let new_schedule = Schedule {
        project: ProjectId::parse("replace-test").unwrap(),
        phases: vec![Phase {
            id: "phase-new".to_owned(),
            description: "New Phase".to_owned(),
            blocked_by: vec![],
            state: ScheduleState::Pending,
            chunks: vec![Chunk {
                id: "chunk-new".to_owned(),
                description: "New Chunk".to_owned(),
                scenarios: vec!["scenario".to_owned()],
                spec_refs: vec![],
                blocked_by: vec![],
                state: ScheduleState::Pending,
                tasks: vec![Task {
                    id: "task-new".to_owned(),
                    description: "New Task".to_owned(),
                    state: ScheduleState::Pending,
                    skill: None,
                    inputs: None,
                    outputs: None,
                }],
            }],
        }],
    };

    let file = dir.path().join("new_schedule.json");
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

    let schedule = read_schedule(dir.path());
    assert_eq!(schedule.phases.len(), 1);
    assert_eq!(schedule.phases[0].id, "phase-new");
}

#[test]
fn replace_rejects_state_regression() {
    let dir = TempDir::new().unwrap();
    setup_rich(dir.path(), "replace-regression-test");

    // Mark task-1 as done in the current schedule.
    rex_cmd()
        .args([
            "project", "schedule", "task", "update", "task-1", "--state", "done",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    // Now attempt to replace with a schedule that has task-1 as pending.
    let regressed_schedule = Schedule {
        project: ProjectId::parse("replace-regression-test").unwrap(),
        phases: vec![Phase {
            id: "phase-a".to_owned(),
            description: "Phase A".to_owned(),
            blocked_by: vec![],
            state: ScheduleState::Pending,
            chunks: vec![Chunk {
                id: "chunk-x".to_owned(),
                description: "Chunk X".to_owned(),
                scenarios: vec![],
                spec_refs: vec![],
                blocked_by: vec![],
                state: ScheduleState::Pending,
                tasks: vec![Task {
                    id: "task-1".to_owned(),
                    description: "task-1 desc".to_owned(),
                    state: ScheduleState::Pending, // regressed from done
                    skill: None,
                    inputs: None,
                    outputs: None,
                }],
            }],
        }],
    };

    let file = dir.path().join("regressed.json");
    std::fs::write(
        &file,
        serde_json::to_string_pretty(&regressed_schedule).unwrap(),
    )
    .unwrap();

    let stderr = rex_cmd()
        .args([
            "project",
            "schedule",
            "replace",
            "--file",
            file.to_str().unwrap(),
        ])
        .current_dir(dir.path())
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    let stderr_s = String::from_utf8(stderr).unwrap();
    // The agent must see which item regressed so it can fix the input.
    assert!(
        stderr_s.contains("task-1"),
        "stderr must name the regressed item; got: {stderr_s}"
    );
    assert!(
        stderr_s.contains("regress"),
        "stderr must explain it's a regression; got: {stderr_s}"
    );
}

/// Replace must succeed when the new schedule keeps every existing `done` item
/// in the `done` state — the regression check should not block benign re-imports
/// (e.g. an agent re-publishes the schedule after editing only pending items).
#[test]
fn replace_accepts_benign_reimport_with_done_intact() {
    let dir = TempDir::new().unwrap();
    setup_rich(dir.path(), "replace-benign-test");

    // Mark task-1 done in the live schedule.
    rex_cmd()
        .args([
            "project", "schedule", "task", "update", "task-1", "--state", "done",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    // Re-import a schedule that keeps task-1 done (even with edits to other tasks).
    let benign_schedule = Schedule {
        project: ProjectId::parse("replace-benign-test").unwrap(),
        phases: vec![Phase {
            id: "phase-a".to_owned(),
            description: "Phase A revised".to_owned(), // edit allowed
            blocked_by: vec![],
            state: ScheduleState::Pending,
            chunks: vec![Chunk {
                id: "chunk-x".to_owned(),
                description: "Chunk X".to_owned(),
                scenarios: vec![],
                spec_refs: vec![],
                blocked_by: vec![],
                state: ScheduleState::Pending,
                tasks: vec![Task {
                    id: "task-1".to_owned(),
                    description: "task-1 desc".to_owned(),
                    state: ScheduleState::Done, // preserved
                    skill: None,
                    inputs: None,
                    outputs: None,
                }],
            }],
        }],
    };
    let file = dir.path().join("benign.json");
    std::fs::write(
        &file,
        serde_json::to_string_pretty(&benign_schedule).unwrap(),
    )
    .unwrap();

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
}

/// CLI must surface the candidate dotted positions when a bare slug is ambiguous,
/// so the agent can re-call with `1.1` or `2.1`.
#[test]
fn ambiguous_slug_emits_candidate_positions() {
    let dir = TempDir::new().unwrap();
    setup(dir.path(), "ambiguous-test");

    // Two phases, each with a chunk named "shared" — bare slug `shared` is ambiguous.
    rex_cmd()
        .args([
            "project",
            "schedule",
            "phase",
            "add",
            "--description",
            "P1",
            "--id",
            "p1",
        ])
        .current_dir(dir.path())
        .assert()
        .success();
    rex_cmd()
        .args([
            "project",
            "schedule",
            "phase",
            "add",
            "--description",
            "P2",
            "--id",
            "p2",
        ])
        .current_dir(dir.path())
        .assert()
        .success();
    rex_cmd()
        .args([
            "project",
            "schedule",
            "chunk",
            "add",
            "--phase",
            "p1",
            "--description",
            "Shared",
            "--id",
            "shared",
        ])
        .current_dir(dir.path())
        .assert()
        .success();
    rex_cmd()
        .args([
            "project",
            "schedule",
            "chunk",
            "add",
            "--phase",
            "p2",
            "--description",
            "Shared",
            "--id",
            "shared",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    let stderr = rex_cmd()
        .args([
            "project",
            "schedule",
            "chunk",
            "update",
            "shared",
            "--description",
            "ambiguous",
        ])
        .current_dir(dir.path())
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    let stderr_s = String::from_utf8(stderr).unwrap();
    assert!(
        stderr_s.contains("1.1") && stderr_s.contains("2.1"),
        "stderr must list both candidate positions; got: {stderr_s}"
    );
    assert!(
        stderr_s.contains("ambiguous"),
        "stderr must mention ambiguity; got: {stderr_s}"
    );
}

// ── counter sync ──────────────────────────────────────────────────────────────

#[test]
fn every_mutation_recomputes_counters_in_project_yaml() {
    let dir = TempDir::new().unwrap();
    setup(dir.path(), "counter-mutation-test");

    // Initial state: 0 chunks, 0 tasks.
    let project = read_project(dir.path());
    assert_eq!(project.chunks_required, 0);
    assert_eq!(project.tasks_required, 0);

    // Add a phase + chunk + two tasks.
    rex_cmd()
        .args(["project", "schedule", "phase", "add", "--description", "P1"])
        .current_dir(dir.path())
        .assert()
        .success();

    rex_cmd()
        .args([
            "project",
            "schedule",
            "chunk",
            "add",
            "--phase",
            "1",
            "--description",
            "C1",
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
            "1.1",
            "--description",
            "T1",
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
            "1.1",
            "--description",
            "T2",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    let project = read_project(dir.path());
    assert_eq!(project.chunks_required, 1);
    assert_eq!(project.tasks_required, 2);
    assert_eq!(project.chunks_completed, 0);
    assert_eq!(project.tasks_completed, 0);

    // Mark both tasks done.
    rex_cmd()
        .args([
            "project", "schedule", "task", "update", "1.1.1", "--state", "done",
        ])
        .current_dir(dir.path())
        .assert()
        .success();
    rex_cmd()
        .args([
            "project", "schedule", "task", "update", "1.1.2", "--state", "done",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    // Mark chunk done.
    rex_cmd()
        .args([
            "project", "schedule", "chunk", "update", "1.1", "--state", "done",
        ])
        .current_dir(dir.path())
        .assert()
        .success();

    let project = read_project(dir.path());
    assert_eq!(project.chunks_completed, 1);
    assert_eq!(project.tasks_completed, 2);
}

// ── address resolution ────────────────────────────────────────────────────────

#[test]
fn address_resolves_slug_or_dotted_position() {
    let dir = TempDir::new().unwrap();
    setup_rich(dir.path(), "addr-test");

    // Address by slug.
    let out_slug = rex_cmd()
        .args([
            "project",
            "schedule",
            "phase",
            "update",
            "phase-a",
            "--description",
            "Updated by slug",
        ])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&out_slug).unwrap();
    assert_eq!(
        json.get("description").and_then(|v| v.as_str()),
        Some("Updated by slug")
    );

    // Address by dotted position.
    let out_pos = rex_cmd()
        .args([
            "project",
            "schedule",
            "phase",
            "update",
            "2",
            "--description",
            "Updated by position",
        ])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&out_pos).unwrap();
    assert_eq!(
        json.get("description").and_then(|v| v.as_str()),
        Some("Updated by position")
    );
}

// ── id validation ─────────────────────────────────────────────────────────────

#[test]
fn phase_add_rejects_id_with_dot() {
    let dir = TempDir::new().unwrap();
    setup(dir.path(), "dot-id-test");

    rex_cmd()
        .args([
            "project",
            "schedule",
            "phase",
            "add",
            "--description",
            "Should fail",
            "--id",
            "bad.id",
        ])
        .current_dir(dir.path())
        .assert()
        .failure();
}

// ── show ──────────────────────────────────────────────────────────────────────

#[test]
fn schedule_show_returns_full_schedule_json() {
    let dir = TempDir::new().unwrap();
    setup_rich(dir.path(), "schedule-show-test");

    let output = rex_cmd()
        .args(["project", "schedule", "show"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert!(json.get("phases").is_some());
    let phases = json["phases"].as_array().unwrap();
    assert_eq!(phases.len(), 2);
}
