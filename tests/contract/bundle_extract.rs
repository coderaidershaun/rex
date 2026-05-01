use rex_cli::bundle::{Bundle, sha256_hex};
use std::path::Path;
use tempfile::TempDir;

/// Embedded bundle must be walkable and return non-empty content for known paths.
#[test]
fn embedded_bundle_walks_nonempty() {
    let bundle = Bundle::Embedded;
    let entries = bundle.walk().expect("walk embedded bundle");
    assert!(
        !entries.is_empty(),
        "embedded bundle must contain at least one file"
    );
}

#[test]
fn embedded_bundle_contains_pipeline_yaml() {
    let bundle = Bundle::Embedded;
    let entries = bundle.walk().expect("walk embedded bundle");
    let has_pipeline = entries
        .iter()
        .any(|(p, _)| p == Path::new("rex/pipeline.yaml"));
    assert!(
        has_pipeline,
        "embedded bundle must contain rex/pipeline.yaml"
    );
}

#[test]
fn embedded_bundle_reads_claude_md_tmpl_via_read_file() {
    // The template is intentionally excluded from walk() so it isn't extracted
    // to user CWD. It must still be accessible via read_file() for internal use.
    let bundle = Bundle::Embedded;
    let bytes = bundle
        .read_file(Path::new("templates/CLAUDE.md.tmpl"))
        .expect("read_file must return the template");
    assert!(
        !bytes.is_empty(),
        "templates/CLAUDE.md.tmpl must have non-empty content"
    );
}

#[test]
fn read_file_pipeline_yaml_nonempty() {
    let bundle = Bundle::Embedded;
    let bytes = bundle
        .read_file(Path::new("rex/pipeline.yaml"))
        .expect("read pipeline.yaml");
    assert!(!bytes.is_empty(), "pipeline.yaml must not be empty");
}

#[test]
fn sha256_stable_across_reads() {
    let bundle = Bundle::Embedded;
    let bytes1 = bundle
        .read_file(Path::new("rex/pipeline.yaml"))
        .expect("read 1");
    let bytes2 = bundle
        .read_file(Path::new("rex/pipeline.yaml"))
        .expect("read 2");
    assert_eq!(
        sha256_hex(&bytes1),
        sha256_hex(&bytes2),
        "sha256 must be stable across reads of same content"
    );
}

/// Live-disk bundle (REX_BUNDLE_DIR) reads the actual source files.
#[test]
fn live_disk_bundle_reads_pipeline_yaml() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"));
    let bundle = Bundle::LiveDisk(repo.to_owned());
    let bytes = bundle
        .read_file(Path::new("rex/pipeline.yaml"))
        .expect("read pipeline.yaml from live disk");
    let yaml = String::from_utf8_lossy(&bytes);
    assert!(
        yaml.contains("required:"),
        "live pipeline.yaml must contain 'required:' field"
    );
}

#[test]
fn apply_bundle_writes_files_to_fresh_dir() {
    let dir = TempDir::new().unwrap();
    let bundle = Bundle::Embedded;
    let summary = rex_cli::bundle::apply_bundle(&bundle, dir.path(), false)
        .expect("apply bundle to temp dir");
    assert!(
        summary.written > 0 || summary.noops > 0,
        "fresh init must write or adopt at least one file"
    );
    assert!(
        dir.path().join(".claude").exists(),
        ".claude/ directory must be created"
    );
}

#[test]
fn apply_bundle_twice_is_idempotent() {
    let dir = TempDir::new().unwrap();
    let bundle = Bundle::Embedded;
    rex_cli::bundle::apply_bundle(&bundle, dir.path(), false).unwrap();
    let summary2 = rex_cli::bundle::apply_bundle(&bundle, dir.path(), false).expect("second apply");
    assert_eq!(
        summary2.written + summary2.upgraded + summary2.conflicts,
        0,
        "re-init of unchanged bundle must produce zero changes"
    );
}
