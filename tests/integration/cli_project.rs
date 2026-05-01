use assert_cmd::Command;
use rex_cli::bundle::Bundle;
use rex_cli::commands::create::{CreateOpts, apply_create};
use rex_cli::project::{PipelineStep, ProjectId, ProjectStore, ProjectYaml, parse_pipeline};
use rex_cli::schedule::{Chunk, Phase, Schedule, ScheduleState, Task};
use tempfile::TempDir;

fn rex_cmd() -> Command {
    Command::cargo_bin("rex").expect("rex binary must be built")
}

/// Seed an active project in `dir` using the embedded pipeline template.
fn setup_with_active_project(dir: &std::path::Path, project_id: &str) {
    let bundle = Bundle::Embedded;
    let bytes = bundle
        .read_file(std::path::Path::new("rex/pipeline.yaml"))
        .unwrap();
    let yaml = String::from_utf8_lossy(&bytes);
    let template = parse_pipeline(&yaml).unwrap();

    let opts = CreateOpts {
        title: "Integration Test Project".to_owned(),
        subtitle: None,
        description: None,
        category: "feature".to_owned(),
        complexity: "medium".to_owned(),
        project_id: ProjectId::parse(project_id).unwrap(),
        selected_optional_steps: vec![],
    };
    apply_create(dir, &template, opts).unwrap();
}

/// Write a hand-crafted active project with mixed completed flags so step tests
/// can verify the first-incomplete selection.
fn setup_with_mixed_steps(dir: &std::path::Path) {
    let project = ProjectYaml {
        project_id: ProjectId::parse("mixed-steps").unwrap(),
        category: "feature".to_owned(),
        title: Some("Mixed Steps".to_owned()),
        subtitle: None,
        description: None,
        complexity: "medium".to_owned(),
        chunks_required: 0,
        chunks_completed: 0,
        tasks_required: 0,
        tasks_completed: 0,
        completed: false,
        steps: vec![
            PipelineStep {
                step: "first".to_owned(),
                required: true,
                skill: None,
                agent: None,
                instructions: None,
                inputs: None,
                outputs: None,
                completed: true,
            },
            PipelineStep {
                step: "second".to_owned(),
                required: true,
                skill: None,
                agent: None,
                instructions: None,
                inputs: None,
                outputs: None,
                completed: false,
            },
            PipelineStep {
                step: "third".to_owned(),
                required: true,
                skill: None,
                agent: None,
                instructions: None,
                inputs: None,
                outputs: None,
                completed: false,
            },
        ],
    };
    ProjectStore::new(dir).write_active(&project).unwrap();
}

/// Write an active project where every step is already completed.
fn setup_with_all_steps_completed(dir: &std::path::Path) {
    let project = ProjectYaml {
        project_id: ProjectId::parse("all-done").unwrap(),
        category: "feature".to_owned(),
        title: Some("All Done".to_owned()),
        subtitle: None,
        description: None,
        complexity: "low".to_owned(),
        chunks_required: 0,
        chunks_completed: 0,
        tasks_required: 0,
        tasks_completed: 0,
        completed: true,
        steps: vec![
            PipelineStep {
                step: "alpha".to_owned(),
                required: true,
                skill: None,
                agent: None,
                instructions: None,
                inputs: None,
                outputs: None,
                completed: true,
            },
            PipelineStep {
                step: "beta".to_owned(),
                required: true,
                skill: None,
                agent: None,
                instructions: None,
                inputs: None,
                outputs: None,
                completed: true,
            },
        ],
    };
    ProjectStore::new(dir).write_active(&project).unwrap();
}

// --- `rex project` ---

#[test]
fn project_full_contains_steps_key() {
    let dir = TempDir::new().unwrap();
    setup_with_active_project(dir.path(), "test-full");

    let output = rex_cmd()
        .arg("project")
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value =
        serde_json::from_slice(&output).expect("stdout must be valid JSON");
    assert!(
        json.get("steps").is_some(),
        "rex project must include a 'steps' key"
    );
}

#[test]
fn project_full_no_active_project_fails() {
    let dir = TempDir::new().unwrap();
    rex_cmd()
        .arg("project")
        .current_dir(dir.path())
        .assert()
        .failure();
}

// --- `rex project meta` ---

#[test]
fn project_meta_excludes_steps_key() {
    let dir = TempDir::new().unwrap();
    setup_with_active_project(dir.path(), "test-meta");

    let output = rex_cmd()
        .args(["project", "meta"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value =
        serde_json::from_slice(&output).expect("stdout must be valid JSON");
    assert!(
        json.get("steps").is_none(),
        "rex project meta must not include a 'steps' key"
    );
}

#[test]
fn project_meta_no_active_project_fails() {
    let dir = TempDir::new().unwrap();
    rex_cmd()
        .args(["project", "meta"])
        .current_dir(dir.path())
        .assert()
        .failure();
}

// --- `rex project step` ---

#[test]
fn project_step_returns_first_incomplete() {
    let dir = TempDir::new().unwrap();
    setup_with_mixed_steps(dir.path());

    let output = rex_cmd()
        .args(["project", "step"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value =
        serde_json::from_slice(&output).expect("stdout must be valid JSON");
    assert_eq!(
        json.get("step").and_then(|v| v.as_str()),
        Some("second"),
        "rex project step must return the first incomplete step"
    );
}

#[test]
fn project_step_all_complete_prints_sentinel() {
    let dir = TempDir::new().unwrap();
    setup_with_all_steps_completed(dir.path());

    let output = rex_cmd()
        .args(["project", "step"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value =
        serde_json::from_slice(&output).expect("stdout must be valid JSON");
    assert_eq!(
        json.get("status").and_then(|v| v.as_str()),
        Some("all-steps-complete"),
        "rex project step must print sentinel when all steps complete"
    );
}

#[test]
fn project_step_no_active_project_fails() {
    let dir = TempDir::new().unwrap();
    rex_cmd()
        .args(["project", "step"])
        .current_dir(dir.path())
        .assert()
        .failure();
}

// --- schedule helpers ---

/// Seed an active project + a schedule file with configurable state.
///
/// The schedule has one phase with two chunks:
/// - `chunk-alpha`: two tasks, both `Pending` by default.
/// - `chunk-beta`: one task, `Pending` by default.
///
/// Pass `done_chunks` to mark the first N chunks as `Done` (all tasks done too).
fn setup_with_schedule(dir: &std::path::Path, project_id: &str, done_chunks: usize) {
    let project = ProjectYaml {
        project_id: ProjectId::parse(project_id).unwrap(),
        category: "feature".to_owned(),
        title: Some("Schedule Test".to_owned()),
        subtitle: None,
        description: None,
        complexity: "medium".to_owned(),
        chunks_required: 2,
        chunks_completed: done_chunks as u32,
        tasks_required: 3,
        tasks_completed: 0,
        completed: false,
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

    let alpha_state = if done_chunks >= 1 {
        ScheduleState::Done
    } else {
        ScheduleState::Pending
    };
    let alpha_task_state = if done_chunks >= 1 {
        ScheduleState::Done
    } else {
        ScheduleState::Pending
    };
    let beta_state = if done_chunks >= 2 {
        ScheduleState::Done
    } else {
        ScheduleState::Pending
    };
    let beta_task_state = if done_chunks >= 2 {
        ScheduleState::Done
    } else {
        ScheduleState::Pending
    };

    let schedule = Schedule {
        project: ProjectId::parse(project_id).unwrap(),
        phases: vec![Phase {
            id: "phase-one".to_owned(),
            description: "First phase".to_owned(),
            blocked_by: vec![],
            state: if done_chunks >= 2 {
                ScheduleState::Done
            } else {
                ScheduleState::Pending
            },
            chunks: vec![
                Chunk {
                    id: "chunk-alpha".to_owned(),
                    description: "Alpha chunk".to_owned(),
                    scenarios: vec!["scenario one".to_owned()],
                    spec_refs: vec![],
                    blocked_by: vec![],
                    state: alpha_state,
                    tasks: vec![
                        Task {
                            id: "task-one".to_owned(),
                            description: "First task".to_owned(),
                            state: alpha_task_state.clone(),
                            skill: None,
                            inputs: None,
                            outputs: None,
                        },
                        Task {
                            id: "task-two".to_owned(),
                            description: "Second task".to_owned(),
                            state: alpha_task_state,
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
                    state: beta_state,
                    tasks: vec![Task {
                        id: "task-three".to_owned(),
                        description: "Third task".to_owned(),
                        state: beta_task_state,
                        skill: None,
                        inputs: None,
                        outputs: None,
                    }],
                },
            ],
        }],
    };
    store.write_schedule(&schedule).unwrap();
}

// --- `rex project chunk-next` ---

#[test]
fn chunk_next_returns_first_pending() {
    let dir = TempDir::new().unwrap();
    setup_with_schedule(dir.path(), "chunk-next-test", 0);

    let output = rex_cmd()
        .args(["project", "chunk-next"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(
        json.get("id").and_then(|v| v.as_str()),
        Some("chunk-alpha"),
        "chunk-next must return first pending chunk"
    );
}

#[test]
fn chunk_next_skips_done_and_returns_next_pending() {
    let dir = TempDir::new().unwrap();
    // chunk-alpha done, chunk-beta still pending
    setup_with_schedule(dir.path(), "chunk-next-skip-test", 1);

    let output = rex_cmd()
        .args(["project", "chunk-next"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(
        json.get("id").and_then(|v| v.as_str()),
        Some("chunk-beta"),
        "chunk-next must skip done chunks and return next pending"
    );
}

#[test]
fn chunk_next_all_done_prints_sentinel() {
    let dir = TempDir::new().unwrap();
    setup_with_schedule(dir.path(), "chunk-next-done-test", 2);

    let output = rex_cmd()
        .args(["project", "chunk-next"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(
        json.get("status").and_then(|v| v.as_str()),
        Some("all-chunks-complete"),
        "chunk-next must print sentinel when all chunks done"
    );
}

// --- `rex project chunk-prior` ---

#[test]
fn chunk_prior_returns_last_done() {
    let dir = TempDir::new().unwrap();
    setup_with_schedule(dir.path(), "chunk-prior-test", 1);

    let output = rex_cmd()
        .args(["project", "chunk-prior"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(
        json.get("id").and_then(|v| v.as_str()),
        Some("chunk-alpha"),
        "chunk-prior must return last done chunk"
    );
}

#[test]
fn chunk_prior_no_done_prints_sentinel() {
    let dir = TempDir::new().unwrap();
    setup_with_schedule(dir.path(), "chunk-prior-none-test", 0);

    let output = rex_cmd()
        .args(["project", "chunk-prior"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(
        json.get("status").and_then(|v| v.as_str()),
        Some("no-prior-chunk"),
        "chunk-prior must print sentinel when no chunk done"
    );
}

// --- `rex project task complete` ---

#[test]
fn task_complete_marks_current_task_done() {
    let dir = TempDir::new().unwrap();
    setup_with_schedule(dir.path(), "task-complete-test", 0);

    let output = rex_cmd()
        .args(["project", "task", "complete"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(
        json.get("id").and_then(|v| v.as_str()),
        Some("task-one"),
        "task complete must return the just-completed task"
    );
    assert_eq!(
        json.get("state").and_then(|v| v.as_str()),
        Some("done"),
        "completed task state must be 'done'"
    );
}

#[test]
fn task_complete_promotes_chunk_when_last_task() {
    let dir = TempDir::new().unwrap();
    setup_with_schedule(dir.path(), "task-promote-test", 0);

    // Complete task-one
    rex_cmd()
        .args(["project", "task", "complete"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Complete task-two (last in chunk-alpha)
    rex_cmd()
        .args(["project", "task", "complete"])
        .current_dir(dir.path())
        .assert()
        .success();

    // chunk-next should now skip chunk-alpha and return chunk-beta
    let output = rex_cmd()
        .args(["project", "chunk-next"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(
        json.get("id").and_then(|v| v.as_str()),
        Some("chunk-beta"),
        "after completing all tasks in chunk-alpha, chunk-next must return chunk-beta"
    );
}

#[test]
fn task_complete_increments_project_counters() {
    let dir = TempDir::new().unwrap();
    setup_with_schedule(dir.path(), "task-counter-test", 0);

    rex_cmd()
        .args(["project", "task", "complete"])
        .current_dir(dir.path())
        .assert()
        .success();

    let store = ProjectStore::new(dir.path());
    let project = store.read_active().unwrap();
    assert_eq!(
        project.tasks_completed, 1,
        "tasks_completed must increment after task complete"
    );

    // Counter invariant: tasks_completed in project.yaml == Done tasks in schedule.json
    let schedule = store.read_schedule().unwrap();
    let done_count = schedule
        .phases
        .iter()
        .flat_map(|p| p.chunks.iter())
        .flat_map(|c| c.tasks.iter())
        .filter(|t| t.state == rex_cli::schedule::ScheduleState::Done)
        .count();
    assert_eq!(
        project.tasks_completed as usize, done_count,
        "tasks_completed in project.yaml must equal Done task count in schedule.json"
    );
}

#[test]
fn task_complete_no_active_task_prints_sentinel() {
    let dir = TempDir::new().unwrap();
    // All chunks done means no active task
    setup_with_schedule(dir.path(), "task-no-active-test", 2);

    let output = rex_cmd()
        .args(["project", "task", "complete"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(
        json.get("status").and_then(|v| v.as_str()),
        Some("no-active-task"),
        "task complete must print sentinel when no pending task exists"
    );
}

// --- `rex project step complete` ---

#[test]
fn step_complete_marks_first_incomplete() {
    let dir = TempDir::new().unwrap();
    setup_with_mixed_steps(dir.path());

    let output = rex_cmd()
        .args(["project", "step", "complete"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(
        json.get("step").and_then(|v| v.as_str()),
        Some("second"),
        "step complete must return the just-completed step"
    );
    assert_eq!(
        json.get("completed").and_then(|v| v.as_bool()),
        Some(true),
        "step must now be completed"
    );
}

#[test]
fn step_complete_sets_project_completed_when_all_required_done() {
    let dir = TempDir::new().unwrap();
    // Set up a project with one required incomplete step
    let project = ProjectYaml {
        project_id: ProjectId::parse("last-step-project").unwrap(),
        category: "feature".to_owned(),
        title: Some("Last Step".to_owned()),
        subtitle: None,
        description: None,
        complexity: "low".to_owned(),
        chunks_required: 0,
        chunks_completed: 0,
        tasks_required: 0,
        tasks_completed: 0,
        completed: false,
        steps: vec![
            PipelineStep {
                step: "first".to_owned(),
                required: true,
                skill: None,
                agent: None,
                instructions: None,
                inputs: None,
                outputs: None,
                completed: true,
            },
            PipelineStep {
                step: "last".to_owned(),
                required: true,
                skill: None,
                agent: None,
                instructions: None,
                inputs: None,
                outputs: None,
                completed: false,
            },
        ],
    };
    ProjectStore::new(dir.path())
        .write_active(&project)
        .unwrap();

    rex_cmd()
        .args(["project", "step", "complete"])
        .current_dir(dir.path())
        .assert()
        .success();

    let store = ProjectStore::new(dir.path());
    let updated = store.read_active().unwrap();
    assert!(
        updated.completed,
        "project.completed must be true when every required step is done"
    );
}

#[test]
fn step_complete_all_complete_prints_sentinel() {
    let dir = TempDir::new().unwrap();
    setup_with_all_steps_completed(dir.path());

    let output = rex_cmd()
        .args(["project", "step", "complete"])
        .current_dir(dir.path())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(
        json.get("status").and_then(|v| v.as_str()),
        Some("all-steps-complete"),
        "step complete must print sentinel when no incomplete step remains"
    );
}
