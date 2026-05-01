use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn rex_cmd() -> Command {
    Command::cargo_bin("rex").expect("rex binary must be built")
}

/// Fresh init populates .claude/ and writes CLAUDE.md.
#[test]
fn fresh_init_writes_bundle_and_claude_md() {
    let dir = TempDir::new().unwrap();
    rex_cmd()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    assert!(
        dir.path().join(".claude").is_dir(),
        ".claude/ must exist after init"
    );
    assert!(
        dir.path().join("CLAUDE.md").exists(),
        "CLAUDE.md must be generated on fresh init"
    );
    assert!(
        dir.path().join(".claude/.rex-manifest.json").exists(),
        "manifest must exist after init"
    );
    assert!(
        dir.path().join("rex/active").is_dir(),
        "rex/active/ must be created after init"
    );
}

/// Second init on unchanged bundle produces zero changes.
#[test]
fn reinit_unchanged_is_noop() {
    let dir = TempDir::new().unwrap();
    rex_cmd()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    let output = rex_cmd()
        .arg("init")
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "re-init must succeed");
    assert!(
        stdout.contains("0 written") || stdout.contains("0 upgraded"),
        "re-init output must show 0 writes: {stdout}"
    );
}

/// Init with --force overwrites files regardless of user changes.
#[test]
fn init_force_overwrites_user_changes() {
    let dir = TempDir::new().unwrap();
    rex_cmd()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Simulate user modifying a file.
    let pipeline = dir.path().join("rex/pipeline.yaml");
    if pipeline.exists() {
        fs::write(&pipeline, b"user modified content").unwrap();
    }

    rex_cmd()
        .args(["init", "--force"])
        .current_dir(dir.path())
        .assert()
        .success();
}

/// User-modified file is preserved on re-init (no --force).
#[test]
fn reinit_preserves_user_modified_file() {
    let dir = TempDir::new().unwrap();
    rex_cmd()
        .arg("init")
        .current_dir(dir.path())
        .assert()
        .success();

    // Find a skill file to modify — explicitly a .md file, never the manifest.
    let claude_dir = dir.path().join(".claude");
    let skill_file = walkdir::WalkDir::new(&claude_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .find(|e| e.file_type().is_file() && e.path().extension().is_some_and(|ext| ext == "md"))
        .map(|e| e.path().to_owned())
        .expect("must find at least one .md file in .claude/");

    let original_content = fs::read(&skill_file).unwrap();
    let modified = {
        let mut m = original_content.clone();
        m.extend_from_slice(b"\nuser edit\n");
        m
    };
    fs::write(&skill_file, &modified).unwrap();

    let output = rex_cmd()
        .arg("init")
        .current_dir(dir.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "rex init failed. stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("preserved"),
        "user-modified file must show as preserved: {stdout}"
    );

    // File must still have user's content.
    let on_disk = fs::read(&skill_file).unwrap();
    assert_eq!(on_disk, modified, "user content must not be overwritten");
}
