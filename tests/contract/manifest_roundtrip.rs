use std::collections::HashMap;
use tempfile::TempDir;

use rex_cli::bundle::{Manifest, read_manifest, write_manifest};

#[test]
fn roundtrip_empty_manifest() {
    let dir = TempDir::new().unwrap();
    let manifest = Manifest {
        rex_version: "0.4.0".to_owned(),
        files: HashMap::new(),
    };
    write_manifest(dir.path(), &manifest).expect("write manifest");
    let loaded = read_manifest(dir.path())
        .expect("read manifest")
        .expect("manifest must be present after write");
    assert_eq!(loaded.rex_version, "0.4.0");
    assert!(loaded.files.is_empty());
}

#[test]
fn roundtrip_with_entries() {
    let dir = TempDir::new().unwrap();
    let mut files = HashMap::new();
    files.insert(
        ".claude/skills/foo/SKILL.md".to_owned(),
        "abc123".to_owned(),
    );
    files.insert("rex/pipeline.yaml".to_owned(), "def456".to_owned());
    let manifest = Manifest {
        rex_version: "0.4.0".to_owned(),
        files,
    };
    write_manifest(dir.path(), &manifest).expect("write manifest");
    let loaded = read_manifest(dir.path())
        .expect("read manifest")
        .expect("present");
    assert_eq!(loaded.files.len(), 2);
    assert_eq!(
        loaded.files.get(".claude/skills/foo/SKILL.md").unwrap(),
        "abc123"
    );
}

#[test]
fn read_manifest_returns_none_when_absent() {
    let dir = TempDir::new().unwrap();
    let result = read_manifest(dir.path()).expect("no error when manifest absent");
    assert!(result.is_none());
}

#[test]
fn write_manifest_is_atomic_no_partial_on_disk() {
    let dir = TempDir::new().unwrap();
    let manifest = Manifest {
        rex_version: "0.4.0".to_owned(),
        files: HashMap::new(),
    };
    write_manifest(dir.path(), &manifest).unwrap();
    // tmp file must not linger after write
    let tmp = dir.path().join(".claude/.rex-manifest.json.tmp");
    assert!(
        !tmp.exists(),
        "temp file must be cleaned up after atomic write"
    );
}
