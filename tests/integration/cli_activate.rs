use assert_cmd::Command;
use tempfile::TempDir;

use rex_cli::bundle::Bundle;
use rex_cli::commands::create::{CreateOpts, apply_create};
use rex_cli::project::{ProjectId, archive_active, parse_pipeline, read_active_project};

fn rex_cmd() -> Command {
    Command::cargo_bin("rex").expect("rex binary must be built")
}

fn setup_with_inactive_project(dir: &std::path::Path, project_id: &str) {
    let bundle = Bundle::Embedded;
    let bytes = bundle
        .read_file(std::path::Path::new("rex/pipeline.yaml"))
        .unwrap();
    let yaml = String::from_utf8_lossy(&bytes);
    let template = parse_pipeline(&yaml).unwrap();

    let opts = CreateOpts {
        title: "Test Project".to_owned(),
        subtitle: None,
        description: None,
        category: "feature".to_owned(),
        complexity: "medium".to_owned(),
        project_id: ProjectId::new(project_id),
        selected_optional_steps: vec![],
    };
    apply_create(dir, &template, opts).unwrap();
    archive_active(dir).unwrap();
}

#[test]
fn activate_swaps_active_project() {
    let dir = TempDir::new().unwrap();
    setup_with_inactive_project(dir.path(), "project-alpha");

    rex_cmd()
        .args(["activate", "project-alpha"])
        .current_dir(dir.path())
        .assert()
        .success();

    let active = read_active_project(dir.path()).unwrap();
    assert_eq!(active.project_id, ProjectId::new("project-alpha"));
}

#[test]
fn activate_nonexistent_fails() {
    let dir = TempDir::new().unwrap();
    rex_cmd()
        .args(["activate", "does-not-exist"])
        .current_dir(dir.path())
        .assert()
        .failure();
}

#[test]
fn activate_archives_current_active() {
    let dir = TempDir::new().unwrap();
    setup_with_inactive_project(dir.path(), "project-alpha");
    setup_with_inactive_project(dir.path(), "project-beta");

    // Activate alpha (beta is in inactive but has no active currently — let's set one up).
    rex_cmd()
        .args(["activate", "project-alpha"])
        .current_dir(dir.path())
        .assert()
        .success();

    // Now activate beta — should archive alpha first.
    rex_cmd()
        .args(["activate", "project-beta"])
        .current_dir(dir.path())
        .assert()
        .success();

    assert!(
        dir.path().join("rex/inactive/project-alpha").exists(),
        "project-alpha must be in inactive after swap"
    );
    let active = read_active_project(dir.path()).unwrap();
    assert_eq!(active.project_id, ProjectId::new("project-beta"));
}
