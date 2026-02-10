use std::collections::{BTreeMap, BTreeSet};

use crate::snapshot::{FileEntry, Snapshot};

#[derive(Debug, Clone, PartialEq)]
pub enum ChangeType {
    Added,
    Modified,
    Deleted,
}

#[derive(Debug, Clone)]
pub struct FileChange {
    pub path: String,
    pub change_type: ChangeType,
    pub entry: Option<FileEntry>, // Current entry (None if deleted)
}

#[derive(Debug, Clone, PartialEq)]
pub enum SyncAction {
    CopyToSsd,
    CopyToLocal,
    DeleteFromSsd,
    DeleteFromLocal,
    Conflict(ConflictInfo),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConflictInfo {
    pub local_change: ChangeType,
    pub ssd_change: ChangeType,
}

#[derive(Debug, Clone)]
pub struct SyncPlan {
    pub actions: Vec<SyncPlanEntry>,
}

#[derive(Debug, Clone)]
pub struct SyncPlanEntry {
    pub path: String,
    pub action: SyncAction,
}

impl SyncPlan {
    pub fn has_conflicts(&self) -> bool {
        self.actions
            .iter()
            .any(|a| matches!(a.action, SyncAction::Conflict(_)))
    }

    pub fn conflict_count(&self) -> usize {
        self.actions
            .iter()
            .filter(|a| matches!(a.action, SyncAction::Conflict(_)))
            .count()
    }

    pub fn non_conflict_count(&self) -> usize {
        self.actions.len() - self.conflict_count()
    }
}

/// Compute changeset between a base snapshot and a current snapshot.
pub fn compute_changes(
    base: &Snapshot,
    current: &Snapshot,
) -> Vec<FileChange> {
    let mut changes = Vec::new();

    let all_paths: BTreeSet<&String> = base
        .files
        .keys()
        .chain(current.files.keys())
        .collect();

    for path in all_paths {
        let in_base = base.files.get(path);
        let in_current = current.files.get(path);

        match (in_base, in_current) {
            (None, Some(entry)) => {
                changes.push(FileChange {
                    path: path.clone(),
                    change_type: ChangeType::Added,
                    entry: Some(entry.clone()),
                });
            }
            (Some(_), None) => {
                changes.push(FileChange {
                    path: path.clone(),
                    change_type: ChangeType::Deleted,
                    entry: None,
                });
            }
            (Some(base_entry), Some(cur_entry)) => {
                if base_entry.hash != cur_entry.hash {
                    changes.push(FileChange {
                        path: path.clone(),
                        change_type: ChangeType::Modified,
                        entry: Some(cur_entry.clone()),
                    });
                }
            }
            (None, None) => unreachable!(),
        }
    }

    changes
}

/// Build a sync plan by merging local and SSD changesets.
pub fn build_sync_plan(
    local_changes: &[FileChange],
    ssd_changes: &[FileChange],
) -> SyncPlan {
    let local_map: BTreeMap<&str, &FileChange> = local_changes
        .iter()
        .map(|c| (c.path.as_str(), c))
        .collect();

    let ssd_map: BTreeMap<&str, &FileChange> = ssd_changes
        .iter()
        .map(|c| (c.path.as_str(), c))
        .collect();

    let all_paths: BTreeSet<&str> = local_map
        .keys()
        .chain(ssd_map.keys())
        .copied()
        .collect();

    let mut actions = Vec::new();

    for path in all_paths {
        let local_change = local_map.get(path);
        let ssd_change = ssd_map.get(path);

        let action = match (local_change, ssd_change) {
            // Only local changed
            (Some(lc), None) => match lc.change_type {
                ChangeType::Added | ChangeType::Modified => SyncAction::CopyToSsd,
                ChangeType::Deleted => SyncAction::DeleteFromSsd,
            },
            // Only SSD changed
            (None, Some(sc)) => match sc.change_type {
                ChangeType::Added | ChangeType::Modified => SyncAction::CopyToLocal,
                ChangeType::Deleted => SyncAction::DeleteFromLocal,
            },
            // Both changed
            (Some(lc), Some(sc)) => {
                // If both made the same change (same hash), no conflict
                if lc.change_type == sc.change_type {
                    match (&lc.change_type, &lc.entry, &sc.entry) {
                        (ChangeType::Deleted, _, _) => {
                            // Both deleted, nothing to do — skip
                            continue;
                        }
                        (_, Some(le), Some(se)) if le.hash == se.hash => {
                            // Both modified/added to same content — skip
                            continue;
                        }
                        _ => SyncAction::Conflict(ConflictInfo {
                            local_change: lc.change_type.clone(),
                            ssd_change: sc.change_type.clone(),
                        }),
                    }
                } else {
                    SyncAction::Conflict(ConflictInfo {
                        local_change: lc.change_type.clone(),
                        ssd_change: sc.change_type.clone(),
                    })
                }
            }
            (None, None) => unreachable!(),
        };

        actions.push(SyncPlanEntry {
            path: path.to_string(),
            action,
        });
    }

    SyncPlan { actions }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::{FileEntry, Snapshot};

    fn make_entry(hash: &str) -> FileEntry {
        FileEntry {
            size: 100,
            mtime_secs: 1000,
            hash: hash.to_string(),
        }
    }

    #[test]
    fn test_compute_changes_added() {
        let base = Snapshot::new("test", "mac");
        let mut current = Snapshot::new("test", "mac");
        current
            .files
            .insert("new.txt".to_string(), make_entry("hash1"));

        let changes = compute_changes(&base, &current);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].change_type, ChangeType::Added);
    }

    #[test]
    fn test_compute_changes_deleted() {
        let mut base = Snapshot::new("test", "mac");
        base.files
            .insert("old.txt".to_string(), make_entry("hash1"));
        let current = Snapshot::new("test", "mac");

        let changes = compute_changes(&base, &current);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].change_type, ChangeType::Deleted);
    }

    #[test]
    fn test_compute_changes_modified() {
        let mut base = Snapshot::new("test", "mac");
        base.files
            .insert("file.txt".to_string(), make_entry("hash1"));
        let mut current = Snapshot::new("test", "mac");
        current
            .files
            .insert("file.txt".to_string(), make_entry("hash2"));

        let changes = compute_changes(&base, &current);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].change_type, ChangeType::Modified);
    }

    #[test]
    fn test_sync_plan_local_add() {
        let local_changes = vec![FileChange {
            path: "new.txt".to_string(),
            change_type: ChangeType::Added,
            entry: Some(make_entry("hash1")),
        }];
        let plan = build_sync_plan(&local_changes, &[]);
        assert_eq!(plan.actions.len(), 1);
        assert_eq!(plan.actions[0].action, SyncAction::CopyToSsd);
    }

    #[test]
    fn test_sync_plan_conflict() {
        let local_changes = vec![FileChange {
            path: "file.txt".to_string(),
            change_type: ChangeType::Modified,
            entry: Some(make_entry("hash_local")),
        }];
        let ssd_changes = vec![FileChange {
            path: "file.txt".to_string(),
            change_type: ChangeType::Modified,
            entry: Some(make_entry("hash_ssd")),
        }];
        let plan = build_sync_plan(&local_changes, &ssd_changes);
        assert_eq!(plan.actions.len(), 1);
        assert!(matches!(plan.actions[0].action, SyncAction::Conflict(_)));
    }
}
