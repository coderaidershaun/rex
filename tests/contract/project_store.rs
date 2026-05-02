use std::fs;

use tempfile::TempDir;

use rex_cli::bundle::Bundle;
use rex_cli::commands::create::apply_create;
use rex_cli::error::RexError;
use rex_cli::project::{
    PipelineStep, PipelineTemplate, ProjectId, ProjectMeta, ProjectStore, ProjectYaml,
    current_incomplete_step, parse_pipeline,
};

fn make_template() -> PipelineTemplate {
    PipelineTemplate {
        project_id: ProjectId::parse("template").unwrap(),
        category: "feature".to_owned(),
        title: None,
        subtitle: None,
        description: None,
        complexity: "medium".to_owned(),
        chunks_required: 0,
        chunks_completed: 0,
        tasks_required: 0,
        tasks_completed: 0,
        completed: false,
        is_autopilot: false,
        steps: vec![
            PipelineStep {
                step: "discovery".to_owned(),
                required: true,
                skill: None,
                agent: None,
                instructions: None,
                inputs: None,
                outputs: None,
                completed: false,
            },
            PipelineStep {
                step: "resources".to_owned(),
                required: false,
                skill: None,
                agent: None,
                instructions: None,
                inputs: None,
                outputs: None,
                completed: false,
            },
        ],
    }
}

fn make_create_opts(project_id: &str) -> rex_cli::commands::create::CreateOpts {
    rex_cli::commands::create::CreateOpts {
        title: "Test Project".to_owned(),
        subtitle: None,
        description: None,
        category: "feature".to_owned(),
        complexity: "medium".to_owned(),
        project_id: ProjectId::parse(project_id).unwrap(),
        selected_optional_steps: vec![],
        research_apis: vec![],
        resources: vec![],
        is_autopilot: false,
    }
}

/// Build a default `ProjectYaml` with the given id, steps, and `Test` title.
fn fixture_project(id: &str, steps: Vec<PipelineStep>) -> ProjectYaml {
    ProjectYaml {
        project_id: ProjectId::parse(id).unwrap(),
        category: "feature".to_owned(),
        title: Some("Test".to_owned()),
        subtitle: None,
        description: None,
        complexity: "medium".to_owned(),
        chunks_required: 0,
        chunks_completed: 0,
        tasks_required: 0,
        tasks_completed: 0,
        completed: false,
        is_autopilot: false,
        steps,
    }
}

/// Build a required `PipelineStep` with the given name and completion flag.
fn pipeline_step(name: &str, completed: bool) -> PipelineStep {
    PipelineStep {
        step: name.to_owned(),
        required: true,
        skill: None,
        agent: None,
        instructions: None,
        inputs: None,
        outputs: None,
        completed,
    }
}

#[test]
fn write_then_read_roundtrip() {
    let dir = TempDir::new().unwrap();
    let store = ProjectStore::new(dir.path());
    let project = fixture_project("test-project", vec![]);
    store.write_active(&project).unwrap();
    let loaded = store.read_active().unwrap();
    assert_eq!(loaded.project_id, ProjectId::parse("test-project").unwrap());
    assert_eq!(loaded.title, Some("Test".to_owned()));
}

#[test]
fn has_active_project_false_when_empty() {
    let dir = TempDir::new().unwrap();
    let store = ProjectStore::new(dir.path());
    assert!(!store.has_active());
}

#[test]
fn has_active_project_true_after_write() {
    let dir = TempDir::new().unwrap();
    let tmpl = make_template();
    apply_create(dir.path(), &tmpl, make_create_opts("my-project")).unwrap();
    let store = ProjectStore::new(dir.path());
    assert!(store.has_active());
}

#[test]
fn archive_active_moves_to_inactive() {
    let dir = TempDir::new().unwrap();
    let tmpl = make_template();
    apply_create(dir.path(), &tmpl, make_create_opts("proj-one")).unwrap();
    let store = ProjectStore::new(dir.path());
    let archived_id = store.archive_active().unwrap();
    assert_eq!(archived_id, ProjectId::parse("proj-one").unwrap());
    assert!(!store.has_active());
    assert!(dir.path().join("rex/inactive/proj-one").exists());
}

#[test]
fn archive_active_collision_returns_error() {
    let dir = TempDir::new().unwrap();
    let tmpl = make_template();
    apply_create(dir.path(), &tmpl, make_create_opts("proj-one")).unwrap();
    let store = ProjectStore::new(dir.path());
    store.archive_active().unwrap();

    // Re-create same project-id so it's active again with same id.
    apply_create(dir.path(), &tmpl, make_create_opts("proj-two")).unwrap();
    // Manually rename to simulate collision.
    fs::rename(
        dir.path().join("rex/active"),
        dir.path().join("rex/inactive/proj-one-dup"),
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("rex/active")).unwrap();
    store
        .write_active(&fixture_project("proj-one", vec![]))
        .unwrap();

    let err = store.archive_active().unwrap_err();
    assert!(
        matches!(err, RexError::SlugCollision { .. }),
        "expected SlugCollision, got: {err}"
    );
}

#[test]
fn swap_active_roundtrip() {
    let dir = TempDir::new().unwrap();
    let tmpl = make_template();
    let store = ProjectStore::new(dir.path());

    apply_create(dir.path(), &tmpl, make_create_opts("proj-a")).unwrap();
    store.archive_active().unwrap();

    apply_create(dir.path(), &tmpl, make_create_opts("proj-b")).unwrap();

    store.swap_active("proj-a").unwrap();
    let active = store.read_active().unwrap();
    assert_eq!(active.project_id, ProjectId::parse("proj-a").unwrap());
    assert!(dir.path().join("rex/inactive/proj-b").exists());
}

#[test]
fn swap_active_missing_id_errors() {
    let dir = TempDir::new().unwrap();
    let store = ProjectStore::new(dir.path());
    let err = store.swap_active("nonexistent").unwrap_err();
    assert!(
        matches!(err, RexError::ProjectNotFound { .. }),
        "expected ProjectNotFound, got: {err}"
    );
}

#[test]
fn list_inactive_empty_when_none() {
    let dir = TempDir::new().unwrap();
    let store = ProjectStore::new(dir.path());
    let ids = store.list_inactive().unwrap();
    assert!(ids.is_empty());
}

#[test]
fn list_inactive_after_archive() {
    let dir = TempDir::new().unwrap();
    let tmpl = make_template();
    let store = ProjectStore::new(dir.path());
    apply_create(dir.path(), &tmpl, make_create_opts("alpha")).unwrap();
    store.archive_active().unwrap();
    apply_create(dir.path(), &tmpl, make_create_opts("beta")).unwrap();
    store.archive_active().unwrap();
    let ids = store.list_inactive().unwrap();
    assert_eq!(
        ids,
        vec![
            ProjectId::parse("alpha").unwrap(),
            ProjectId::parse("beta").unwrap()
        ]
    );
}

#[test]
fn parse_pipeline_reads_required_steps() {
    let bundle = Bundle::Embedded;
    let bytes = bundle
        .read_file(std::path::Path::new("rex/pipeline.yaml"))
        .unwrap();
    let yaml = String::from_utf8_lossy(&bytes);
    let tmpl = parse_pipeline(&yaml).unwrap();
    let required: Vec<_> = tmpl.steps.iter().filter(|s| s.required).collect();
    assert!(
        !required.is_empty(),
        "pipeline must have at least one required step"
    );
}

#[test]
fn create_opts_slug_collision_errors() {
    let dir = TempDir::new().unwrap();
    let tmpl = make_template();
    let store = ProjectStore::new(dir.path());
    apply_create(dir.path(), &tmpl, make_create_opts("col-proj")).unwrap();
    store.archive_active().unwrap();
    let err = apply_create(dir.path(), &tmpl, make_create_opts("col-proj")).unwrap_err();
    assert!(
        matches!(err, RexError::SlugCollision { .. }),
        "expected SlugCollision for duplicate project-id"
    );
}

// --- ProjectMeta ---

fn make_project_yaml_with_steps() -> ProjectYaml {
    fixture_project("meta-test", vec![pipeline_step("one", false)])
}

#[test]
fn project_meta_json_excludes_steps_key() {
    let project = make_project_yaml_with_steps();
    let meta = ProjectMeta::from(&project);
    let json = serde_json::to_string(&meta).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(
        value.get("steps").is_none(),
        "ProjectMeta JSON must not contain a 'steps' key"
    );
}

#[test]
fn project_meta_json_includes_project_id() {
    let project = make_project_yaml_with_steps();
    let meta = ProjectMeta::from(&project);
    let json = serde_json::to_string(&meta).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(
        value.get("project-id").and_then(|v| v.as_str()),
        Some("meta-test")
    );
}

// --- current_incomplete_step ---

#[test]
fn current_incomplete_step_returns_first_incomplete() {
    let project = fixture_project(
        "x",
        vec![
            pipeline_step("done", true),
            pipeline_step("pending", false),
            pipeline_step("also-pending", false),
        ],
    );
    let step = current_incomplete_step(&project).unwrap();
    assert_eq!(step.step, "pending");
}

#[test]
fn current_incomplete_step_none_when_all_complete() {
    let project = fixture_project(
        "x",
        vec![pipeline_step("a", true), pipeline_step("b", true)],
    );
    assert!(current_incomplete_step(&project).is_none());
}

#[test]
fn current_incomplete_step_none_when_steps_empty() {
    let project = fixture_project("x", vec![]);
    assert!(current_incomplete_step(&project).is_none());
}
