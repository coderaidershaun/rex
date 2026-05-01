use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

use crate::error::RexError;

use super::{
    Bundle,
    manifest::{MANIFEST_PATH, Manifest},
};

/// How `apply` resolves files that exist on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundleMode {
    /// Run the three-way merge: preserve user changes, write upgrades, flag conflicts.
    Merge,
    /// Overwrite every bundle file regardless of user modifications.
    Force,
}

/// What the three-way merge logic should do with one file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MergeAction {
    /// Write the bundle version and record its hash.
    Write,
    /// Leave disk untouched. Record the existing hash so future runs detect drift.
    Adopt,
    /// User modified the file; do not touch it.
    PreserveUser,
    /// Both disk and bundle diverge from the manifest. Write a `.rex-new` sibling.
    WriteNew,
    /// File matches the manifest on disk and in the bundle. Nothing to do.
    Noop,
}

/// Per-action counters from a single `apply` run.
#[derive(Debug, Default)]
pub struct InitSummary {
    /// Files newly written on a fresh init.
    pub written: u32,
    /// Files upgraded from a prior bundle version.
    pub upgraded: u32,
    /// Files left untouched because the user had modified them.
    pub preserved: u32,
    /// Files where bundle and disk diverged; bundle written to `<path>.rex-new`.
    pub conflicts: u32,
    /// Files whose disk + bundle + manifest hashes all matched.
    pub noops: u32,
}

/// Run the three-way merge for all bundle files in `cwd`, writing/skipping per the merge table.
///
/// In [`BundleMode::Force`] every file is overwritten regardless of disk state.
///
/// # Errors
/// - [`RexError::Io`] reading or writing files under `cwd`
/// - [`RexError::JsonParse`] / [`RexError::JsonSerialize`] for the manifest
/// - [`RexError::BundleFileNotFound`] if the bundle is missing an expected file
pub fn apply(bundle: &Bundle, cwd: &Path, mode: BundleMode) -> Result<InitSummary, RexError> {
    let existing_manifest = Manifest::load(cwd)?;
    let mut new_files: HashMap<String, String> = HashMap::new();
    let mut summary = InitSummary::default();

    let entries = bundle.walk()?;

    for (rel, bundle_contents) in &entries {
        let rel_str = rel.to_string_lossy().to_string();
        if rel_str == MANIFEST_PATH {
            continue;
        }

        let bundle_hash = sha256_hex(bundle_contents);
        let disk_path = cwd.join(rel);

        let disk_hash = if disk_path.exists() {
            let disk_contents = fs::read(&disk_path).map_err(|source| RexError::Io {
                path: disk_path.clone(),
                source,
            })?;
            Some(sha256_hex(&disk_contents))
        } else {
            None
        };

        let manifest_hash = existing_manifest
            .as_ref()
            .and_then(|m| m.files.get(&rel_str))
            .map(String::as_str);

        let action = match mode {
            BundleMode::Force => MergeAction::Write,
            BundleMode::Merge => merge_action(manifest_hash, disk_hash.as_deref(), &bundle_hash),
        };

        match action {
            MergeAction::Write => {
                write_bundle_file(&disk_path, bundle_contents)?;
                new_files.insert(rel_str, bundle_hash);
                if existing_manifest.is_some() {
                    summary.upgraded += 1;
                } else {
                    summary.written += 1;
                }
            }
            MergeAction::Adopt => {
                new_files.insert(rel_str, bundle_hash);
                summary.noops += 1;
            }
            MergeAction::PreserveUser => {
                if let Some(h) = manifest_hash {
                    new_files.insert(rel_str, h.to_owned());
                } else if let Some(dh) = disk_hash {
                    new_files.insert(rel_str, dh);
                }
                summary.preserved += 1;
            }
            MergeAction::WriteNew => {
                let sibling = rex_new_sibling(&disk_path);
                write_bundle_file(&sibling, bundle_contents)?;
                if let Some(h) = manifest_hash {
                    new_files.insert(rel_str, h.to_owned());
                }
                summary.conflicts += 1;
            }
            MergeAction::Noop => {
                new_files.insert(rel_str, bundle_hash);
                summary.noops += 1;
            }
        }
    }

    let rex_version = env!("CARGO_PKG_VERSION").to_owned();
    let manifest = Manifest {
        rex_version,
        files: new_files,
    };
    manifest.save(cwd)?;

    Ok(summary)
}

/// Hex-encoded SHA-256 of `data`.
pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Compute merge action.
///
/// # Parameters
/// - `manifest_hash`: stored hash from prior init run, or `None` on first init
/// - `disk_hash`: current hash of the on-disk file, or `None` if file absent
/// - `bundle_hash`: hash of the bundle's version of the file
///
/// First-init logic (manifest_hash == None):
///   - missing on disk → write
///   - same bytes as bundle → adopt (record bundle hash, don't write)
///   - differs from bundle → preserve user (treat existing file as user-modified)
///
/// Subsequent-run logic (manifest_hash == Some):
///   - disk == manifest, bundle == manifest → noop
///   - disk == manifest, bundle != manifest → upgrade (write bundle)
///   - disk != manifest, bundle == manifest → preserve user
///   - disk != manifest, bundle != manifest → conflict (write .rex-new sibling)
fn merge_action(
    manifest_hash: Option<&str>,
    disk_hash: Option<&str>,
    bundle_hash: &str,
) -> MergeAction {
    match (manifest_hash, disk_hash) {
        (None, None) => MergeAction::Write,
        (None, Some(dh)) => {
            if dh == bundle_hash {
                MergeAction::Adopt
            } else {
                MergeAction::PreserveUser
            }
        }
        (Some(_), None) => MergeAction::PreserveUser,
        (Some(mh), Some(dh)) => {
            let disk_matches_manifest = dh == mh;
            let bundle_matches_manifest = bundle_hash == mh;

            match (disk_matches_manifest, bundle_matches_manifest) {
                (true, true) => MergeAction::Noop,
                (true, false) => MergeAction::Write,
                (false, true) => MergeAction::PreserveUser,
                (false, false) => MergeAction::WriteNew,
            }
        }
    }
}

fn write_bundle_file(path: &Path, contents: &[u8]) -> Result<(), RexError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| RexError::Io {
            path: parent.to_owned(),
            source,
        })?;
    }
    fs::write(path, contents).map_err(|source| RexError::Io {
        path: path.to_owned(),
        source,
    })
}

/// Conflict-sibling path for a file whose user copy diverges from the bundle.
///
/// Always appends `.rex-new`, never overwrites the original.
fn rex_new_sibling(path: &Path) -> PathBuf {
    let mut sibling = path.as_os_str().to_owned();
    sibling.push(".rex-new");
    PathBuf::from(sibling)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_hex_produces_known_digest() {
        let result = sha256_hex(b"");
        assert_eq!(
            result,
            "e3b0c44298fc1c149afbf4c8996fb924\
             27ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn first_init_absent_file_writes() {
        assert_eq!(merge_action(None, None, "abc"), MergeAction::Write);
    }

    #[test]
    fn first_init_matching_file_adopts() {
        assert_eq!(merge_action(None, Some("abc"), "abc"), MergeAction::Adopt);
    }

    #[test]
    fn first_init_differing_file_preserves() {
        assert_eq!(
            merge_action(None, Some("disk_hash"), "bundle_hash"),
            MergeAction::PreserveUser
        );
    }

    #[test]
    fn subsequent_all_match_is_noop() {
        assert_eq!(
            merge_action(Some("abc"), Some("abc"), "abc"),
            MergeAction::Noop
        );
    }

    #[test]
    fn subsequent_bundle_changed_upgrades() {
        assert_eq!(
            merge_action(Some("old"), Some("old"), "new"),
            MergeAction::Write
        );
    }

    #[test]
    fn subsequent_user_changed_preserves() {
        assert_eq!(
            merge_action(Some("old"), Some("user"), "old"),
            MergeAction::PreserveUser
        );
    }

    #[test]
    fn subsequent_both_changed_conflicts() {
        assert_eq!(
            merge_action(Some("old"), Some("user"), "new"),
            MergeAction::WriteNew
        );
    }

    #[test]
    fn subsequent_disk_deleted_preserves() {
        assert_eq!(
            merge_action(Some("old"), None, "old"),
            MergeAction::PreserveUser
        );
    }

    #[test]
    fn rex_new_sibling_with_extension() {
        let p = Path::new("/tmp/foo.md");
        assert_eq!(rex_new_sibling(p), Path::new("/tmp/foo.md.rex-new"));
    }

    #[test]
    fn rex_new_sibling_without_extension() {
        let p = Path::new("/tmp/foo");
        assert_eq!(rex_new_sibling(p), Path::new("/tmp/foo.rex-new"));
    }
}
