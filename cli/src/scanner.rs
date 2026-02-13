use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::io::Write;
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

    // Collect all directories for empty-dir detection
    let mut all_dirs: BTreeSet<String> = BTreeSet::new();
    // Track which directories contain files (directly or indirectly)
    let mut non_empty_dirs: BTreeSet<String> = BTreeSet::new();

    let mut file_count: usize = 0;

    let walker = WalkDir::new(root).follow_links(false).into_iter();
    // 使用 filter_entry 跳过忽略目录的整个子树
    for entry in walker.filter_entry(|e| {
        let rel = e.path().strip_prefix(root).unwrap_or(e.path());
        let rel_str = rel
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join("/");
        rel_str.is_empty() || !ignore.is_ignored(&rel_str)
    }) {
        let entry = entry.with_context(|| format!("Failed to walk directory: {}", root.display()))?;

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

        if rel_str.is_empty() {
            continue; // Skip root itself
        }

        if entry.file_type().is_dir() {
            all_dirs.insert(rel_str);
            continue;
        }

        if !entry.file_type().is_file() {
            continue;
        }

        // Mark all ancestor directories as non-empty
        let mut ancestor = rel_path.parent();
        while let Some(p) = ancestor {
            if p == Path::new("") {
                break;
            }
            let ancestor_str = p
                .components()
                .map(|c| c.as_os_str().to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join("/");
            non_empty_dirs.insert(ancestor_str);
            ancestor = p.parent();
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
                is_dir: false,
            },
        );

        file_count += 1;
        if file_count % 100 == 0 {
            print!("\r  Scanning... {} files", file_count);
            let _ = std::io::stdout().flush();
        }
    }

    // 清除进度行
    if file_count >= 100 {
        print!("\r{}", " ".repeat(40));
        print!("\r");
        let _ = std::io::stdout().flush();
    }

    // Add empty directories to the snapshot
    for dir in &all_dirs {
        if !non_empty_dirs.contains(dir) {
            snapshot.files.insert(
                dir.clone(),
                FileEntry {
                    size: 0,
                    mtime_secs: 0,
                    hash: "empty-dir".to_string(),
                    is_dir: true,
                },
            );
        }
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
    local_cache: Option<&Snapshot>,
    ssd_cache: Option<&Snapshot>,
) -> Result<(Snapshot, Snapshot)> {
    log::info!("Scanning local + SSD in parallel...");

    // 并行扫描本地和 SSD 目录，大幅减少总扫描时间
    let (local_result, ssd_result) = std::thread::scope(|s| {
        let local_handle = s.spawn(|| {
            scan_directory(local_root, sync_folder, machine, ignore, local_cache)
        });
        let ssd_handle = s.spawn(|| {
            scan_directory(ssd_root, sync_folder, machine, ignore, ssd_cache)
        });

        let local_res = local_handle.join().expect("local scan thread panicked");
        let ssd_res = ssd_handle.join().expect("SSD scan thread panicked");
        (local_res, ssd_res)
    });

    let local_snap = local_result?;
    let ssd_snap = ssd_result?;

    log::info!(
        "Scan complete: {} local files, {} SSD files",
        local_snap.files.len(),
        ssd_snap.files.len()
    );

    Ok((local_snap, ssd_snap))
}
