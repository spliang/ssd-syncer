use anyhow::{Context, Result};
use std::io::Write;
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
    pub verbose: bool,
}

pub struct SyncResult {
    pub copied_to_ssd: usize,
    pub copied_to_local: usize,
    pub deleted_from_ssd: usize,
    pub deleted_from_local: usize,
    pub conflicts: usize,
    pub errors: Vec<String>,
    pub total_files: usize,
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
            total_files: 0,
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
    pub fn new(machine_name: &str, conflict_strategy: ConflictStrategy, dry_run: bool, verbose: bool) -> Self {
        Self {
            machine_name: machine_name.to_string(),
            conflict_strategy,
            dry_run,
            verbose,
        }
    }

    pub fn execute_plan(
        &self,
        plan: &SyncPlan,
        local_root: &Path,
        ssd_root: &Path,
    ) -> Result<SyncResult> {
        let mut result = SyncResult::new();
        let total = plan.actions.len();

        for (idx, entry) in plan.actions.iter().enumerate() {
            let progress = format!("[{}/{}]", idx + 1, total);
            let action_desc = match &entry.action {
                SyncAction::CopyToSsd => "→ SSD",
                SyncAction::CopyToLocal => "← Local",
                SyncAction::DeleteFromSsd => "✕ SSD",
                SyncAction::DeleteFromLocal => "✕ Local",
                SyncAction::Conflict(_) => "⚠ Conflict",
            };
            if self.verbose {
                println!("  {} {} {}", progress, action_desc, entry.path);
            } else {
                print!("\r  {} {} {}", progress, action_desc, entry.path);
                // 用空格覆盖可能的残留字符
                print!("{}", " ".repeat(10));
                let _ = std::io::stdout().flush();
            }
            match &entry.action {
                SyncAction::CopyToSsd => {
                    if entry.is_dir {
                        if let Err(e) = self.create_dir(&ssd_root.join(&entry.path)) {
                            result.errors.push(format!("CreateDirSsd {}: {}", entry.path, e));
                        } else {
                            result.copied_to_ssd += 1;
                        }
                    } else if let Err(e) = self.copy_file(
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
                    if entry.is_dir {
                        if let Err(e) = self.create_dir(&local_root.join(&entry.path)) {
                            result.errors.push(format!("CreateDirLocal {}: {}", entry.path, e));
                        } else {
                            result.copied_to_local += 1;
                        }
                    } else if let Err(e) = self.copy_file(
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
                    if entry.is_dir {
                        if let Err(e) = self.delete_dir(&ssd_root.join(&entry.path)) {
                            result.errors.push(format!("DeleteDirSsd {}: {}", entry.path, e));
                        } else {
                            result.deleted_from_ssd += 1;
                        }
                    } else if let Err(e) = self.delete_file(&ssd_root.join(&entry.path)) {
                        result
                            .errors
                            .push(format!("DeleteFromSsd {}: {}", entry.path, e));
                    } else {
                        result.deleted_from_ssd += 1;
                    }
                }
                SyncAction::DeleteFromLocal => {
                    if entry.is_dir {
                        if let Err(e) = self.delete_dir(&local_root.join(&entry.path)) {
                            result.errors.push(format!("DeleteDirLocal {}: {}", entry.path, e));
                        } else {
                            result.deleted_from_local += 1;
                        }
                    } else if let Err(e) = self.delete_file(&local_root.join(&entry.path)) {
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

        // compact 模式下清除进度行
        if !self.verbose && total > 0 {
            print!("\r{}", " ".repeat(80));
            print!("\r");
            let _ = std::io::stdout().flush();
        }

        // 通知 Windows 资源管理器刷新所有受影响的目录
        if !self.dry_run && result.total_actions() > 0 {
            let mut affected_dirs: std::collections::BTreeSet<std::path::PathBuf> = std::collections::BTreeSet::new();
            // 根目录始终需要通知
            affected_dirs.insert(local_root.to_path_buf());
            affected_dirs.insert(ssd_root.to_path_buf());
            // 收集所有受影响文件的父目录
            for entry in &plan.actions {
                if let Some(parent) = Path::new(&entry.path).parent() {
                    if !parent.as_os_str().is_empty() {
                        affected_dirs.insert(local_root.join(parent));
                        affected_dirs.insert(ssd_root.join(parent));
                    }
                }
            }
            for dir in &affected_dirs {
                notify_shell_update(dir);
            }
        }

        Ok(result)
    }

    fn create_dir(&self, path: &Path) -> Result<()> {
        if self.dry_run {
            log::info!("[DRY RUN] Create dir {}", path.display());
            return Ok(());
        }

        std::fs::create_dir_all(path)
            .with_context(|| format!("Failed to create dir: {}", path.display()))?;
        log::debug!("Created dir {}", path.display());
        Ok(())
    }

    fn delete_dir(&self, path: &Path) -> Result<()> {
        if self.dry_run {
            log::info!("[DRY RUN] Delete dir {}", path.display());
            return Ok(());
        }

        if path.exists() && path.is_dir() {
            std::fs::remove_dir(path)
                .with_context(|| format!("Failed to delete dir: {}", path.display()))?;
            log::debug!("Deleted dir {}", path.display());
            self.cleanup_empty_parents(path)?;
        }

        Ok(())
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

        log::debug!("Copied {} -> {}", src.display(), dst.display());
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
            log::debug!("Deleted {}", path.display());

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

/// 通知操作系统文件管理器刷新目录显示
#[cfg(target_os = "windows")]
fn notify_shell_update(path: &Path) {
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "shell32")]
    extern "system" {
        fn SHChangeNotify(wEventId: i32, uFlags: u32, dwItem1: *const u16, dwItem2: *const u16);
    }

    const SHCNE_UPDATEDIR: i32 = 0x00001000;
    const SHCNF_PATHW: u32 = 0x0005;
    const SHCNF_FLUSHNOWAIT: u32 = 0x3000;

    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    unsafe {
        SHChangeNotify(SHCNE_UPDATEDIR, SHCNF_PATHW | SHCNF_FLUSHNOWAIT, wide.as_ptr(), std::ptr::null());
    }
}

#[cfg(not(target_os = "windows"))]
fn notify_shell_update(_path: &Path) {
    // macOS/Linux 文件管理器通常会自动刷新
}

/// Run a full sync for one mapping (从磁盘加载快照).
pub fn sync_one_mapping(
    local_root: &Path,
    ssd_data_root: &Path,
    ssd_rel: &str,
    machine_name: &str,
    ignore: &IgnoreMatcher,
    conflict_strategy: &ConflictStrategy,
    dry_run: bool,
    verbose: bool,
) -> Result<(SyncPlan, SyncResult)> {
    let (plan, result, _, _) = sync_one_mapping_cached(
        local_root, ssd_data_root, ssd_rel, machine_name,
        ignore, conflict_strategy, dry_run, verbose, None,
    )?;
    Ok((plan, result))
}

/// 同步一个映射（支持内存缓存快照）。
/// 接受 `cached_snapshots`: Option<(base_snapshot, ssd_cache)>，如果有则跳过磁盘加载。
/// 返回 (plan, result, 更新后的base_snapshot, 更新后的ssd_cache)。
fn sync_one_mapping_cached(
    local_root: &Path,
    ssd_data_root: &Path,
    ssd_rel: &str,
    machine_name: &str,
    ignore: &IgnoreMatcher,
    conflict_strategy: &ConflictStrategy,
    dry_run: bool,
    verbose: bool,
    cached_snapshots: Option<(Snapshot, Snapshot)>,
) -> Result<(SyncPlan, SyncResult, Snapshot, Snapshot)> {
    let ssd_folder = ssd_data_root.join(ssd_rel);

    // Ensure SSD folder exists
    if !ssd_folder.exists() {
        std::fs::create_dir_all(&ssd_folder)?;
    }

    // 快照文件路径（用于持久化保存）
    let snapshot_dir =
        AppConfig::ssd_snapshots_dir(ssd_data_root, machine_name);
    let snapshot_file = snapshot_dir.join(Snapshot::snapshot_filename(ssd_rel));
    let ssd_cache_filename = format!("{}_ssd_cache.json",
        ssd_rel.replace('/', "_").replace('\\', "_").replace(':', "_"));
    let ssd_cache_file = snapshot_dir.join(&ssd_cache_filename);

    // 使用内存缓存的快照（如果有），否则从磁盘加载
    let (base_snapshot, ssd_cache) = match cached_snapshots {
        Some((base, cache)) => {
            log::debug!("Using in-memory cached snapshots");
            (base, cache)
        }
        None => {
            let base = Snapshot::load_or_empty(&snapshot_file, ssd_rel, machine_name)?;
            let cache = Snapshot::load_or_empty(&ssd_cache_file, ssd_rel, machine_name)?;
            (base, cache)
        }
    };

    // Scan both directories (并行扫描，各自使用独立的缓存快照)
    let (local_snap, ssd_snap) =
        scanner::scan_pair(local_root, &ssd_folder, ssd_rel, machine_name, ignore,
            Some(&base_snapshot), Some(&ssd_cache))?;

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
        // 即使无需同步，也更新缓存快照以加速后续扫描
        let mut updated_base = local_snap;
        let mut updated_ssd = ssd_snap;
        if !dry_run {
            updated_base.synced_at = chrono::Utc::now();
            updated_base.save(&snapshot_file)?;
            updated_ssd.synced_at = chrono::Utc::now();
            updated_ssd.save(&ssd_cache_file)?;
        }
        return Ok((plan, SyncResult::new(), updated_base, updated_ssd));
    }

    // Execute
    let engine = SyncEngine::new(machine_name, conflict_strategy.clone(), dry_run, verbose);
    let mut result = engine.execute_plan(&plan, local_root, &ssd_folder)?;

    // Update snapshots
    // 关键：基准快照 = 本地与SSD的交集（防止同步期间新增的本地文件被误判为"SSD删除"）
    let (updated_base, updated_ssd) = if !dry_run {
        let (final_local, final_ssd) = scanner::scan_pair(
            local_root, &ssd_folder, ssd_rel, machine_name, ignore,
            Some(&local_snap), Some(&ssd_snap))?;
        result.total_files = final_local.files.len();

        // 基准快照 = 本地文件中同时存在于SSD的部分（保留本地mtime用于扫描缓存）
        let mut new_base = final_local;
        new_base.files.retain(|path, _| final_ssd.files.contains_key(path));
        new_base.synced_at = chrono::Utc::now();
        new_base.save(&snapshot_file)?;

        // SSD 侧缓存快照
        let mut new_ssd_cache = final_ssd;
        new_ssd_cache.synced_at = chrono::Utc::now();
        new_ssd_cache.save(&ssd_cache_file)?;

        log::debug!("Snapshots updated: {}", snapshot_file.display());
        (new_base, new_ssd_cache)
    } else {
        result.total_files = local_snap.files.len();
        (local_snap, ssd_snap)
    };

    Ok((plan, result, updated_base, updated_ssd))
}
