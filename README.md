# SVMM

星露谷物语（Stardew Valley）模组管理器 — 基于 Tauri 2 + Vue 3，支持 Windows 与 macOS。

## 环境要求

- **Node.js** 18+（建议 LTS）
- **Rust** stable（仓库根目录的 `rust-toolchain.toml` 会选用当前系统的 stable 工具链）

### Windows

- 目标三元组：`x86_64-pc-windows-msvc`
- **Visual Studio Build Tools**（含「使用 C++ 的桌面开发」工作负载）
- WebView2（Windows 10/11 通常已预装）

> 请使用 MSVC 工具链。GNU（`x86_64-pc-windows-gnu`）可能因缺少 `dlltool` 等工具而无法链接。
> Windows 上 `rustup` 的默认 host 即为 MSVC。

### macOS

- Xcode Command Line Tools（`xcode-select --install`）
- 系统自带的 WebKit，无需额外安装 WebView

## 安装与运行

```bash
npm install
npm run tauri dev
```

仅构建前端：

```bash
npm run build
```

仅检查 Rust：

```bash
cd src-tauri
cargo check
```

打包安装包：

```bash
npm run tauri build
```

Windows 需要本机具备 NSIS/WiX；macOS 会生成 `.app` / `.dmg`。

## 数据与日志

- Windows：`%APPDATA%\svmm\`
- macOS：`~/Library/Application Support/svmm/`

- 设置：`settings.json`
- 配置档：`profiles\`
- 日志：`logs\svmm.log`（设置页可「打开日志目录」）

Nexus API 密钥保存在应用数据目录的 `secrets` 下，不会写入设置文件，也不会写入系统钥匙串。

## 游戏目录

自动探测顺序：

- Windows：Steam（Program Files (x86)）→ Steam（Program Files）→ Xbox → GOG
- macOS：Steam（`~/Library/Application Support/Steam/.../Contents/MacOS`）→ GOG（`/Applications/Stardew Valley.app/Contents/MacOS`）

macOS 上也可手动选择游戏文件夹或 `.app`，保存时会解析到 `Contents/MacOS`。SMAPI 启动文件为 `StardewModdingAPI`（无扩展名），不是 `.exe`。

## 许可

见仓库内许可证文件（若有）。
