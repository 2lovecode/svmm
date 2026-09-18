## Why

模组现在直接住在游戏 `Mods/` 里，方案组只是一组启用/禁用名单。玩家无法在不污染游戏目录的情况下收藏模组，也无法用一套名单整组替换正在玩的模组。需要把「目录、库存、正在用的方案」拆成三层，并在替换时保住模组自己写在文件夹里的配置。

## What Changes

- **BREAKING**：下载、NXM、拖入 zip 只进入本地库，不再直接解压进游戏 `Mods/`。
- **BREAKING**：方案组从「UniqueID 启用集合 + 差异启用/禁用」改为「从本地库勾选的模组名单」。应用时清空游戏 `Mods/` 中的玩家模组，再部署该组全部模组。
- 新增 Nexus 目录：按分类、模组 ID、关键词查询；对照本地库展示未下载、已有、有更新。
- 新增本地库：包文件不进 `Mods/`；可按分类、UniqueID、关键词筛选；支持更新、降级与删除。每个 UniqueID 只保留一份当前包，不做多版本共存。更新或降级都是替换这一份；方案组只记 UniqueID，因此包含该模组的方案一起变为新包。若该模组当前已在游戏 `Mods/` 中，立即先收用户数据、再换部署目录、再盖回用户数据。删除时从所有方案组移除该模组。
- 应用前把当前 `Mods/` 里相对安装包有改动的文件（主要是 `config.json` 及模组自写数据）收进共享用户数据，部署后再盖回。用户数据按 UniqueID 共享，不按方案组隔离。
- 应用时保留 SMAPI 自带模组（`ConsoleCommands`、`ErrorHandler`、`SaveBackup`），不删、不纳入方案组。
- 首页改为默认方案组：展示该组模组，并标明当前 `Mods/` 是否已与该组一致。
- 不在本变更做「同一模组在不同方案组使用不同配置」。

## Capabilities

### New Capabilities

- `nexus-catalog`: 按分类、Nexus 模组 ID、关键词查询 Nexus 目录，并对照本地库给出未下载 / 已有 / 有更新。
- `mod-library`: 本地库存放下载包（不进游戏 `Mods/`），支持分类、UniqueID、关键词筛选，以及更新、降级与删除。换版本时方案组与已部署副本一起换成唯一的当前包。
- `profile-groups`: 方案组从本地库勾选模组；应用时保留用户数据并整组部署；首页即默认方案组。

### Modified Capabilities

- （无。`openspec/specs/` 尚无既有规格。）

## Impact

- 后端：`domain/mods` 安装目标从游戏 `Mods/` 改为应用数据下的 library；`domain/profiles` 的 diff/apply 从启用切换改为清目录 + 部署；新增 userdata 抽取与回写。
- 前端：首页、浏览页、方案页的数据源与操作变更；顶栏方案切换改为应用方案组。
- 存储：`应用数据/svmm/library/`、`应用数据/svmm/userdata/`；既有 `profiles/*.json` 的 `enabledModIds` 模型需迁移或视为不兼容并重建默认组。
- 游戏目录：应用会删除 `Mods/` 下非 SMAPI 自带模组。星露谷存档与 `WriteSaveData` 不在 `Mods/` 内，不受影响。
