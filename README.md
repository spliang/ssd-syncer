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
- **Configurable ignore**: `.DS_Store`, `Thumbs.db`, `desktop.ini` ignored by default, supports glob patterns
- **Dry run mode**: preview all changes before applying
- **Sync history log**: track when and what was synced

### Installation

#### Build from Source

Requires [Rust toolchain](https://rustup.rs/).

```bash
cd rust-cli
cargo build --release
```

The binary will be at `rust-cli/target/release/ssd-syncer` (~3MB).

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

#### 2. Add folder mappings

```bash
# On Mac
ssd-syncer add --local /Users/summer/share/abc --ssd share/abc

# On Windows
ssd-syncer add --local D:/share/abc --ssd share/abc

# On Ubuntu
ssd-syncer add --local /home/summer/share/abc --ssd share/abc
```

#### 3. Plug in SSD and sync

```bash
# On Mac (SSD mounted at /Volumes/MySSD)
ssd-syncer sync /Volumes/MySSD

# On Windows (SSD mounted at E:)
ssd-syncer sync E:

# On Ubuntu (SSD mounted at /mnt/ssd)
ssd-syncer sync /mnt/ssd
```

#### 4. Preview before syncing

```bash
# Dry run — see what would happen, no changes applied
ssd-syncer sync /Volumes/MySSD --dry-run

# Quick status summary
ssd-syncer status /Volumes/MySSD

# Detailed file-by-file diff
ssd-syncer diff /Volumes/MySSD
```

### CLI Reference

| Command | Description |
|---------|-------------|
| `ssd-syncer init --name <NAME>` | Initialize local config with machine name |
| `ssd-syncer add --local <PATH> --ssd <PATH>` | Add a sync folder mapping |
| `ssd-syncer remove --ssd <PATH>` | Remove a sync folder mapping |
| `ssd-syncer list` | List all configured mappings |
| `ssd-syncer sync <SSD_MOUNT>` | Sync all folders with SSD |
| `ssd-syncer sync <SSD_MOUNT> --dry-run` | Preview sync without applying |
| `ssd-syncer status <SSD_MOUNT>` | Show pending changes summary |
| `ssd-syncer diff <SSD_MOUNT>` | Show detailed file differences |
| `ssd-syncer log <SSD_MOUNT>` | Show sync history |

### Configuration

Local config is stored at `~/.ssd-syncer/config.toml`:

```toml
[machine]
name = "macbook-pro"

[[sync]]
local = "/Users/summer/share/abc"
ssd = "share/abc"

[[sync]]
local = "/Users/summer/share/xyz"
ssd = "share/xyz"

[ignore]
patterns = [".DS_Store", "Thumbs.db", "desktop.ini", ".ssd-syncer"]

[conflict]
strategy = "both"  # both / local-wins / ssd-wins / newer-wins / ask
```

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
- **可配置忽略规则**：默认忽略 `.DS_Store`、`Thumbs.db`、`desktop.ini`，支持 glob 模式
- **Dry Run 模式**：预览所有变更，确认后再执行
- **同步历史日志**：记录每次同步的时间和操作数

### 安装

#### 从源码编译

需要安装 [Rust 工具链](https://rustup.rs/)。

```bash
cd rust-cli
cargo build --release
```

编译产物位于 `rust-cli/target/release/ssd-syncer`（约 3MB）。

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

#### 2. 添加同步目录映射

```bash
# Mac 上
ssd-syncer add --local /Users/summer/share/abc --ssd share/abc

# Windows 上
ssd-syncer add --local D:/share/abc --ssd share/abc

# Ubuntu 上
ssd-syncer add --local /home/summer/share/abc --ssd share/abc
```

#### 3. 插入 SSD 执行同步

```bash
# Mac 上（SSD 挂载在 /Volumes/MySSD）
ssd-syncer sync /Volumes/MySSD

# Windows 上（SSD 挂载在 E:）
ssd-syncer sync E:

# Ubuntu 上（SSD 挂载在 /mnt/ssd）
ssd-syncer sync /mnt/ssd
```

#### 4. 同步前预览

```bash
# Dry Run —— 只看不改
ssd-syncer sync /Volumes/MySSD --dry-run

# 快速查看变更摘要
ssd-syncer status /Volumes/MySSD

# 查看逐文件差异
ssd-syncer diff /Volumes/MySSD
```

### 命令参考

| 命令 | 说明 |
|------|------|
| `ssd-syncer init --name <名称>` | 初始化本机配置 |
| `ssd-syncer add --local <路径> --ssd <路径>` | 添加同步目录映射 |
| `ssd-syncer remove --ssd <路径>` | 移除同步目录映射 |
| `ssd-syncer list` | 列出所有已配置的映射 |
| `ssd-syncer sync <SSD挂载点>` | 执行同步 |
| `ssd-syncer sync <SSD挂载点> --dry-run` | 预览同步（不执行） |
| `ssd-syncer status <SSD挂载点>` | 查看待同步变更摘要 |
| `ssd-syncer diff <SSD挂载点>` | 查看详细文件差异 |
| `ssd-syncer log <SSD挂载点>` | 查看同步历史 |

### 配置文件

本地配置保存在 `~/.ssd-syncer/config.toml`：

```toml
[machine]
name = "macbook-pro"

[[sync]]
local = "/Users/summer/share/abc"
ssd = "share/abc"

[[sync]]
local = "/Users/summer/share/xyz"
ssd = "share/xyz"

[ignore]
patterns = [".DS_Store", "Thumbs.db", "desktop.ini", ".ssd-syncer"]

[conflict]
strategy = "both"  # both / local-wins / ssd-wins / newer-wins / ask
```

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
