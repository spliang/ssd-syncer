use anyhow::{Context, Result};
use std::path::Path;
use walkdir::WalkDir;

use crate::ignore::IgnoreMatcher;
use crate::snapshot::{FileEntry, Snapshot};

pub fn scan_directory(
    root: &Path,
    sync_folder: &str,
    machine: &str,
    ignore: &IgnoreMatcher,
    base_snapshot: Option<&Snapshot>,
) -> Result<Snapshot> {
    let mut snapshot = Snapshot::new(sync_folder, machine);

    if !root.exists() {
        anyhow::bail!("Directory does not exist: {}", root.display());
    }

    if !root.is_dir() {
        anyhow::bail!("Path is not a directory: {}", root.display());
    }

    for entry in WalkDir::new(root).follow_links(false) {
        let entry = entry.with_context(|| format!("Failed to walk directory: {}", root.display()))?;

        if entry.file_type().is_dir() {
            continue;
        }

        if !entry.file_type().is_file() {
            continue;
        }

        let abs_path = entry.path();
        let rel_path = abs_path
            .strip_prefix(root)
            .with_context(|| "Failed to compute relative path")?;

        // Normalize to forward slashes
        let rel_str = rel_path
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join("/");

        if ignore.is_ignored(&rel_str) {
            continue;
        }

        let metadata = std::fs::metadata(abs_path)
            .with_context(|| format!("Failed to read metadata: {}", abs_path.display()))?;

        let size = metadata.len();
        let mtime_secs = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        // Optimization: check if file changed since last snapshot
        let needs_hash = if let Some(base) = base_snapshot {
            if let Some(prev_entry) = base.files.get(&rel_str) {
                // If size and mtime match, reuse previous hash
                if prev_entry.size == size && prev_entry.mtime_secs == mtime_secs {
                    snapshot.files.insert(rel_str, prev_entry.clone());
                    continue;
                }
                true
            } else {
                true // New file
            }
        } else {
            true // No base snapshot, must hash
        };

        let hash = if needs_hash {
            compute_file_hash(abs_path)?
        } else {
            unreachable!()
        };

        snapshot.files.insert(
            rel_str,
            FileEntry {
                size,
                mtime_secs,
                hash,
            },
        );
    }

    Ok(snapshot)
}

pub fn compute_file_hash(path: &Path) -> Result<String> {
    let content = std::fs::read(path)
        .with_context(|| format!("Failed to read file for hashing: {}", path.display()))?;
    let hash = blake3::hash(&content);
    Ok(format!("blake3:{}", hash.to_hex()))
}

pub fn scan_pair(
    local_root: &Path,
    ssd_root: &Path,
    sync_folder: &str,
    machine: &str,
    ignore: &IgnoreMatcher,
    base_snapshot: Option<&Snapshot>,
) -> Result<(Snapshot, Snapshot)> {
    log::info!("Scanning local: {}", local_root.display());
    let local_snap = scan_directory(local_root, sync_folder, machine, ignore, base_snapshot)?;
    log::info!(
        "Local scan complete: {} files",
        local_snap.files.len()
    );

    log::info!("Scanning SSD: {}", ssd_root.display());
    let ssd_snap = scan_directory(ssd_root, sync_folder, machine, ignore, base_snapshot)?;
    log::info!(
        "SSD scan complete: {} files",
        ssd_snap.files.len()
    );

    Ok((local_snap, ssd_snap))
}
