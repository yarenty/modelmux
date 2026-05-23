//!
//! One-shot, idempotent macOS configuration migration.
//!
//! Earlier ModelMux versions stored configuration under
//! `~/Library/Application Support/com.SkyCorp.modelmux/` on macOS. From 1.1.0
//! onwards configuration lives under `~/.config/modelmux/` (XDG-style),
//! matching Linux. This module handles the automatic relocation so existing
//! users never have to think about it.
//!
//! Design goals:
//! - **Idempotent**: safe to call on every startup; becomes a no-op once
//!   `~/.config/modelmux/config.toml` exists.
//! - **Non-destructive**: never overwrites a file that already exists in the
//!   new location. If the user already has a file there, the legacy copy is
//!   left untouched and a hint is printed.
//! - **Path-aware**: rewrites absolute references to the legacy directory
//!   inside `config.toml` so users with hardcoded
//!   `service_account_file = "/Users/.../Library/Application Support/..."`
//!   entries don't end up with a broken config after migration.
//! - **Platform-gated**: only compiled on macOS; a no-op stub exists for
//!   other platforms so the call site stays clean.
//!
//! Authors:
//!   Jaro <yarenty@gmail.com>
//!
//! Copyright (c) 2026 SkyCorp

/* --- uses ------------------------------------------------------------------------------------ */

#[cfg(target_os = "macos")]
use crate::config::paths;
#[cfg(target_os = "macos")]
use crate::error::{ProxyError, Result};
#[cfg(target_os = "macos")]
use std::path::{Path, PathBuf};

/* --- types ----------------------------------------------------------------------------------- */

/// Outcome of a migration run. Returned from the inner helper so tests can
/// assert on it; the public entry-point just emits a stderr message.
#[cfg(target_os = "macos")]
#[derive(Debug, Default, PartialEq, Eq)]
pub struct MigrationOutcome {
    /// Files successfully moved from the legacy dir into the new dir.
    pub moved: Vec<PathBuf>,
    /// Files that already existed in the new dir and were therefore not
    /// touched (legacy copy is left in place for the user to inspect).
    pub skipped_existing: Vec<PathBuf>,
    /// The legacy directory that was migrated from, if any.
    pub legacy_dir: Option<PathBuf>,
    /// `true` if `config.toml` was rewritten to update absolute paths
    /// referencing the legacy directory.
    pub rewrote_paths: bool,
}

/* --- public API ------------------------------------------------------------------------------ */

/// Migrate configuration from legacy macOS locations to `~/.config/modelmux/`.
///
/// Safe to call on every startup. Returns `Ok(outcome)` with details about
/// what was migrated (if anything). The function is idempotent — once
/// `~/.config/modelmux/config.toml` exists it short-circuits and returns an
/// empty outcome.
///
/// Errors are reported back to the caller but the caller is expected to treat
/// them as non-fatal (the loader has its own legacy fallback as a safety net).
#[cfg(target_os = "macos")]
pub fn migrate_legacy_macos_config() -> Result<MigrationOutcome> {
    let new_dir = paths::user_config_dir()?;
    let legacy_dirs = paths::legacy_macos_user_config_dirs();
    let outcome = migrate_inner(&legacy_dirs, &new_dir)?;

    if !outcome.moved.is_empty() {
        emit_success_message(&outcome, &new_dir);
    } else if let Some(legacy) = outcome
        .skipped_existing
        .first()
        .and_then(|p| p.parent())
        .map(Path::to_path_buf)
        .or_else(|| outcome.legacy_dir.clone())
    {
        // We found a legacy dir but didn't move anything because the new dir
        // already has files. Tell the user once so they can clean up by hand.
        if !outcome.skipped_existing.is_empty() {
            emit_skip_hint(&legacy, &new_dir, &outcome.skipped_existing);
        }
    }

    Ok(outcome)
}

/// No-op stub for non-macOS targets, so the call site in `Config::load`
/// stays unconditional.
#[cfg(not(target_os = "macos"))]
pub fn migrate_legacy_macos_config() -> crate::error::Result<()> {
    Ok(())
}

/* --- private helpers (macOS) ----------------------------------------------------------------- */

/// Substring of the legacy macOS application-support paths used to recognise
/// stale absolute references inside `config.toml`. Detection is case-sensitive
/// to avoid mangling unrelated user content.
#[cfg(target_os = "macos")]
const LEGACY_PATH_NEEDLES: &[&str] = &[
    "Library/Application Support/com.SkyCorp.modelmux",
    "Library/Application Support/modelmux",
];

/// Core migration logic, parameterised on the legacy/destination directories
/// so it can be unit-tested with temporary directories.
#[cfg(target_os = "macos")]
fn migrate_inner(legacy_dirs: &[PathBuf], new_dir: &Path) -> Result<MigrationOutcome> {
    let mut outcome = MigrationOutcome::default();

    // Idempotency guard: once the new config exists we never touch the
    // legacy dir again. This is the contract that makes the function safe to
    // call on every startup.
    if new_dir.join("config.toml").exists() {
        // If a legacy dir also still has files we don't fail, we just record
        // the conflict so the caller can print a one-time hint.
        for legacy in legacy_dirs {
            if !legacy.is_dir() {
                continue;
            }
            let entries = match std::fs::read_dir(legacy) {
                Ok(entries) => entries,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    outcome.skipped_existing.push(path);
                }
            }
            if !outcome.skipped_existing.is_empty() {
                outcome.legacy_dir = Some(legacy.clone());
                break;
            }
        }
        return Ok(outcome);
    }

    // Find the first legacy dir that actually contains a config.toml so we
    // don't migrate empty placeholder directories.
    let legacy_dir = legacy_dirs
        .iter()
        .find(|d| d.join("config.toml").is_file())
        .cloned();

    let Some(legacy_dir) = legacy_dir else {
        return Ok(outcome);
    };

    // Make sure the destination exists (no-op if already there).
    std::fs::create_dir_all(new_dir).map_err(|e| {
        ProxyError::Config(format!(
            "Failed to create '{}' for macOS config migration: {}",
            new_dir.display(),
            e
        ))
    })?;

    // Move every file (not subdirectories) from legacy → new.
    let entries = std::fs::read_dir(&legacy_dir).map_err(|e| {
        ProxyError::Config(format!(
            "Failed to read legacy config dir '{}': {}",
            legacy_dir.display(),
            e
        ))
    })?;

    for entry in entries.flatten() {
        let src = entry.path();
        if !src.is_file() {
            continue;
        }
        let Some(file_name) = src.file_name().map(|s| s.to_owned()) else {
            continue;
        };
        let dst = new_dir.join(&file_name);

        // Never clobber an existing file in the new location.
        if dst.exists() {
            outcome.skipped_existing.push(dst);
            continue;
        }

        // Try a fast rename first; fall back to copy+remove if rename fails
        // (e.g. across filesystems). Either path leaves the file in `dst`.
        if let Err(rename_err) = std::fs::rename(&src, &dst) {
            std::fs::copy(&src, &dst).map_err(|copy_err| {
                ProxyError::Config(format!(
                    "Failed to migrate '{}' to '{}': rename failed ({}), copy fallback failed ({})",
                    src.display(),
                    dst.display(),
                    rename_err,
                    copy_err
                ))
            })?;
            // Best-effort cleanup of the source; if it can't be removed the
            // user is no worse off than before (file exists in both places).
            let _ = std::fs::remove_file(&src);
        }

        outcome.moved.push(dst);
    }

    outcome.legacy_dir = Some(legacy_dir.clone());

    // Rewrite absolute legacy paths inside the freshly-moved config.toml so
    // entries like `service_account_file = ".../Library/Application Support/com.SkyCorp.modelmux/..."`
    // still resolve.
    let new_config = new_dir.join("config.toml");
    if new_config.is_file() {
        outcome.rewrote_paths = rewrite_legacy_paths_in_config(&new_config, new_dir)?;
    }

    // Try to remove the now-empty legacy directory. Failure is non-fatal —
    // we'll leave the empty dir for the user to clean up.
    let _ = std::fs::remove_dir(&legacy_dir);

    Ok(outcome)
}

/// Replace any absolute legacy macOS application-support paths inside the
/// given config.toml with the new directory. Returns `true` when the file was
/// rewritten.
#[cfg(target_os = "macos")]
fn rewrite_legacy_paths_in_config(config_file: &Path, new_dir: &Path) -> Result<bool> {
    let original = std::fs::read_to_string(config_file).map_err(|e| {
        ProxyError::Config(format!(
            "Failed to read '{}' for path rewriting: {}",
            config_file.display(),
            e
        ))
    })?;

    let new_dir_str = new_dir.to_string_lossy();
    let mut updated = original.clone();

    for needle in LEGACY_PATH_NEEDLES {
        // Walk every occurrence of the needle, then back up to the start of
        // the absolute path (the nearest `/`) and forward to the end of the
        // legacy directory portion. Replace that whole span with `new_dir`.
        loop {
            let Some(idx) = updated.find(needle) else { break };
            // Find the start of the absolute path containing this needle.
            let start = updated[..idx].rfind('/').map(|p| {
                // Walk back further while we keep seeing path chars (so
                // `/Users/me/Library/...` is captured from the leading `/`).
                let bytes = updated.as_bytes();
                let mut s = p;
                while s > 0 {
                    let prev = s - 1;
                    let c = bytes[prev] as char;
                    if c == '"' || c == '\'' || c == ' ' || c == '=' || c == '\n' {
                        break;
                    }
                    s = prev;
                }
                s
            });
            let Some(start) = start else { break };

            let end = idx + needle.len();
            let mut next = String::with_capacity(updated.len());
            next.push_str(&updated[..start]);
            next.push_str(&new_dir_str);
            next.push_str(&updated[end..]);
            updated = next;
        }
    }

    if updated != original {
        std::fs::write(config_file, &updated).map_err(|e| {
            ProxyError::Config(format!(
                "Failed to write updated paths into '{}': {}",
                config_file.display(),
                e
            ))
        })?;
        return Ok(true);
    }

    Ok(false)
}

#[cfg(target_os = "macos")]
fn emit_success_message(outcome: &MigrationOutcome, new_dir: &Path) {
    eprintln!(
        "✅ ModelMux: migrated configuration to XDG-style macOS location\n   \
         New location: {}",
        new_dir.display()
    );
    if let Some(legacy) = &outcome.legacy_dir {
        eprintln!("   Migrated from: {}", legacy.display());
    }
    if !outcome.moved.is_empty() {
        eprintln!("   Moved files:");
        for f in &outcome.moved {
            eprintln!(
                "     - {}",
                f.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
            );
        }
    }
    if outcome.rewrote_paths {
        eprintln!(
            "   Rewrote absolute paths in config.toml to point at the new location."
        );
    }
    if !outcome.skipped_existing.is_empty() {
        eprintln!("   Skipped (already present in new location):");
        for f in &outcome.skipped_existing {
            eprintln!(
                "     - {}",
                f.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
            );
        }
    }
}

#[cfg(target_os = "macos")]
fn emit_skip_hint(legacy_dir: &Path, new_dir: &Path, skipped: &[PathBuf]) {
    eprintln!(
        "ℹ️  ModelMux: both new and legacy macOS config locations contain files.\n   \
         New (in use): {}\n   \
         Legacy:       {}\n   \
         The new location is authoritative. You can remove the legacy directory\n   \
         once you've confirmed nothing important is left behind:",
        new_dir.display(),
        legacy_dir.display(),
    );
    for f in skipped {
        eprintln!(
            "     - {} (still present at legacy)",
            f.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
        );
    }
}

/* --- tests ----------------------------------------------------------------------------------- */

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    #[test]
    fn migrates_files_and_rewrites_absolute_paths() {
        let tmp = TempDir::new().unwrap();
        let legacy =
            tmp.path().join("Users/me/Library/Application Support/com.SkyCorp.modelmux");
        let new_dir = tmp.path().join("Users/me/.config/modelmux");

        let legacy_sa = legacy.join("service-account.json");
        write(&legacy.join("config.toml"),
            &format!(
                "[auth]\nservice_account_file = \"{}\"\n",
                legacy_sa.display()
            ),
        );
        write(&legacy_sa, "{\"type\":\"service_account\"}");

        let outcome = migrate_inner(&[legacy.clone()], &new_dir).expect("migration succeeds");

        assert_eq!(outcome.moved.len(), 2, "should move config.toml and service-account.json");
        assert!(outcome.rewrote_paths, "should rewrite the absolute legacy path");
        assert_eq!(outcome.legacy_dir.as_deref(), Some(legacy.as_path()));

        assert!(new_dir.join("config.toml").is_file());
        assert!(new_dir.join("service-account.json").is_file());
        assert!(
            !legacy.exists() || fs::read_dir(&legacy).map(|d| d.count()).unwrap_or(0) == 0,
            "legacy dir should be removed or emptied"
        );

        let rewritten = fs::read_to_string(new_dir.join("config.toml")).unwrap();
        assert!(
            rewritten.contains(&new_dir.to_string_lossy().to_string()),
            "rewritten config should reference the new dir: {}",
            rewritten
        );
        assert!(
            !rewritten.contains("Library/Application Support/com.SkyCorp.modelmux"),
            "rewritten config must not still contain the legacy path: {}",
            rewritten
        );
    }

    #[test]
    fn idempotent_no_op_when_new_config_already_exists() {
        let tmp = TempDir::new().unwrap();
        let legacy =
            tmp.path().join("Users/me/Library/Application Support/com.SkyCorp.modelmux");
        let new_dir = tmp.path().join("Users/me/.config/modelmux");

        write(&legacy.join("config.toml"), "[server]\nport = 1\n");
        write(&new_dir.join("config.toml"), "[server]\nport = 2\n");

        let outcome = migrate_inner(&[legacy.clone()], &new_dir).expect("idempotent run");

        assert!(outcome.moved.is_empty(), "must not move anything when new config exists");
        assert!(!outcome.rewrote_paths);
        // Legacy untouched
        assert!(legacy.join("config.toml").is_file());
        // New config not modified
        let body = fs::read_to_string(new_dir.join("config.toml")).unwrap();
        assert!(body.contains("port = 2"));
    }

    #[test]
    fn does_not_clobber_existing_files_in_new_dir() {
        let tmp = TempDir::new().unwrap();
        let legacy =
            tmp.path().join("Users/me/Library/Application Support/com.SkyCorp.modelmux");
        let new_dir = tmp.path().join("Users/me/.config/modelmux");

        // Legacy has both files, new dir has only service-account.json. The
        // outer guard (new_dir/config.toml absent) lets migration run, but
        // service-account.json must be skipped, not overwritten.
        write(&legacy.join("config.toml"), "[server]\nport = 1\n");
        write(&legacy.join("service-account.json"), "{\"legacy\":true}");
        write(&new_dir.join("service-account.json"), "{\"new\":true}");

        let outcome = migrate_inner(&[legacy.clone()], &new_dir).expect("migration succeeds");

        assert_eq!(outcome.moved.len(), 1, "only config.toml should be moved");
        assert_eq!(outcome.skipped_existing.len(), 1);
        let preserved = fs::read_to_string(new_dir.join("service-account.json")).unwrap();
        assert!(preserved.contains("\"new\":true"), "new file must not be overwritten");
    }

    #[test]
    fn no_op_when_no_legacy_config_present() {
        let tmp = TempDir::new().unwrap();
        let legacy =
            tmp.path().join("Users/me/Library/Application Support/com.SkyCorp.modelmux");
        let new_dir = tmp.path().join("Users/me/.config/modelmux");
        fs::create_dir_all(&legacy).unwrap();

        let outcome = migrate_inner(&[legacy], &new_dir).expect("no-op succeeds");

        assert!(outcome.moved.is_empty());
        assert!(outcome.legacy_dir.is_none());
        assert!(!outcome.rewrote_paths);
    }
}
