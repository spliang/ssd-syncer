mod config;
mod diff;
mod ignore;
mod scanner;
mod snapshot;
mod sync_engine;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::Path;

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
        /// Relative path on SSD (e.g. "share/abc")
        #[arg(long)]
        ssd: String,
    },

    /// Remove a sync folder mapping
    Remove {
        /// Relative path on SSD to remove
        #[arg(long)]
        ssd: String,
    },

    /// List all configured sync mappings
    List,

    /// Sync all configured folders with SSD
    Sync {
        /// SSD mount point path
        ssd_mount: String,
        /// Dry run (preview only, no changes)
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },

    /// Show sync status (preview changes without applying)
    Status {
        /// SSD mount point path
        ssd_mount: String,
    },

    /// Show detailed diff between local and SSD
    Diff {
        /// SSD mount point path
        ssd_mount: String,
    },

    /// Show sync history log
    Log {
        /// SSD mount point path
        ssd_mount: String,
        /// Number of recent entries to show
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Init { name } => cmd_init(&name),
        Commands::Add { local, ssd } => cmd_add(&local, &ssd),
        Commands::Remove { ssd } => cmd_remove(&ssd),
        Commands::List => cmd_list(),
        Commands::Sync { ssd_mount, dry_run } => cmd_sync(&ssd_mount, dry_run),
        Commands::Status { ssd_mount } => cmd_status(&ssd_mount),
        Commands::Diff { ssd_mount } => cmd_diff(&ssd_mount),
        Commands::Log { ssd_mount, limit } => cmd_log(&ssd_mount, limit),
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

fn cmd_add(local: &str, ssd: &str) -> Result<()> {
    let mut config = AppConfig::load()?;

    // Check for duplicate
    if config.find_mapping_by_ssd(ssd).is_some() {
        anyhow::bail!("Mapping for SSD path '{}' already exists", ssd);
    }

    // Validate local path exists
    let local_path = Path::new(local);
    if !local_path.exists() {
        anyhow::bail!("Local path does not exist: {}", local);
    }

    config.sync.push(config::SyncMapping {
        local: local.to_string(),
        ssd: ssd.to_string(),
    });
    config.save()?;

    println!("Added sync mapping:");
    println!("  Local: {}", local);
    println!("  SSD:   {}", ssd);
    Ok(())
}

fn cmd_remove(ssd: &str) -> Result<()> {
    let mut config = AppConfig::load()?;
    let before = config.sync.len();
    config.sync.retain(|m| m.ssd != ssd);

    if config.sync.len() == before {
        anyhow::bail!("No mapping found for SSD path '{}'", ssd);
    }

    config.save()?;
    println!("Removed sync mapping for SSD path '{}'", ssd);
    Ok(())
}

fn cmd_list() -> Result<()> {
    let config = AppConfig::load()?;

    println!("Machine: {}", config.machine.name);
    println!("Conflict strategy: {:?}", config.conflict.strategy);
    println!();

    if config.sync.is_empty() {
        println!("No sync mappings configured. Use `ssd-syncer add` to add one.");
        return Ok(());
    }

    println!("Sync mappings:");
    for (i, mapping) in config.sync.iter().enumerate() {
        println!("  {}. Local: {}", i + 1, mapping.local);
        println!("     SSD:   {}", mapping.ssd);
    }

    println!();
    println!("Ignore patterns: {:?}", config.ignore.patterns);
    Ok(())
}

fn cmd_sync(ssd_mount: &str, dry_run: bool) -> Result<()> {
    let config = AppConfig::load()?;
    let ssd_path = Path::new(ssd_mount);

    if !ssd_path.exists() {
        anyhow::bail!("SSD mount point does not exist: {}", ssd_mount);
    }

    if config.sync.is_empty() {
        println!("No sync mappings configured. Use `ssd-syncer add` to add one.");
        return Ok(());
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

    for mapping in &config.sync {
        println!("━━━ Syncing: {} ↔ {} ━━━", mapping.local, mapping.ssd);

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

    Ok(())
}

fn cmd_status(ssd_mount: &str) -> Result<()> {
    let config = AppConfig::load()?;
    let ssd_path = Path::new(ssd_mount);

    if !ssd_path.exists() {
        anyhow::bail!("SSD mount point does not exist: {}", ssd_mount);
    }

    let ignore = IgnoreMatcher::new(&config.ignore.patterns);

    for mapping in &config.sync {
        println!("━━━ Status: {} ↔ {} ━━━", mapping.local, mapping.ssd);

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

fn cmd_diff(ssd_mount: &str) -> Result<()> {
    let config = AppConfig::load()?;
    let ssd_path = Path::new(ssd_mount);

    if !ssd_path.exists() {
        anyhow::bail!("SSD mount point does not exist: {}", ssd_mount);
    }

    let ignore = IgnoreMatcher::new(&config.ignore.patterns);

    for mapping in &config.sync {
        println!("━━━ Diff: {} ↔ {} ━━━", mapping.local, mapping.ssd);

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

fn cmd_log(ssd_mount: &str, limit: usize) -> Result<()> {
    let ssd_path = Path::new(ssd_mount);
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

