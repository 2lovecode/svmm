## ADDED Requirements

### Requirement: 方案组从本地库勾选模组
系统 SHALL 允许用户创建方案组，并把本地库中的模组加入或移出该组。方案组保存的是 UniqueID 名单，而不是游戏 `Mods/` 里的启用状态。

#### Scenario: 把本地模组加入方案组
- **WHEN** 用户把一个存在于本地库的模组加入某方案组
- **THEN** 该方案组的名单包含该模组的 UniqueID，且游戏 `Mods/` 不变

#### Scenario: 不能加入不在本地库的模组
- **WHEN** 用户试图把一个不在本地库的 UniqueID 加入方案组
- **THEN** 系统拒绝该操作，方案组名单不变

#### Scenario: 方案组不绑定版本
- **WHEN** 本地库把某 UniqueID 的当前包换成另一文件版本
- **THEN** 每个包含该 UniqueID 的方案组名单不变，展示版本等于本地库的当前版本

### Requirement: 默认方案组与首页
系统 MUST 始终存在一个不可删除的默认方案组。首页 MUST 展示默认方案组中的模组，而不是直接把游戏 `Mods/` 当作名单来源。

#### Scenario: 首次启动
- **WHEN** 用户第一次打开应用且尚无方案组
- **THEN** 系统创建默认方案组，首页展示该组（可以为空）

#### Scenario: 尝试删除默认方案组
- **WHEN** 用户删除默认方案组
- **THEN** 系统拒绝删除，默认方案组仍在

#### Scenario: 打开首页
- **WHEN** 用户打开首页
- **THEN** 系统展示默认方案组的模组列表

### Requirement: 标明方案是否已应用到游戏目录
系统 SHALL 标明当前游戏 `Mods/` 是否与正在查看的方案组一致。一致指：除 SMAPI 自带模组外，`Mods/` 中的 UniqueID 集合与该组名单相同，且每个模组的版本与本地库当前版本相同。

#### Scenario: 尚未应用
- **WHEN** 用户修改了方案组名单，但还没有应用
- **THEN** 界面标明该方案组未应用到游戏目录

#### Scenario: 应用成功后
- **WHEN** 用户成功应用某方案组
- **THEN** 界面标明该方案组已应用到游戏目录

### Requirement: 应用时保留玩家数据再整组部署
系统 MUST 按以下顺序应用方案组：先把当前游戏 `Mods/` 中相对对应安装包清单有差异的文件收入按 UniqueID 共享的用户数据；再删除 `Mods/` 中非 SMAPI 自带模组；再把方案组中每个模组的当前包复制进 `Mods/`；最后把该 UniqueID 的用户数据覆盖回部署目录。用户数据 MUST 按 UniqueID 共享，MUST NOT 按方案组隔离。`config.json` 若与安装包不同，恢复时 MUST 以用户数据为准。

#### Scenario: 切换方案组后配置仍在
- **WHEN** 模组 A 的 `config.json` 已被玩家修改，用户应用一个不含 A 的方案组，然后再应用一个含 A 的方案组
- **THEN** 重新部署的 A 使用修改后的 `config.json`，而不是安装包里的原文件

#### Scenario: 两个方案组共享同一模组的配置
- **WHEN** 方案组甲和方案组乙都包含模组 A，且玩家在甲已应用时改过 A 的 `config.json`
- **THEN** 应用方案组乙之后，A 的 `config.json` 仍是该修改

#### Scenario: 应用会清空不在名单中的玩家模组
- **WHEN** 游戏 `Mods/` 中有一个不在目标方案组、且不是 SMAPI 自带模组的模组，用户应用该方案组
- **THEN** 该模组不再存在于游戏 `Mods/`

#### Scenario: 应用不会改动星露谷存档
- **WHEN** 用户应用任意方案组
- **THEN** 游戏存档目录中的存档文件保持不变

### Requirement: 保留 SMAPI 自带模组
应用方案组时，系统 MUST 保留 `ConsoleCommands`、`ErrorHandler`、`SaveBackup` 这三个 SMAPI 自带模组目录。系统 MUST NOT 把它们收入本地库或任何方案组名单。

#### Scenario: 应用后自带模组仍在
- **WHEN** 用户应用一个不含任何 SMAPI 自带模组的方案组
- **THEN** 游戏 `Mods/` 中这三个目录仍在，且与应用前一致

### Requirement: 应用失败时恢复游戏 Mods 目录
若应用在清空之后、部署完成之前失败，系统 MUST 把游戏 `Mods/` 恢复到应用开始前的内容。

#### Scenario: 部署中途失败
- **WHEN** 清空玩家模组之后，复制方案组模组失败
- **THEN** 游戏 `Mods/` 回到应用开始前的文件集合，且界面向用户报告失败
