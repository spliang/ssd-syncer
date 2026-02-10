use anyhow::{Context, Result};
use std::path::Path;

use crate::config::{AppConfig, ConflictStrategy};
use crate::diff::{ConflictInfo, SyncAction, SyncPlan};
use crate::ignore::IgnoreMatcher;
use crate::scanner;
use crate::snapshot::Snapshot;

pub struct SyncEngine {
    pub machine_name: String,
    pub conflict_strategy: ConflictStrategy,
    pub dry_run: bool,
}

pub struct SyncResult {
    pub copied_to_ssd: usize,
    pub copied_to_local: usize,
    pub deleted_from_ssd: usize,
    pub deleted_from_local: usize,
    pub conflicts: usize,
    pub errors: Vec<String>,
}

impl SyncResult {
    fn new() -> Self {
        Self {
            copied_to_ssd: 0,
            copied_to_local: 0,
            deleted_from_ssd: 0,
            deleted_from_local: 0,
            conflicts: 0,
            errors: vec![],
        }
    }

    pub fn total_actions(&self) -> usize {
        self.copied_to_ssd
            + self.copied_to_local
            + self.deleted_from_ssd
            + self.deleted_from_local
            + self.conflicts
    }
}

impl SyncEngine {
    pub fn new(machine_name: &str, conflict_strategy: ConflictStrategy, dry_run: bool) -> Self {
        Self {
            machine_name: machine_name.to_string(),
            conflict_strategy,
            dry_run,
        }
    }

    pub fn execute_plan(
        &self,
        plan: &SyncPlan,
        local_root: &Path,
        ssd_root: &Path,
    ) -> Result<SyncResult> {
        let mut result = SyncResult::new();

        for entry in &plan.actions {
            match &entry.action {
                SyncAction::CopyToSsd => {
                    if let Err(e) = self.copy_file(
                        &local_root.join(&entry.path),
                        &ssd_root.join(&entry.path),
                    ) {
                        result
                            .errors
                            .push(format!("CopyToSsd {}: {}", entry.path, e));
                    } else {
                        result.copied_to_ssd += 1;
                    }
                }
                SyncAction::CopyToLocal => {
                    if let Err(e) = self.copy_file(
                        &ssd_root.join(&entry.path),
                        &local_root.join(&entry.path),
                    ) {
                        result
                            .errors
                            .push(format!("CopyToLocal {}: {}", entry.path, e));
                    } else {
                        result.copied_to_local += 1;
                    }
                }
                SyncAction::DeleteFromSsd => {
                    if let Err(e) = self.delete_file(&ssd_root.join(&entry.path)) {
                        result
                            .errors
                            .push(format!("DeleteFromSsd {}: {}", entry.path, e));
                    } else {
                        result.deleted_from_ssd += 1;
                    }
                }
                SyncAction::DeleteFromLocal => {
                    if let Err(e) = self.delete_file(&local_root.join(&entry.path)) {
                        result
                            .errors
                            .push(format!("DeleteFromLocal {}: {}", entry.path, e));
                    } else {
                        result.deleted_from_local += 1;
                    }
                }
                SyncAction::Conflict(info) => {
                    if let Err(e) =
                        self.handle_conflict(&entry.path, info, local_root, ssd_root)
                    {
                        result
                            .errors
                            .push(format!("Conflict {}: {}", entry.path, e));
                    } else {
                        result.conflicts += 1;
                    }
                }
            }
        }

        Ok(result)
    }

    fn copy_file(&self, src: &Path, dst: &Path) -> Result<()> {
        if self.dry_run {
            log::info!("[DRY RUN] Copy {} -> {}", src.display(), dst.display());
            return Ok(());
        }

        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create dir: {}", parent.display()))?;
        }

        std::fs::copy(src, dst).with_context(|| {
            format!("Failed to copy {} -> {}", src.display(), dst.display())
        })?;

        log::info!("Copied {} -> {}", src.display(), dst.display());
        Ok(())
    }

    fn delete_file(&self, path: &Path) -> Result<()> {
        if self.dry_run {
            log::info!("[DRY RUN] Delete {}", path.display());
            return Ok(());
        }

        if path.exists() {
            std::fs::remove_file(path)
                .with_context(|| format!("Failed to delete: {}", path.display()))?;
            log::info!("Deleted {}", path.display());

            // Clean up empty parent directories
            self.cleanup_empty_parents(path)?;
        }

        Ok(())
    }

    fn cleanup_empty_parents(&self, path: &Path) -> Result<()> {
        let mut current = path.parent();
        while let Some(dir) = current {
            if dir.read_dir()?.next().is_none() {
                std::fs::remove_dir(dir).ok();
                current = dir.parent();
            } else {
                break;
            }
        }
        Ok(())
    }

    fn handle_conflict(
        &self,
        rel_path: &str,
        _info: &ConflictInfo,
        local_root: &Path,
        ssd_root: &Path,
    ) -> Result<()> {
        let local_path = local_root.join(rel_path);
        let ssd_path = ssd_root.join(rel_path);

        match &self.conflict_strategy {
            ConflictStrategy::Both => {
                self.resolve_both(rel_path, &local_path, &ssd_path, local_root, ssd_root)
            }
            ConflictStrategy::LocalWins => {
                // Local version wins: copy local to SSD
                if local_path.exists() {
                    self.copy_file(&local_path, &ssd_path)
                } else {
                    self.delete_file(&ssd_path)
                }
            }
            ConflictStrategy::SsdWins => {
                // SSD version wins: copy SSD to local
                if ssd_path.exists() {
                    self.copy_file(&ssd_path, &local_path)
                } else {
                    self.delete_file(&local_path)
                }
            }
            ConflictStrategy::NewerWins => {
                self.resolve_newer(&local_path, &ssd_path)
            }
            ConflictStrategy::Ask => {
                // In non-interactive mode, fall back to Both
                log::warn!(
                    "Conflict on '{}': interactive mode not available, keeping both versions",
                    rel_path
                );
                self.resolve_both(rel_path, &local_path, &ssd_path, local_root, ssd_root)
            }
        }
    }

    fn resolve_both(
        &self,
        rel_path: &str,
        local_path: &Path,
        ssd_path: &Path,
        local_root: &Path,
        ssd_root: &Path,
    ) -> Result<()> {
        let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");

        if self.dry_run {
            log::info!(
                "[DRY RUN] Conflict '{}': would keep both versions",
                rel_path
            );
            return Ok(());
        }

        // Generate conflict file names
        let path_obj = Path::new(rel_path);
        let stem = path_obj
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let extension = path_obj
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();
        let parent = path_obj.parent().unwrap_or(Path::new(""));

        let conflict_name = format!(
            "{}.conflict.{}.{}{}",
            stem, self.machine_name, timestamp, extension
        );
        let conflict_rel = if parent == Path::new("") {
            conflict_name.clone()
        } else {
            format!("{}/{}", parent.display(), conflict_name)
        };

        // Keep SSD version as-is in both locations
        // Rename local version with conflict suffix in both locations
        if local_path.exists() && ssd_path.exists() {
            // Copy SSD version to local (overwrite local with SSD version)
            let local_conflict = local_root.join(&conflict_rel);
            // Rename current local file to conflict name
            if let Some(p) = local_conflict.parent() {
                std::fs::create_dir_all(p)?;
            }
            std::fs::rename(local_path, &local_conflict)?;
            // Copy SSD version to local
            self.copy_file(ssd_path, local_path)?;
            // Also copy conflict version to SSD
            let ssd_conflict = ssd_root.join(&conflict_rel);
            self.copy_file(&local_conflict, &ssd_conflict)?;

            log::warn!(
                "Conflict '{}': kept both. SSD version → original name, local version → '{}'",
                rel_path,
                conflict_rel
            );
        } else if local_path.exists() {
            // SSD was deleted but local was modified → keep local, copy to SSD
            self.copy_file(local_path, ssd_path)?;
            log::warn!(
                "Conflict '{}': SSD deleted but local modified → kept local version",
                rel_path
            );
        } else if ssd_path.exists() {
            // Local was deleted but SSD was modified → keep SSD, copy to local
            self.copy_file(ssd_path, local_path)?;
            log::warn!(
                "Conflict '{}': local deleted but SSD modified → kept SSD version",
                rel_path
            );
        }

        Ok(())
    }

    fn resolve_newer(&self, local_path: &Path, ssd_path: &Path) -> Result<()> {
        let local_mtime = local_path
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let ssd_mtime = ssd_path
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        if local_mtime >= ssd_mtime {
            if local_path.exists() {
                self.copy_file(local_path, ssd_path)
            } else {
                self.delete_file(ssd_path)
            }
        } else {
            if ssd_path.exists() {
                self.copy_file(ssd_path, local_path)
            } else {
                self.delete_file(local_path)
            }
        }
    }
}

/// Run a full sync for one mapping.
pub fn sync_one_mapping(
    local_root: &Path,
    ssd_data_root: &Path,
    ssd_rel: &str,
    machine_name: &str,
    ignore: &IgnoreMatcher,
    conflict_strategy: &ConflictStrategy,
    dry_run: bool,
) -> Result<(SyncPlan, SyncResult)> {
    let ssd_folder = ssd_data_root.join(ssd_rel);

    // Ensure SSD folder exists
    if !ssd_folder.exists() {
        std::fs::create_dir_all(&ssd_folder)?;
    }

    // Load base snapshot (last sync state for this machine)
    let snapshot_dir =
        AppConfig::ssd_snapshots_dir(ssd_data_root, machine_name);
    let snapshot_file = snapshot_dir.join(Snapshot::snapshot_filename(ssd_rel));
    let base_snapshot = Snapshot::load_or_empty(&snapshot_file, ssd_rel, machine_name)?;

    // Scan both directories
    let (local_snap, ssd_snap) =
        scanner::scan_pair(local_root, &ssd_folder, ssd_rel, machine_name, ignore, Some(&base_snapshot))?;

    // Compute changes
    let local_changes = crate::diff::compute_changes(&base_snapshot, &local_snap);
    let ssd_changes = crate::diff::compute_changes(&base_snapshot, &ssd_snap);

    log::info!(
        "Changes: {} local, {} SSD",
        local_changes.len(),
        ssd_changes.len()
    );

    // Build sync plan
    let plan = crate::diff::build_sync_plan(&local_changes, &ssd_changes);

    if plan.actions.is_empty() {
        log::info!("No changes to sync for '{}'", ssd_rel);
        return Ok((plan, SyncResult::new()));
    }

    // Execute
    let engine = SyncEngine::new(machine_name, conflict_strategy.clone(), dry_run);
    let result = engine.execute_plan(&plan, local_root, &ssd_folder)?;

    // Update snapshot (re-scan after sync to capture actual state)
    if !dry_run {
        let final_local = scanner::scan_directory(local_root, ssd_rel, machine_name, ignore, None)?;
        let mut final_snapshot = final_local;
        final_snapshot.synced_at = chrono::Utc::now();
        final_snapshot.save(&snapshot_file)?;
        log::info!("Snapshot updated: {}", snapshot_file.display());
    }

    Ok((plan, result))
}
