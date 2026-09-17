# SVMM

星露谷物语（Stardew Valley）模组管理器 — 基于 Tauri 2 + Vue 3，面向 Windows。

## 环境要求

- **Node.js** 18+（建议 LTS）
- **Rust** stable，且目标三元组为 **MSVC**：
  - `x86_64-pc-windows-msvc`
- **Visual Studio Build Tools**（含「使用 C++ 的桌面开发」工作负载）
- WebView2（Windows 10/11 通常已预装）

> 本项目在 Windows 上请使用 MSVC 工具链。GNU（`x86_64-pc-windows-gnu`）可能因缺少 `dlltool` 等工具而无法链接。
> 仓库根目录的 `rust-toolchain.toml` 会自动选用 `stable-x86_64-pc-windows-msvc`。

### 设置 MSVC 工具链

若 `rustup` 尚未安装该工具链：

```powershell
rustup toolchain install stable-x86_64-pc-windows-msvc
rustup target add x86_64-pc-windows-msvc
```

也可在未使用 `rust-toolchain.toml` 时显式指定：

```powershell
cargo +stable-x86_64-pc-windows-msvc check
```
## 安装与运行

```powershell
npm install
npm run tauri dev
```

仅构建前端：

```powershell
npm run build
```

仅检查 Rust（MSVC）：

```powershell
cd src-tauri
$env:CARGO_TARGET_DIR = "target-msvc"
cargo +stable-x86_64-pc-windows-msvc check
```

打包安装包（需本机具备完整 NSIS/WiX 等打包依赖）：

```powershell
npm run tauri build
```

## 数据与日志

应用数据目录：`%APPDATA%\svmm\`

- 设置：`settings.json`
- 配置档：`profiles\`
- 日志：`logs\svmm.log`（设置页可「打开日志目录」）

Nexus API 密钥保存在系统凭据存储中，不会写入设置文件。

## 许可

见仓库内许可证文件（若有）。
