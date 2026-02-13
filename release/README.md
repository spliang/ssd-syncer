# SSD-Syncer Pre-built Binaries / 预编译二进制文件

[English](#english) | [中文](#中文)

---

<a id="english"></a>

## English

### Available Files

| File | Platform | Description |
|------|----------|-------------|
| `ssd-syncer-macos` | macOS (Darwin) | Binary (x86_64 / Apple Silicon via Rosetta) |
| `ssd-syncer-windows.exe` | Windows | Binary (x86_64) |
| `install.sh` | macOS / Linux | Automated install script |
| `install.bat` | Windows | Automated install script |

### Quick Install

#### Windows

1. Download `ssd-syncer-windows.exe` and `install.bat` to the **same folder**
2. Double-click `install.bat` (or run it in CMD)
3. Open a **new** terminal window
4. Use `sync <command>` globally

#### macOS

1. Download `ssd-syncer-macos` and `install.sh` to the **same folder**
2. Run:
   ```bash
   chmod +x install.sh && ./install.sh
   ```
3. Open a new terminal (or run `source ~/.zshrc`)
4. Use `sync <command>` globally

#### Linux

1. Download `ssd-syncer-linux` and `install.sh` to the **same folder**
2. Run:
   ```bash
   chmod +x install.sh && ./install.sh
   ```
3. Open a new terminal (or run `source ~/.bashrc`)
4. Use `sync <command>` globally

### What the Installer Does

1. Copies the binary to `~/.ssd-syncer/bin/ssd-syncer` (or `%USERPROFILE%\.ssd-syncer\bin\ssd-syncer.exe` on Windows)
2. Creates a `sync` wrapper script in the same directory
3. Adds `~/.ssd-syncer/bin` to your PATH (shell profile on macOS/Linux, user PATH on Windows)

### Note

- The macOS binary is built on macOS with `cargo build --release`
- The Windows binary is built with `cargo build --release`
- Linux binary will be added in future releases
- If the binary doesn't work on your system, you can build from source — see the [main README](../README.md)

---

<a id="中文"></a>

## 中文

### 可用文件

| 文件 | 平台 | 说明 |
|------|------|------|
| `ssd-syncer-macos` | macOS (Darwin) | 二进制文件 (x86_64 / Apple Silicon via Rosetta) |
| `ssd-syncer-windows.exe` | Windows | 二进制文件 (x86_64) |
| `install.sh` | macOS / Linux | 自动安装脚本 |
| `install.bat` | Windows | 自动安装脚本 |

### 快速安装

#### Windows

1. 下载 `ssd-syncer-windows.exe` 和 `install.bat` 到**同一目录**
2. 双击运行 `install.bat`（或在 CMD 中执行）
3. 打开**新的**终端窗口
4. 即可全局使用 `sync <命令>`

#### macOS

1. 下载 `ssd-syncer-macos` 和 `install.sh` 到**同一目录**
2. 执行：
   ```bash
   chmod +x install.sh && ./install.sh
   ```
3. 打开新终端（或执行 `source ~/.zshrc`）
4. 即可全局使用 `sync <命令>`

#### Linux

1. 下载 `ssd-syncer-linux` 和 `install.sh` 到**同一目录**
2. 执行：
   ```bash
   chmod +x install.sh && ./install.sh
   ```
3. 打开新终端（或执行 `source ~/.bashrc`）
4. 即可全局使用 `sync <命令>`

### 安装脚本做了什么

1. 将二进制文件复制到 `~/.ssd-syncer/bin/ssd-syncer`（Windows 为 `%USERPROFILE%\.ssd-syncer\bin\ssd-syncer.exe`）
2. 在同一目录创建 `sync` 快捷命令
3. 将 `~/.ssd-syncer/bin` 添加到 PATH（macOS/Linux 修改 shell 配置文件，Windows 修改用户 PATH）

### 说明

- macOS 二进制文件通过 `cargo build --release` 在 macOS 上编译
- Windows 二进制文件通过 `cargo build --release` 编译
- Linux 版本将在后续发布中添加
- 如果二进制文件无法在你的系统上运行，可以从源码编译 —— 参见[主 README](../README.md)
