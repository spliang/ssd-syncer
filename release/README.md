# SSD-Syncer Pre-built Binaries / 预编译二进制文件

[English](#english) | [中文](#中文)

---

<a id="english"></a>

## English

### Available Binaries

| File | Platform | Architecture | Version |
|------|----------|-------------|---------|
| `ssd-syncer-macos` | macOS (Darwin) | x86_64 / Apple Silicon (Rosetta) | v0.1.0 |

### Usage

1. Download the binary for your platform
2. Make it executable (macOS/Linux):
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

### Note

- The macOS binary is built on macOS with `cargo build --release`
- Windows and Linux binaries will be added in future releases
- If the binary doesn't work on your system, you can build from source — see the [main README](../README.md)

---

<a id="中文"></a>

## 中文

### 可用的二进制文件

| 文件 | 平台 | 架构 | 版本 |
|------|------|------|------|
| `ssd-syncer-macos` | macOS (Darwin) | x86_64 / Apple Silicon (Rosetta) | v0.1.0 |

### 使用方法

1. 下载对应平台的二进制文件
2. 赋予执行权限（macOS/Linux）：
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

### 说明

- macOS 二进制文件通过 `cargo build --release` 在 macOS 上编译
- Windows 和 Linux 版本将在后续发布中添加
- 如果二进制文件无法在你的系统上运行，可以从源码编译 —— 参见[主 README](../README.md)
