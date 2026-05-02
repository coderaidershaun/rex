use assert_cmd::Command;
use rex_cli::bundle::Bundle;
use rex_cli::commands::create::{CreateOpts, apply_create};
use rex_cli::project::{ProjectId, ProjectStore, parse_pipeline};
use std::fs;
use tempfile::TempDir;

fn rex_cmd() -> Command {
    Command::cargo_bin("rex").expect("rex binary must be built")
}

fn make_opts(project_id: &str, is_autopilot: bool) -> CreateOpts {
    CreateOpts {
        title: "Autopilot Test".to_owned(),
        subtitle: None,
        description: None,
        category: "feature".to_owned(),
        complexity: "medium".to_owned(),
        project_id: ProjectId::parse(project_id).unwrap(),
        selected_optional_steps: vec![],
        is_autopilot,
    }
}

fn embedded_template() -> rex_cli::project::PipelineTemplate {
    let bytes = Bundle::Embedded
        .read_file(std::path::Path::new("rex/pipeline.yaml"))
        .unwrap();
    parse_pipeline(&String::from_utf8_lossy(&bytes)).unwrap()
}

#[test]
fn is_autopilot_false_roundtrips_through_yaml_and_meta() {
    let dir = TempDir::new().unwrap();
    let template = embedded_template();
    apply_create(dir.path(), &template, make_opts("paused", false)).unwrap();

    let yaml = fs::read_to_string(dir.path().join("rex/active/project.yaml")).unwrap();
    assert!(
        yaml.contains("is-autopilot: false"),
        "project.yaml must serialize is-autopilot in kebab-case:\n{yaml}"
    );

    let project = ProjectStore::new(dir.path()).read_active().unwrap();
    assert!(!project.is_autopilot);

    let output = rex_cmd()
        .args(["project", "meta"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "rex project meta failed");
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json.get("is-autopilot").and_then(|v| v.as_bool()), Some(false));
}

#[test]
fn is_autopilot_true_roundtrips_through_yaml_and_meta() {
    let dir = TempDir::new().unwrap();
    let template = embedded_template();
    apply_create(dir.path(), &template, make_opts("auto", true)).unwrap();

    let yaml = fs::read_to_string(dir.path().join("rex/active/project.yaml")).unwrap();
    assert!(yaml.contains("is-autopilot: true"), "got:\n{yaml}");

    let project = ProjectStore::new(dir.path()).read_active().unwrap();
    assert!(project.is_autopilot);

    let output = rex_cmd()
        .args(["project", "meta"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json.get("is-autopilot").and_then(|v| v.as_bool()), Some(true));
}

#[test]
fn missing_is_autopilot_loads_as_false_for_legacy_files() {
    let dir = TempDir::new().unwrap();
    let active_dir = dir.path().join("rex/active");
    fs::create_dir_all(&active_dir).unwrap();
    let legacy = r#"project-id: legacy
category: feature
title: Legacy
subtitle: null
description: null
complexity: medium
chunks-required: 0
chunks-completed: 0
tasks-required: 0
tasks-completed: 0
completed: false
steps: []
"#;
    fs::write(active_dir.join("project.yaml"), legacy).unwrap();

    let project = ProjectStore::new(dir.path()).read_active().unwrap();
    assert!(
        !project.is_autopilot,
        "legacy project.yaml without is-autopilot must default to false"
    );
}
