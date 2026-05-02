//! Integration tests for `apply_create` row-prompt feature.
//!
//! Covers: research-api row write, resources row write, negative (no optional steps).

use std::fs;

use rex_cli::bundle::Bundle;
use rex_cli::commands::create::{CreateOpts, ResearchApiRow, ResourceRow, apply_create};
use rex_cli::project::{ProjectId, ProjectStore, parse_pipeline};
use tempfile::TempDir;

fn embedded_template() -> rex_cli::project::PipelineTemplate {
    let bytes = Bundle::Embedded
        .read_file(std::path::Path::new("rex/pipeline.yaml"))
        .unwrap();
    parse_pipeline(&String::from_utf8_lossy(&bytes)).unwrap()
}

#[test]
fn create_with_research_api_writes_apis_md_and_sets_inputs() {
    let dir = TempDir::new().unwrap();
    let template = embedded_template();

    let opts = CreateOpts {
        title: "Row Test".to_owned(),
        subtitle: None,
        description: None,
        category: "feature".to_owned(),
        complexity: "medium".to_owned(),
        project_id: ProjectId::parse("row-test-api").unwrap(),
        selected_optional_steps: vec!["research-api".to_owned()],
        research_apis: vec![ResearchApiRow {
            name: "Binance Spot".to_owned(),
            url: "https://binance-docs.github.io/apidocs/spot/en/".to_owned(),
        }],
        resources: Vec::new(),
        is_autopilot: false,
    };

    apply_create(dir.path(), &template, opts).unwrap();

    let md_path = dir.path().join("rex/active/research/apis.md");
    assert!(md_path.exists(), "apis.md must be created under rex/active/research/");

    let body = fs::read_to_string(&md_path).unwrap();
    assert!(body.contains("Binance Spot"), "apis.md must contain the API name");
    assert!(
        body.contains("https://binance-docs.github.io"),
        "apis.md must contain the URL"
    );

    let project = ProjectStore::new(dir.path()).read_active().unwrap();
    let step = project
        .steps
        .iter()
        .find(|s| s.step == "research-api")
        .expect("research-api step must be present in project.yaml");
    assert_eq!(
        step.inputs.as_deref(),
        Some("rex/active/research/*"),
        "research-api step inputs must point at rex/active/research/*"
    );
}

#[test]
fn create_with_resources_writes_urls_md_and_sets_inputs() {
    let dir = TempDir::new().unwrap();
    let template = embedded_template();

    let opts = CreateOpts {
        title: "Row Test".to_owned(),
        subtitle: None,
        description: None,
        category: "feature".to_owned(),
        complexity: "medium".to_owned(),
        project_id: ProjectId::parse("row-test-res").unwrap(),
        selected_optional_steps: vec!["resources".to_owned()],
        research_apis: Vec::new(),
        resources: vec![ResourceRow {
            label: "Design spec".to_owned(),
            url: "https://example.com/design.pdf".to_owned(),
        }],
        is_autopilot: false,
    };

    apply_create(dir.path(), &template, opts).unwrap();

    let md_path = dir.path().join("rex/active/resources/urls.md");
    assert!(md_path.exists(), "urls.md must be created under rex/active/resources/");

    let body = fs::read_to_string(&md_path).unwrap();
    assert!(body.contains("Design spec"), "urls.md must contain the resource label");
    assert!(
        body.contains("https://example.com/design.pdf"),
        "urls.md must contain the URL"
    );

    let project = ProjectStore::new(dir.path()).read_active().unwrap();
    let step = project
        .steps
        .iter()
        .find(|s| s.step == "resources")
        .expect("resources step must be present in project.yaml");
    assert_eq!(
        step.inputs.as_deref(),
        Some("rex/active/resources/*"),
        "resources step inputs must point at rex/active/resources/*"
    );
}

#[test]
fn create_without_optional_steps_writes_no_row_subfolders() {
    let dir = TempDir::new().unwrap();
    let template = embedded_template();

    let opts = CreateOpts {
        title: "No Rows".to_owned(),
        subtitle: None,
        description: None,
        category: "feature".to_owned(),
        complexity: "low".to_owned(),
        project_id: ProjectId::parse("no-rows").unwrap(),
        selected_optional_steps: vec![],
        research_apis: Vec::new(),
        resources: Vec::new(),
        is_autopilot: false,
    };

    apply_create(dir.path(), &template, opts).unwrap();

    assert!(
        !dir.path().join("rex/active/research").exists(),
        "research/ subfolder must not exist when no research-api rows were supplied"
    );
    assert!(
        !dir.path().join("rex/active/resources").exists(),
        "resources/ subfolder must not exist when no resource rows were supplied"
    );
}
