mod config;
mod diff;
mod ignore;
mod scanner;
mod snapshot;
mod sync_engine;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::Path;
use std::time::Instant;

use config::AppConfig;
use diff::SyncAction;
use ignore::IgnoreMatcher;
use snapshot::Snapshot;

#[derive(Parser)]
#[command(name = "ssd-syncer", version, about = "Sync folders via SSD across machines")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize local configuration
    Init {
        /// Machine name (unique identifier for this computer)
        #[arg(long)]
        name: String,
    },

    /// Add a sync folder mapping
    Add {
        /// Local folder path
        #[arg(long)]
        local: String,
        /// SSD target absolute path (e.g. "/Volumes/WORK_SYNC/WORK_SYNC")
        #[arg(long)]
        ssd: String,
        /// Alias name for this mapping (e.g. "WORK")
        #[arg(long)]
        name: String,
    },

    /// Remove a sync folder mapping
    Remove {
        /// Mapping name to remove
        #[arg(long)]
        name: String,
    },

    /// List all configured sync mappings
    List,

    /// Sync all configured folders with SSD
    Sync {
        /// Mapping name (optional if only one mapping exists)
        name: Option<String>,
        /// Dry run (preview only, no changes)
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        /// Verbose mode: show each file operation on a separate line
        #[arg(long, short, default_value_t = false)]
        verbose: bool,
    },

    /// Show sync status (preview changes without applying)
    Status {
        /// Mapping name (optional if only one mapping exists)
        name: Option<String>,
    },

    /// Show detailed diff between local and SSD
    Diff {
        /// Mapping name (optional if only one mapping exists)
        name: Option<String>,
    },

    /// Show sync history log
    Log {
        /// Mapping name (optional if only one mapping exists)
        name: Option<String>,
        /// Number of recent entries to show
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },

    /// Reset ignore patterns to defaults (includes common build/temp directories)
    IgnoreReset,

    /// List current ignore patterns
    IgnoreList,

    /// Add one or more ignore patterns
    IgnoreAdd {
        /// Patterns to add (file/directory names or glob patterns, e.g. "*.log" "tmp")
        patterns: Vec<String>,
    },

    /// Remove one or more ignore patterns
    IgnoreRemove {
        /// Patterns to remove
        patterns: Vec<String>,
    },

}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Init { name } => cmd_init(&name),
        Commands::Add { local, ssd, name } => cmd_add(&local, &ssd, &name),
        Commands::Remove { name } => cmd_remove(&name),
        Commands::List => cmd_list(),
        Commands::Sync { name, dry_run, verbose } => cmd_sync(name.as_deref(), dry_run, verbose),
        Commands::Status { name } => cmd_status(name.as_deref()),
        Commands::Diff { name } => cmd_diff(name.as_deref()),
        Commands::Log { name, limit } => cmd_log(name.as_deref(), limit),
        Commands::IgnoreReset => cmd_ignore_reset(),
        Commands::IgnoreList => cmd_ignore_list(),
        Commands::IgnoreAdd { patterns } => cmd_ignore_add(&patterns),
        Commands::IgnoreRemove { patterns } => cmd_ignore_remove(&patterns),
    }
}

fn cmd_init(name: &str) -> Result<()> {
    let config_path = AppConfig::config_path()?;
    if config_path.exists() {
        anyhow::bail!(
            "Config already exists at {}. Delete it first to reinitialize.",
            config_path.display()
        );
    }

    let _config = AppConfig::create_new(name)?;
    println!("Initialized ssd-syncer for machine '{}'", name);
    println!("Config saved to: {}", config_path.display());
    Ok(())
}

fn cmd_add(local: &str, ssd: &str, name: &str) -> Result<()> {
    let mut config = AppConfig::load()?;

    // Validate SSD path is absolute
    if !Path::new(ssd).is_absolute() {
        anyhow::bail!("SSD path must be an absolute path, got: '{}'", ssd);
    }

    // Check name uniqueness
    if config.find_mapping_by_name(name).is_some() {
        anyhow::bail!("Mapping with name '{}' already exists", name);
    }

    // Validate local path exists
    let local_path = Path::new(local);
    if !local_path.exists() {
        anyhow::bail!("Local path does not exist: {}", local);
    }

    config.sync.push(config::SyncMapping {
        name: Some(name.to_string()),
        local: local.to_string(),
        ssd: ssd.to_string(),
    });
    config.save()?;

    println!("Added sync mapping:");
    println!("  Name:  {}", name);
    println!("  Local: {}", local);
    println!("  SSD:   {}", ssd);
    Ok(())
}

fn cmd_remove(name: &str) -> Result<()> {
    let mut config = AppConfig::load()?;
    let before = config.sync.len();
    config.sync.retain(|m| m.name.as_deref() != Some(name));

    if config.sync.len() == before {
        anyhow::bail!("No mapping found with name '{}'", name);
    }

    config.save()?;
    println!("Removed sync mapping '{}'", name);
    Ok(())
}

fn cmd_list() -> Result<()> {
    let config = AppConfig::load()?;

    println!("Machine: {}", config.machine.name);
    if let Some(ref ssd) = config.machine.ssd_mount {
        println!("Default SSD mount: {}", ssd);
    }
    println!("Conflict strategy: {:?}", config.conflict.strategy);
    println!();

    if config.sync.is_empty() {
        println!("No sync mappings configured. Use `ssd-syncer add` to add one.");
        return Ok(());
    }

    println!("Sync mappings:");
    for (i, mapping) in config.sync.iter().enumerate() {
        if let Some(ref name) = mapping.name {
            println!("  {}. [{}]", i + 1, name);
        } else {
            println!("  {}.", i + 1);
        }
        println!("     Local: {}", mapping.local);
        println!("     SSD:   {}", mapping.ssd);
    }

    println!();
    println!("Ignore patterns: {:?}", config.ignore.patterns);
    Ok(())
}

/// Resolve mapping(s) by name. If name is given, find that mapping.
/// If name is None and only one mapping exists, auto-select it.
/// Returns (ssd_path_string, Vec of matching mappings).
fn resolve_mappings<'a>(name: Option<&str>, config: &'a AppConfig) -> Result<(String, Vec<&'a config::SyncMapping>)> {
    match name {
        Some(n) => {
            let mapping = config.find_mapping_by_name(n)
                .ok_or_else(|| anyhow::anyhow!("No mapping found with name '{}'. Use `ssd-syncer list` to see configured mappings.", n))?;
            Ok((mapping.ssd.clone(), vec![mapping]))
        }
        None => {
            if config.sync.is_empty() {
                anyhow::bail!("No sync mappings configured. Use `ssd-syncer add` to add one.");
            }
            if config.sync.len() == 1 {
                let mapping = &config.sync[0];
                Ok((mapping.ssd.clone(), vec![mapping]))
            } else {
                anyhow::bail!(
                    "Multiple mappings configured. Please specify a mapping name.\nUse `ssd-syncer list` to see configured mappings."
                );
            }
        }
    }
}

fn cmd_sync(name: Option<&str>, dry_run: bool, verbose: bool) -> Result<()> {
    let start_time = Instant::now();
    let config = AppConfig::load()?;
    let (ssd_mount_str, mappings) = resolve_mappings(name, &config)?;
    let ssd_path = Path::new(&ssd_mount_str);

    if !ssd_path.exists() {
        anyhow::bail!("SSD mount point does not exist: {}", ssd_mount_str);
    }

    // Ensure .ssd-syncer directory on SSD
    let syncer_dir = AppConfig::ssd_syncer_dir(ssd_path);
    if !syncer_dir.exists() {
        std::fs::create_dir_all(&syncer_dir)?;
    }

    let ignore = IgnoreMatcher::new(&config.ignore.patterns);

    if dry_run {
        println!("=== DRY RUN (no changes will be made) ===");
        println!();
    }

    let mut total_actions = 0;

    for mapping in &mappings {
        let label = mapping.name.as_deref().unwrap_or(&mapping.ssd);
        println!("━━━ Syncing: {} ↔ {} ━━━", mapping.local, label);

        let local_path = Path::new(&mapping.local);
        if !local_path.exists() {
            println!("  ⚠ Local path does not exist, skipping: {}", mapping.local);
            continue;
        }

        match sync_engine::sync_one_mapping(
            local_path,
            ssd_path,
            &mapping.ssd,
            &config.machine.name,
            &ignore,
            &config.conflict.strategy,
            dry_run,
            verbose,
        ) {
            Ok((_plan, result)) => {
                print_sync_result(&result);
                total_actions += result.total_actions();

                if !result.errors.is_empty() {
                    println!("  Errors:");
                    for err in &result.errors {
                        println!("    - {}", err);
                    }
                }
            }
            Err(e) => {
                println!("  Error syncing '{}': {}", mapping.ssd, e);
            }
        }

        println!();
    }

    // Append to sync log
    if !dry_run && total_actions > 0 {
        append_sync_log(ssd_path, &config.machine.name, total_actions)?;
    }

    if total_actions == 0 {
        println!("Everything is in sync!");
    }

    // 显示总耗时
    let elapsed = start_time.elapsed();
    let secs = elapsed.as_secs();
    if secs >= 60 {
        println!("Total time: {}m {:.1}s", secs / 60, elapsed.as_secs_f64() % 60.0);
    } else {
        println!("Total time: {:.1}s", elapsed.as_secs_f64());
    }

    Ok(())
}

fn cmd_status(name: Option<&str>) -> Result<()> {
    let config = AppConfig::load()?;
    let (ssd_mount_str, mappings) = resolve_mappings(name, &config)?;
    let ssd_path = Path::new(&ssd_mount_str);

    if !ssd_path.exists() {
        anyhow::bail!("SSD mount point does not exist: {}", ssd_mount_str);
    }

    let ignore = IgnoreMatcher::new(&config.ignore.patterns);

    for mapping in &mappings {
        let label = mapping.name.as_deref().unwrap_or(&mapping.ssd);
        println!("━━━ Status: {} ↔ {} ━━━", mapping.local, label);

        let local_path = Path::new(&mapping.local);
        if !local_path.exists() {
            println!("  ⚠ Local path does not exist: {}", mapping.local);
            continue;
        }

        let ssd_folder = ssd_path.join(&mapping.ssd);
        if !ssd_folder.exists() {
            println!("  SSD folder does not exist yet (will be created on first sync)");
            println!("  Local files will be copied to SSD");
            continue;
        }

        let snapshot_dir =
            AppConfig::ssd_snapshots_dir(ssd_path, &config.machine.name);
        let snapshot_file = snapshot_dir.join(Snapshot::snapshot_filename(&mapping.ssd));
        let base = Snapshot::load_or_empty(&snapshot_file, &mapping.ssd, &config.machine.name)?;

        let (local_snap, ssd_snap) = scanner::scan_pair(
            local_path,
            &ssd_folder,
            &mapping.ssd,
            &config.machine.name,
            &ignore,
            Some(&base),
            Some(&base),
        )?;

        let local_changes = diff::compute_changes(&base, &local_snap);
        let ssd_changes = diff::compute_changes(&base, &ssd_snap);

        let plan = diff::build_sync_plan(&local_changes, &ssd_changes);

        if plan.actions.is_empty() {
            println!("  In sync ✓");
        } else {
            let mut copy_to_ssd = 0;
            let mut copy_to_local = 0;
            let mut del_ssd = 0;
            let mut del_local = 0;
            let mut conflicts = 0;

            for a in &plan.actions {
                match &a.action {
                    SyncAction::CopyToSsd => copy_to_ssd += 1,
                    SyncAction::CopyToLocal => copy_to_local += 1,
                    SyncAction::DeleteFromSsd => del_ssd += 1,
                    SyncAction::DeleteFromLocal => del_local += 1,
                    SyncAction::Conflict(_) => conflicts += 1,
                }
            }

            if copy_to_ssd > 0 {
                println!("  → {} file(s) to copy to SSD", copy_to_ssd);
            }
            if copy_to_local > 0 {
                println!("  ← {} file(s) to copy to local", copy_to_local);
            }
            if del_ssd > 0 {
                println!("  ✕ {} file(s) to delete from SSD", del_ssd);
            }
            if del_local > 0 {
                println!("  ✕ {} file(s) to delete from local", del_local);
            }
            if conflicts > 0 {
                println!("  ⚠ {} conflict(s)", conflicts);
            }
        }

        println!();
    }

    Ok(())
}

fn cmd_diff(name: Option<&str>) -> Result<()> {
    let config = AppConfig::load()?;
    let (ssd_mount_str, mappings) = resolve_mappings(name, &config)?;
    let ssd_path = Path::new(&ssd_mount_str);

    if !ssd_path.exists() {
        anyhow::bail!("SSD mount point does not exist: {}", ssd_mount_str);
    }

    let ignore = IgnoreMatcher::new(&config.ignore.patterns);

    for mapping in &mappings {
        let label = mapping.name.as_deref().unwrap_or(&mapping.ssd);
        println!("━━━ Diff: {} ↔ {} ━━━", mapping.local, label);

        let local_path = Path::new(&mapping.local);
        if !local_path.exists() {
            println!("  ⚠ Local path does not exist: {}", mapping.local);
            continue;
        }

        let ssd_folder = ssd_path.join(&mapping.ssd);
        if !ssd_folder.exists() {
            println!("  SSD folder does not exist yet");
            continue;
        }

        let snapshot_dir =
            AppConfig::ssd_snapshots_dir(ssd_path, &config.machine.name);
        let snapshot_file = snapshot_dir.join(Snapshot::snapshot_filename(&mapping.ssd));
        let base = Snapshot::load_or_empty(&snapshot_file, &mapping.ssd, &config.machine.name)?;

        let (local_snap, ssd_snap) = scanner::scan_pair(
            local_path,
            &ssd_folder,
            &mapping.ssd,
            &config.machine.name,
            &ignore,
            Some(&base),
            Some(&base),
        )?;

        let local_changes = diff::compute_changes(&base, &local_snap);
        let ssd_changes = diff::compute_changes(&base, &ssd_snap);

        let plan = diff::build_sync_plan(&local_changes, &ssd_changes);

        if plan.actions.is_empty() {
            println!("  No differences.");
        } else {
            for entry in &plan.actions {
                let symbol = match &entry.action {
                    SyncAction::CopyToSsd => "→ SSD  ",
                    SyncAction::CopyToLocal => "← LOCAL",
                    SyncAction::DeleteFromSsd => "✕ SSD  ",
                    SyncAction::DeleteFromLocal => "✕ LOCAL",
                    SyncAction::Conflict(_) => "⚠ CONFLICT",
                };
                println!("  {} {}", symbol, entry.path);
            }
        }

        println!();
    }

    Ok(())
}

fn cmd_log(name: Option<&str>, limit: usize) -> Result<()> {
    let config = AppConfig::load()?;
    let (ssd_mount_str, _mappings) = resolve_mappings(name, &config)?;
    let ssd_path = Path::new(&ssd_mount_str);
    let log_path = AppConfig::ssd_syncer_dir(ssd_path).join("sync.log");

    if !log_path.exists() {
        println!("No sync history found.");
        return Ok(());
    }

    let content = std::fs::read_to_string(&log_path)?;
    let lines: Vec<&str> = content.lines().collect();
    let start = if lines.len() > limit {
        lines.len() - limit
    } else {
        0
    };

    println!("Sync history (last {} entries):", limit);
    for line in &lines[start..] {
        println!("  {}", line);
    }

    Ok(())
}

fn print_sync_result(result: &sync_engine::SyncResult) {
    if result.total_files > 0 {
        println!("  Total files in sync folder: {}", result.total_files);
    }
    if result.total_actions() == 0 {
        println!("  No changes needed.");
        return;
    }

    if result.copied_to_ssd > 0 {
        println!("  → Copied to SSD: {} file(s)", result.copied_to_ssd);
    }
    if result.copied_to_local > 0 {
        println!("  ← Copied to local: {} file(s)", result.copied_to_local);
    }
    if result.deleted_from_ssd > 0 {
        println!("  ✕ Deleted from SSD: {} file(s)", result.deleted_from_ssd);
    }
    if result.deleted_from_local > 0 {
        println!(
            "  ✕ Deleted from local: {} file(s)",
            result.deleted_from_local
        );
    }
    if result.conflicts > 0 {
        println!("  ⚠ Conflicts handled: {}", result.conflicts);
    }
}

fn cmd_ignore_reset() -> Result<()> {
    let mut config = AppConfig::load()?;
    let old_count = config.ignore.patterns.len();
    config.ignore = config::IgnoreConfig::default();
    config.save()?;
    println!("Ignore patterns reset to defaults.");
    println!("  Before: {} patterns", old_count);
    println!("  After:  {} patterns", config.ignore.patterns.len());
    println!();
    println!("Current ignore patterns:");
    for p in &config.ignore.patterns {
        println!("  - {}", p);
    }
    Ok(())
}

fn cmd_ignore_list() -> Result<()> {
    let config = AppConfig::load()?;
    println!("Ignore patterns ({} total):", config.ignore.patterns.len());
    for p in &config.ignore.patterns {
        println!("  - {}", p);
    }
    Ok(())
}

fn cmd_ignore_add(patterns: &[String]) -> Result<()> {
    if patterns.is_empty() {
        anyhow::bail!("Please provide at least one pattern to add.");
    }
    let mut config = AppConfig::load()?;
    let mut added = Vec::new();
    let mut skipped = Vec::new();
    for p in patterns {
        if config.ignore.patterns.contains(p) {
            skipped.push(p.as_str());
        } else {
            config.ignore.patterns.push(p.clone());
            added.push(p.as_str());
        }
    }
    config.save()?;
    if !added.is_empty() {
        println!("Added {} pattern(s):", added.len());
        for p in &added {
            println!("  + {}", p);
        }
    }
    if !skipped.is_empty() {
        println!("Skipped {} (already exists):", skipped.len());
        for p in &skipped {
            println!("  ~ {}", p);
        }
    }
    println!("Total: {} patterns", config.ignore.patterns.len());
    Ok(())
}

fn cmd_ignore_remove(patterns: &[String]) -> Result<()> {
    if patterns.is_empty() {
        anyhow::bail!("Please provide at least one pattern to remove.");
    }
    let mut config = AppConfig::load()?;
    let before = config.ignore.patterns.len();
    let mut removed = Vec::new();
    let mut not_found = Vec::new();
    for p in patterns {
        if let Some(pos) = config.ignore.patterns.iter().position(|x| x == p) {
            config.ignore.patterns.remove(pos);
            removed.push(p.as_str());
        } else {
            not_found.push(p.as_str());
        }
    }
    config.save()?;
    if !removed.is_empty() {
        println!("Removed {} pattern(s):", removed.len());
        for p in &removed {
            println!("  - {}", p);
        }
    }
    if !not_found.is_empty() {
        println!("Not found {} (skipped):", not_found.len());
        for p in &not_found {
            println!("  ~ {}", p);
        }
    }
    println!("Total: {} patterns (was {})", config.ignore.patterns.len(), before);
    Ok(())
}

fn append_sync_log(ssd_mount: &Path, machine: &str, actions: usize) -> Result<()> {
    let log_path = AppConfig::ssd_syncer_dir(ssd_mount).join("sync.log");
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    let entry = format!("[{}] machine={} actions={}\n", timestamp, machine, actions);

    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    file.write_all(entry.as_bytes())?;

    Ok(())
}

