use std::fs;

use tempfile::TempDir;

use rex_cli::bundle::Bundle;
use rex_cli::commands::create::apply_create;
use rex_cli::error::RexError;
use rex_cli::project::{
    PipelineStep, PipelineTemplate, ProjectYaml, archive_active, has_active_project, list_inactive,
    parse_pipeline, read_active_project, swap_active, write_active_project,
};

fn make_template() -> PipelineTemplate {
    PipelineTemplate {
        project_id: "template".to_owned(),
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
        project_id: project_id.to_owned(),
        selected_optional_steps: vec![],
    }
}

#[test]
fn write_then_read_roundtrip() {
    let dir = TempDir::new().unwrap();
    let project = ProjectYaml {
        project_id: "test-project".to_owned(),
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
        steps: vec![],
    };
    write_active_project(dir.path(), &project).unwrap();
    let loaded = read_active_project(dir.path()).unwrap();
    assert_eq!(loaded.project_id, "test-project");
    assert_eq!(loaded.title, Some("Test".to_owned()));
}

#[test]
fn has_active_project_false_when_empty() {
    let dir = TempDir::new().unwrap();
    assert!(!has_active_project(dir.path()));
}

#[test]
fn has_active_project_true_after_write() {
    let dir = TempDir::new().unwrap();
    let tmpl = make_template();
    apply_create(dir.path(), &tmpl, make_create_opts("my-project")).unwrap();
    assert!(has_active_project(dir.path()));
}

#[test]
fn archive_active_moves_to_inactive() {
    let dir = TempDir::new().unwrap();
    let tmpl = make_template();
    apply_create(dir.path(), &tmpl, make_create_opts("proj-one")).unwrap();
    let archived_id = archive_active(dir.path()).unwrap();
    assert_eq!(archived_id, "proj-one");
    assert!(!has_active_project(dir.path()));
    assert!(dir.path().join("rex/inactive/proj-one").exists());
}

#[test]
fn archive_active_collision_returns_error() {
    let dir = TempDir::new().unwrap();
    let tmpl = make_template();
    apply_create(dir.path(), &tmpl, make_create_opts("proj-one")).unwrap();
    archive_active(dir.path()).unwrap();

    // Re-create same project-id so it's active again with same id.
    apply_create(dir.path(), &tmpl, make_create_opts("proj-two")).unwrap();
    // Manually rename to simulate collision.
    fs::rename(
        dir.path().join("rex/active"),
        dir.path().join("rex/inactive/proj-one-dup"),
    )
    .unwrap();
    fs::create_dir_all(dir.path().join("rex/active")).unwrap();
    write_active_project(
        dir.path(),
        &ProjectYaml {
            project_id: "proj-one".to_owned(),
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
            steps: vec![],
        },
    )
    .unwrap();

    let err = archive_active(dir.path()).unwrap_err();
    assert!(
        matches!(err, RexError::SlugCollision { .. }),
        "expected SlugCollision, got: {err}"
    );
}

#[test]
fn swap_active_roundtrip() {
    let dir = TempDir::new().unwrap();
    let tmpl = make_template();

    apply_create(dir.path(), &tmpl, make_create_opts("proj-a")).unwrap();
    archive_active(dir.path()).unwrap();

    apply_create(dir.path(), &tmpl, make_create_opts("proj-b")).unwrap();

    // Activate proj-a (archives proj-b first)
    swap_active(dir.path(), "proj-a").unwrap();
    let active = read_active_project(dir.path()).unwrap();
    assert_eq!(active.project_id, "proj-a");
    assert!(dir.path().join("rex/inactive/proj-b").exists());
}

#[test]
fn swap_active_missing_id_errors() {
    let dir = TempDir::new().unwrap();
    let err = swap_active(dir.path(), "nonexistent").unwrap_err();
    assert!(
        matches!(err, RexError::ProjectNotFound { .. }),
        "expected ProjectNotFound, got: {err}"
    );
}

#[test]
fn list_inactive_empty_when_none() {
    let dir = TempDir::new().unwrap();
    let ids = list_inactive(dir.path()).unwrap();
    assert!(ids.is_empty());
}

#[test]
fn list_inactive_after_archive() {
    let dir = TempDir::new().unwrap();
    let tmpl = make_template();
    apply_create(dir.path(), &tmpl, make_create_opts("alpha")).unwrap();
    archive_active(dir.path()).unwrap();
    apply_create(dir.path(), &tmpl, make_create_opts("beta")).unwrap();
    archive_active(dir.path()).unwrap();
    let ids = list_inactive(dir.path()).unwrap();
    assert_eq!(ids, vec!["alpha", "beta"]);
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
    apply_create(dir.path(), &tmpl, make_create_opts("col-proj")).unwrap();
    archive_active(dir.path()).unwrap();
    let err = apply_create(dir.path(), &tmpl, make_create_opts("col-proj")).unwrap_err();
    assert!(
        matches!(err, RexError::SlugCollision { .. }),
        "expected SlugCollision for duplicate project-id"
    );
}
