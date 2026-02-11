# SSD-Syncer Pre-built Binaries / 预编译二进制文件

[English](#english) | [中文](#中文)

---

<a id="english"></a>

## English

### Available Binaries

| File | Platform | Architecture | Version |
|------|----------|-------------|---------|
| `ssd-syncer-macos` | macOS (Darwin) | x86_64 / Apple Silicon (Rosetta) | v0.1.0 |
| `ssd-syncer-windows.exe` | Windows | x86_64 | v0.1.0 |

### Usage

#### macOS

1. Download `ssd-syncer-macos`
2. Make it executable:
   ```bash
   chmod +x ssd-syncer-macos
   ```
3. Run it:
   ```bash
   ./ssd-syncer-macos --help
   ```
4. (Optional) Move to your PATH for global access:
   ```bash
   sudo mv ssd-syncer-macos /usr/local/bin/ssd-syncer
   ```

#### Windows

1. Download `ssd-syncer-windows.exe`
2. Run it directly:
   ```powershell
   .\ssd-syncer-windows.exe --help
   ```
3. (Optional) Rename and move to a directory in your PATH:
   ```powershell
   Move-Item ssd-syncer-windows.exe C:\Users\<YourUser>\bin\ssd-syncer.exe
   ```

### Note

- The macOS binary is built on macOS with `cargo build --release`
- The Windows binary is built with `cargo build --release` using the GNU toolchain (`x86_64-pc-windows-gnu`)
- Linux binary will be added in future releases
- If the binary doesn't work on your system, you can build from source — see the [main README](../README.md)

---

<a id="中文"></a>

## 中文

### 可用的二进制文件

| 文件 | 平台 | 架构 | 版本 |
|------|------|------|------|
| `ssd-syncer-macos` | macOS (Darwin) | x86_64 / Apple Silicon (Rosetta) | v0.1.0 |
| `ssd-syncer-windows.exe` | Windows | x86_64 | v0.1.0 |

### 使用方法

#### macOS

1. 下载 `ssd-syncer-macos`
2. 赋予执行权限：
   ```bash
   chmod +x ssd-syncer-macos
   ```
3. 运行：
   ```bash
   ./ssd-syncer-macos --help
   ```
4. （可选）移动到 PATH 目录以便全局使用：
   ```bash
   sudo mv ssd-syncer-macos /usr/local/bin/ssd-syncer
   ```

#### Windows

1. 下载 `ssd-syncer-windows.exe`
2. 直接运行：
   ```powershell
   .\ssd-syncer-windows.exe --help
   ```
3. （可选）重命名并移动到 PATH 目录：
   ```powershell
   Move-Item ssd-syncer-windows.exe C:\Users\<你的用户名>\bin\ssd-syncer.exe
   ```

### 说明

- macOS 二进制文件通过 `cargo build --release` 在 macOS 上编译
- Windows 二进制文件通过 `cargo build --release` 使用 GNU 工具链（`x86_64-pc-windows-gnu`）编译
- Linux 版本将在后续发布中添加
- 如果二进制文件无法在你的系统上运行，可以从源码编译 —— 参见[主 README](../README.md)
