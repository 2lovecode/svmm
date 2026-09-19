# SVMM

星露谷物语模组管理器。用本地库保管模组，用方案决定这次玩哪些，只有点击「应用」才会写入游戏的 `Mods` 目录。

技术栈：Tauri 2、Vue 3、Rust。目前按 Windows 与 macOS 使用。

## 怎么用

1. 在设置里填好游戏目录，并安装 SMAPI。Nexus 浏览需要个人 API 密钥。
2. 在「浏览」里把模组入库，或把已有压缩包拖进首页。入库只进本地库，不会改游戏目录。
3. 把模组加入方案。加入本身也不会部署。
4. 在首页点「应用」，游戏 `Mods` 才会换成这个方案。已经应用过的方案不再显示「应用」。
5. 点「启动」。Steam 安装会从 Steam 启动并加载 SMAPI；其他安装直接启动 SMAPI。

首页可以切换正在查看的方案，切换不会自动应用。

## 三层

| | 是什么 | 写到哪里 |
| --- | --- | --- |
| 浏览 | Nexus 目录。可按名称或模组 ID 查询 | 不写游戏目录 |
| 本地库 | 每个 UniqueID 只保留一份当前包 | 应用数据目录里的 `library` |
| 方案 | 一份 UniqueID 名单。默认方案不能删除，显示名为「默认方案」 | 应用后才复制到游戏 `Mods` |

SMAPI 自带的 `ConsoleCommands`、`ErrorHandler`、`SaveBackup` 不会进入本地库。

从本地库删除模组时，所有方案里的这一项也会去掉。玩家改过的配置会留下。游戏目录里的副本要再点一次「应用」才会消失。删过的模组不会在下次刷新时从游戏目录自动导回来。

## Nexus

密钥只存在本机应用数据目录的 `secrets` 里，不会写进设置文件。

有 Nexus Premium 时，入库和更新会直接下载。没有 Premium 时，会打开 Nexus 文件页；在网页上点下载后，Nexus 通过 `nxm://` 把文件交回本应用，再写入本地库。

## 环境

- Node.js 18 或更新的 LTS
- Rust stable。仓库里的 `rust-toolchain.toml` 使用本机 stable

Windows 请用 MSVC 工具链（`x86_64-pc-windows-msvc`），并安装带「使用 C++ 的桌面开发」的 Visual Studio Build Tools。WebView2 在 Windows 10/11 上通常已经有了。GNU 工具链可能因为缺少 `dlltool` 而链接失败。

macOS 需要 Xcode Command Line Tools（`xcode-select --install`）。

## 开发

```bash
npm install
npm run tauri dev
```

打包：

```bash
npm run tauri build
```

只检查后端：

```bash
cd src-tauri
cargo check
```

## 数据目录

Windows 上 `directories` 的配置目录是 `%APPDATA%\svmm\config\`。macOS 上是 `~/Library/Application Support/svmm/`。

| 路径 | 内容 |
| --- | --- |
| `library/` | 本地库包 |
| `userdata/` | 按 UniqueID 保存的玩家配置 |
| `profiles/` | 方案 |
| `secrets/nexus_api_key` | Nexus API 密钥 |
| `logs/svmm.log` | 日志。设置页可以打开这个目录 |

游戏目录可在设置里填写，也可自动探测 Steam、GOG，以及 Windows 上的 Xbox 安装。macOS 上 SMAPI 可执行文件没有 `.exe` 后缀。
