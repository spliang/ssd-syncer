# SSD-Syncer

[English](#english) | [中文](#中文)

---

<a id="english"></a>

## English

### What is SSD-Syncer?

SSD-Syncer is a cross-platform CLI tool that synchronizes folders between multiple computers (macOS, Windows, Ubuntu) using a physical SSD drive — **no network required**.

It works like a "sneakernet" sync: plug your SSD into one computer, sync, then plug it into another — all machines end up with identical folder contents.

### How It Works

```
MacOS ↔ SSD ↔ Windows
              ↔ Ubuntu
```

- The SSD acts as a **hub**. Each time you plug it into a computer, the CLI performs a **bidirectional sync** between the local folder and the SSD folder.
- Uses a **three-way merge** algorithm: compares the current local state and SSD state against the last-sync snapshot to determine what changed on each side.
- **Optimized scanning**: first compares file modification time + size (fast), then computes BLAKE3 hash only for changed files.

### Features

- **Cross-platform**: single Rust binary runs on macOS, Windows, and Linux
- **Bidirectional sync**: changes on either side are correctly merged
- **Conflict detection**: 5 resolution strategies (`both` / `local-wins` / `ssd-wins` / `newer-wins` / `ask`)
- **Fast scanning**: mtime+size pre-filtering, BLAKE3 hashing only when needed
- **Smart ignore**: common build/temp directories (`node_modules`, `__pycache__`, `target`, `.git`, `dist`, `build`, etc.) ignored by default, supports glob patterns
- **Dry run mode**: preview all changes before applying
- **Sync history log**: track when and what was synced

### Installation

#### Quick Install (Recommended)

Download the binary and install script for your platform from the [release](./release/) directory:

**Windows:**
1. Download `ssd-syncer-windows.exe` and `install.bat` to the same folder
2. Double-click `install.bat` (or run it in CMD)
3. Open a **new** terminal and use `sync <command>`

**macOS:**
1. Download `ssd-syncer-macos` and `install.sh` to the same folder
2. Run:
   ```bash
   chmod +x install.sh && ./install.sh
   ```
3. Open a new terminal (or `source ~/.zshrc`) and use `sync <command>`

**Linux:**
1. Download `ssd-syncer-linux` and `install.sh` to the same folder
2. Run:
   ```bash
   chmod +x install.sh && ./install.sh
   ```
3. Open a new terminal (or `source ~/.bashrc`) and use `sync <command>`

#### Build from Source

Requires [Rust toolchain](https://rustup.rs/).

```bash
cd cli
cargo build --release
```

The binary will be at `cli/target/release/ssd-syncer`.

#### Cross-compile

Use [cross](https://github.com/cross-rs/cross) for easy cross-compilation:

```bash
# For Linux
cross build --release --target x86_64-unknown-linux-gnu

# For Windows
cross build --release --target x86_64-pc-windows-gnu
```

### Quick Start

#### 1. Initialize on each computer

```bash
# On your Mac
ssd-syncer init --name "macbook-pro"

# On your Windows PC
ssd-syncer init --name "win-desktop"

# On your Ubuntu machine
ssd-syncer init --name "ubuntu-dev"
```

#### 2. Add folder mappings (with name and SSD path)

```bash
# On Mac — use absolute SSD path + alias name
ssd-syncer add --local /Users/summer/share/abc --ssd /Volumes/MySSD --name WORK

# On Windows
ssd-syncer add --local D:/share/abc --ssd E: --name WORK

# On Ubuntu
ssd-syncer add --local /home/summer/share/abc --ssd /mnt/ssd --name WORK
```

#### 3. Plug in SSD and sync

```bash
# Quick sync by mapping name (recommended!)
ssd-syncer sync WORK

# Or specify SSD mount point directly (classic usage)
ssd-syncer sync /Volumes/MySSD
```

#### 4. Preview before syncing

```bash
# Dry run — see what would happen, no changes applied
ssd-syncer sync WORK --dry-run

# Quick status summary
ssd-syncer status WORK

# Detailed file-by-file diff
ssd-syncer diff WORK
```

### CLI Reference

#### `init` — Initialize local config
```bash
ssd-syncer init --name "macbook-pro"
```

#### `add` — Add a sync folder mapping
```bash
# With absolute SSD path + alias name (recommended)
ssd-syncer add --local /Users/summer/Documents/work --ssd /Volumes/MySSD --name WORK

# With relative path (classic, requires ssd_mount when syncing)
ssd-syncer add --local /Users/summer/Documents/work --ssd WORK_SYNC
```

#### `remove` — Remove a sync folder mapping
```bash
ssd-syncer remove --ssd WORK_SYNC
```

#### `list` — List all configured mappings
```bash
ssd-syncer list
```

#### `set-ssd` — Set default SSD mount point
```bash
ssd-syncer set-ssd /Volumes/MySSD    # macOS
ssd-syncer set-ssd E:                # Windows
ssd-syncer set-ssd /mnt/ssd          # Linux
```

#### `sync` — Sync all folders with SSD
```bash
# By mapping name (recommended — no need to remember paths!)
ssd-syncer sync WORK

# By SSD mount point path (classic)
ssd-syncer sync /Volumes/MySSD

# Preview only (no changes applied)
ssd-syncer sync WORK --dry-run

# Verbose: show each file operation on a separate line
ssd-syncer sync WORK --verbose
ssd-syncer sync WORK -v
```

#### `status` — Show pending changes summary
```bash
ssd-syncer status WORK                # By mapping name
ssd-syncer status /Volumes/MySSD      # By path
```

#### `diff` — Show detailed file differences
```bash
ssd-syncer diff WORK                  # By mapping name
ssd-syncer diff /Volumes/MySSD        # By path
```

#### `log` — Show sync history
```bash
ssd-syncer log WORK                       # By mapping name
ssd-syncer log /Volumes/MySSD             # Last 20 entries (default)
ssd-syncer log /Volumes/MySSD --limit 50  # Last 50 entries
```

#### `ignore-reset` — Reset ignore patterns to defaults
```bash
ssd-syncer ignore-reset
```

#### `ignore-list` — List current ignore patterns
```bash
ssd-syncer ignore-list
```

#### `ignore-add` — Add ignore patterns
```bash
# Name pattern: ignore all directories/files named "logs" anywhere
ssd-syncer ignore-add "*.log" "logs" ".env" "coverage"

# Path pattern (contains /): ignore a specific folder only
ssd-syncer ignore-add "projects/myapp/tmp" "data/cache"
```

> **Pattern rules:**
>
> All patterns are matched against **relative paths within the sync folder root** (the local directory you configured with `ssd-syncer add --local`).
>
> - **Name pattern** (no `/`): matches any file/directory with that name at any level.
>   - Example: pattern `node_modules` ignores all directories named `node_modules` no matter how deeply nested.
> - **Path pattern** (contains `/`): matches only the exact relative path and everything under it.
>   - Example: pattern `projects/myapp/tmp` only ignores the folder at that specific relative path.
> - **Glob** (`*`, `?`): supported in both types, e.g. `*.log`, `*.pyc`.
>
> **Example to clarify “relative to sync folder root”:**
>
> Suppose your sync mapping is:
> ```
> local = "C:\Users\Summer\sync"   ← this is the sync folder root
> ssd   = "MY_SYNC"
> ```
> Then the pattern `projects/myapp/tmp` matches:
> - Local: `C:\Users\Summer\sync\projects\myapp\tmp\` and all files within
> - SSD:   `<SSD_MOUNT>/MY_SYNC/projects/myapp/tmp/` and all files within
>
> It does **NOT** mean a path on the SSD only — it applies equally to both sides.

#### `ignore-remove` — Remove ignore patterns
```bash
ssd-syncer ignore-remove "vendor" "dist"
```

### Configuration

Local config is stored at `~/.ssd-syncer/config.toml`:

```toml
[machine]
name = "macbook-pro"

[[sync]]
name = "WORK"
local = "/Users/summer/share/abc"
ssd = "/Volumes/MySSD"          # absolute path: use `sync WORK` directly!

[[sync]]
name = "PHOTOS"
local = "/Users/summer/share/xyz"
ssd = "/Volumes/MySSD"          # same SSD, different local folder

[ignore]
patterns = [
  ".DS_Store", "Thumbs.db", "desktop.ini", ".ssd-syncer",
  ".git", ".svn", ".hg",
  "__pycache__", ".venv", "venv", "node_modules",
  "target", "dist", "build", ".cache",
  ".idea", ".vs",
  # ... and more (use `ignore-reset` to see full list)
]

[conflict]
strategy = "both"  # both / local-wins / ssd-wins / newer-wins / ask
```

> **Tip**: If your config was created before v0.2.0, run `ssd-syncer ignore-reset` to update to the latest default ignore patterns.

### Conflict Resolution Strategies

| Strategy | Behavior |
|----------|----------|
| `both` (default) | Keep both versions, rename the conflicting file with `.conflict.<machine>.<timestamp>` suffix |
| `local-wins` | Local version always wins |
| `ssd-wins` | SSD version always wins |
| `newer-wins` | The file with the more recent modification time wins |
| `ask` | Interactive prompt (falls back to `both` in non-interactive mode) |

### SSD Directory Structure

```
<ssd-mount>/
├── .ssd-syncer/
│   ├── snapshots/
│   │   ├── macbook-pro/
│   │   │   └── share_abc.json
│   │   └── win-desktop/
│   │       └── share_abc.json
│   └── sync.log
└── share/
    └── abc/
        └── (your synced files)
```

### License

MIT

---

<a id="中文"></a>

## 中文

### 简介

SSD-Syncer 是一个跨平台（macOS / Windows / Ubuntu）的命令行工具，通过物理 SSD 硬盘在多台电脑之间同步指定文件夹的内容——**无需网络**。

使用方式类似"U盘中转"：将 SSD 插入一台电脑同步，再插入另一台电脑同步，最终所有机器上的文件夹内容完全一致。

### 工作原理

```
MacOS ↔ SSD ↔ Windows
              ↔ Ubuntu
```

- SSD 作为**中转站（Hub）**，每次插入一台电脑时，CLI 会执行本地文件夹与 SSD 文件夹之间的**双向同步**
- 使用**三方比对算法**：将当前本地状态和 SSD 状态分别与上次同步快照对比，判断各方的变更
- **扫描优化**：先比较文件修改时间 + 大小（极快），仅对有变化的文件计算 BLAKE3 哈希

### 特性

- **跨平台**：单个 Rust 二进制文件，支持 macOS、Windows、Linux
- **双向同步**：两端的变更都能正确合并
- **冲突检测**：5 种冲突解决策略（`both` / `local-wins` / `ssd-wins` / `newer-wins` / `ask`）
- **快速扫描**：mtime + size 预过滤，仅必要时计算 BLAKE3 哈希
- **智能忽略**：默认忽略常见编译/临时目录（`node_modules`、`__pycache__`、`target`、`.git`、`dist`、`build` 等），支持 glob 模式
- **Dry Run 模式**：预览所有变更，确认后再执行
- **同步历史日志**：记录每次同步的时间和操作数

### 安装

#### 快速安装（推荐）

从 [release](./release/) 目录下载对应平台的二进制文件和安装脚本：

**Windows：**
1. 下载 `ssd-syncer-windows.exe` 和 `install.bat` 到同一目录
2. 双击运行 `install.bat`（或在 CMD 中执行）
3. 打开**新的**终端窗口，即可使用 `sync <命令>`

**macOS：**
1. 下载 `ssd-syncer-macos` 和 `install.sh` 到同一目录
2. 执行：
   ```bash
   chmod +x install.sh && ./install.sh
   ```
3. 打开新终端（或 `source ~/.zshrc`）即可使用 `sync <命令>`

**Linux：**
1. 下载 `ssd-syncer-linux` 和 `install.sh` 到同一目录
2. 执行：
   ```bash
   chmod +x install.sh && ./install.sh
   ```
3. 打开新终端（或 `source ~/.bashrc`）即可使用 `sync <命令>`

#### 从源码编译

需要安装 [Rust 工具链](https://rustup.rs/)。

```bash
cd cli
cargo build --release
```

编译产物位于 `cli/target/release/ssd-syncer`。

#### 交叉编译

使用 [cross](https://github.com/cross-rs/cross) 工具可以轻松交叉编译：

```bash
# 编译 Linux 版本
cross build --release --target x86_64-unknown-linux-gnu

# 编译 Windows 版本
cross build --release --target x86_64-pc-windows-gnu
```

### 快速开始

#### 1. 在每台电脑上初始化

```bash
# Mac 上
ssd-syncer init --name "macbook-pro"

# Windows 上
ssd-syncer init --name "win-desktop"

# Ubuntu 上
ssd-syncer init --name "ubuntu-dev"
```

#### 2. 添加同步目录映射（指定 SSD 路径 + 别名）

```bash
# Mac 上 — 使用 SSD 绝对路径 + 别名
ssd-syncer add --local /Users/summer/share/abc --ssd /Volumes/MySSD --name WORK

# Windows 上
ssd-syncer add --local D:/share/abc --ssd E: --name WORK

# Ubuntu 上
ssd-syncer add --local /home/summer/share/abc --ssd /mnt/ssd --name WORK
```

#### 3. 插入 SSD 执行同步

```bash
# 通过别名快速同步（推荐！不用记路径）
ssd-syncer sync WORK

# 或直接指定 SSD 挂载点路径（传统用法）
ssd-syncer sync /Volumes/MySSD
```

#### 4. 同步前预览

```bash
# Dry Run —— 只看不改
ssd-syncer sync WORK --dry-run

# 快速查看变更摘要
ssd-syncer status WORK

# 查看逐文件差异
ssd-syncer diff WORK
```

### 命令参考

#### `init` — 初始化本机配置
```bash
ssd-syncer init --name "macbook-pro"
```

#### `add` — 添加同步目录映射
```bash
# 使用 SSD 绝对路径 + 别名（推荐）
ssd-syncer add --local /Users/summer/Documents/work --ssd /Volumes/MySSD --name WORK

# 使用相对路径（传统方式，同步时需手动指定挂载点）
ssd-syncer add --local /Users/summer/Documents/work --ssd WORK_SYNC
```

#### `remove` — 移除同步目录映射
```bash
ssd-syncer remove --ssd WORK_SYNC
```

#### `list` — 列出所有已配置的映射
```bash
ssd-syncer list
```

#### `set-ssd` — 设置默认 SSD 挂载点
```bash
ssd-syncer set-ssd /Volumes/MySSD    # macOS
ssd-syncer set-ssd E:                # Windows
ssd-syncer set-ssd /mnt/ssd          # Linux
```

#### `sync` — 执行同步
```bash
# 通过别名同步（推荐！不用记路径）
ssd-syncer sync WORK

# 通过 SSD 挂载点路径同步（传统用法）
ssd-syncer sync /Volumes/MySSD

# 仅预览（不实际执行）
ssd-syncer sync WORK --dry-run

# 详细模式：逐文件显示操作
ssd-syncer sync WORK --verbose
ssd-syncer sync WORK -v
```

#### `status` — 查看待同步变更摘要
```bash
ssd-syncer status WORK                # 通过别名
ssd-syncer status /Volumes/MySSD      # 通过路径
```

#### `diff` — 查看详细文件差异
```bash
ssd-syncer diff WORK                  # 通过别名
ssd-syncer diff /Volumes/MySSD        # 通过路径
```

#### `log` — 查看同步历史
```bash
ssd-syncer log WORK                       # 通过别名
ssd-syncer log /Volumes/MySSD             # 默认显示最近 20 条
ssd-syncer log /Volumes/MySSD --limit 50  # 显示最近 50 条
```

#### `ignore-reset` — 重置忽略规则为默认值
```bash
ssd-syncer ignore-reset
```

#### `ignore-list` — 查看当前忽略规则
```bash
ssd-syncer ignore-list
```

#### `ignore-add` — 添加忽略规则
```bash
# 名称模式：忽略所有叫这个名字的文件/目录
ssd-syncer ignore-add "*.log" "logs" ".env" "coverage"

# 路径模式（含 /）：只忽略特定路径的文件夹
ssd-syncer ignore-add "projects/myapp/tmp" "data/cache"
```

> **模式规则：**
>
> 所有模式都是相对于**同步文件夹根目录**（即 `ssd-syncer add --local` 配置的本地目录）进行匹配的。
>
> - **名称模式**（不含 `/`）：匹配任意层级下的同名文件/目录。
>   - 例如：模式 `node_modules` 会忽略所有叫 `node_modules` 的目录，无论嵌套多深。
> - **路径模式**（含 `/`）：只匹配特定相对路径及其下所有内容。
>   - 例如：模式 `projects/myapp/tmp` 只忽略该特定相对路径下的文件夹。
> - **通配符**（`*`、`?`）：两种模式均支持，例如 `*.log`、`*.pyc`。
>
> **举例说明“相对于同步文件夹根目录”：**
>
> 假设你的同步映射配置为：
> ```
> local = "C:\Users\Summer\sync"   ← 这就是同步文件夹根目录
> ssd   = "MY_SYNC"
> ```
> 那么忽略模式 `projects/myapp/tmp` 实际匹配的是：
> - 本地：`C:\Users\Summer\sync\projects\myapp\tmp\` 及其下所有文件
> - SSD： `<SSD挂载点>/MY_SYNC/projects/myapp/tmp/` 及其下所有文件
>
> 忽略规则对本地和 SSD 两侧**同时生效**，而不是只作用于某一侧。

#### `ignore-remove` — 移除忽略规则
```bash
ssd-syncer ignore-remove "vendor" "dist"
```

### 配置文件

本地配置保存在 `~/.ssd-syncer/config.toml`：

```toml
[machine]
name = "macbook-pro"

[[sync]]
name = "WORK"
local = "/Users/summer/share/abc"
ssd = "/Volumes/MySSD"          # 绝对路径：可直接 `sync WORK`！

[[sync]]
name = "PHOTOS"
local = "/Users/summer/share/xyz"
ssd = "/Volumes/MySSD"          # 同一个 SSD，不同的本地目录

[ignore]
patterns = [
  ".DS_Store", "Thumbs.db", "desktop.ini", ".ssd-syncer",
  ".git", ".svn", ".hg",
  "__pycache__", ".venv", "venv", "node_modules",
  "target", "dist", "build", ".cache",
  ".idea", ".vs",
  # ... 更多默认规则（运行 `ignore-reset` 查看完整列表）
]

[conflict]
strategy = "both"  # both / local-wins / ssd-wins / newer-wins / ask
```

> **提示**：如果你的配置是在 v0.2.0 之前创建的，运行 `ssd-syncer ignore-reset` 可以更新为最新的默认忽略规则。

### 冲突解决策略

| 策略 | 行为 |
|------|------|
| `both`（默认） | 保留双方版本，冲突文件添加 `.conflict.<机器名>.<时间戳>` 后缀 |
| `local-wins` | 始终以本地版本为准 |
| `ssd-wins` | 始终以 SSD 版本为准 |
| `newer-wins` | 以修改时间更新的版本为准 |
| `ask` | 交互式询问（非交互模式下退回到 `both`） |

### SSD 目录结构

```
<SSD挂载点>/
├── .ssd-syncer/
│   ├── snapshots/
│   │   ├── macbook-pro/
│   │   │   └── share_abc.json
│   │   └── win-desktop/
│   │       └── share_abc.json
│   └── sync.log
└── share/
    └── abc/
        └── （你的同步文件）
```

### 许可证

MIT
