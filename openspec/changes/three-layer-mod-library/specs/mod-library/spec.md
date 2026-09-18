## ADDED Requirements

### Requirement: 安装只进入本地库
系统 MUST 把 Nexus 下载、NXM 安装和本地 zip 导入写入应用数据下的本地库，且 MUST NOT 把这些文件直接解压进游戏 `Mods/`。

#### Scenario: 从 Nexus 下载
- **WHEN** 用户把一条状态为未下载的 Nexus 模组加入本地库
- **THEN** 包文件出现在本地库，且游戏 `Mods/` 的内容不变

#### Scenario: 拖入 zip
- **WHEN** 用户把一个含 `manifest.json` 的模组 zip 导入
- **THEN** 系统将其写入本地库，且游戏 `Mods/` 的内容不变

### Requirement: 本地库可筛选
系统 SHALL 允许用户按 Nexus 分类、UniqueID 和关键词筛选本地库。没有分类信息的手动导入模组 MUST 仍可按 UniqueID 与关键词筛选。

#### Scenario: 按分类筛选
- **WHEN** 用户选择一个分类
- **THEN** 列表只保留该分类的本地模组

#### Scenario: 按 UniqueID 筛选
- **WHEN** 用户输入一个 UniqueID
- **THEN** 列表只保留该 UniqueID 的本地模组

#### Scenario: 按关键词筛选
- **WHEN** 用户输入非空关键词
- **THEN** 列表只保留名称、作者或 UniqueID 包含该关键词的本地模组

### Requirement: 记录安装包清单
系统 MUST 在模组进入本地库、更新或降级时，为当前包的每个文件记录相对路径与内容哈希。

#### Scenario: 新入库
- **WHEN** 一个模组首次写入本地库
- **THEN** 该模组拥有一份覆盖其全部包文件的清单

### Requirement: 更新或降级只保留一份当前包
系统 SHALL 允许用户把本地库中已有 Nexus 来源的模组替换为 Nexus 上的另一个文件版本，包括更新到最新版，以及降级到用户选定的历史文件版本。替换 MUST 用新包覆盖当前包并重写清单。系统 MUST NOT 在本地库中同时保留该模组的两个版本。方案组 MUST NOT 为该模组单独保存版本号。

#### Scenario: 更新到最新版
- **WHEN** 用户对一条状态为有更新的本地模组执行更新
- **THEN** 本地库中只剩新版本的包，清单与新包一致，旧包不再存在

#### Scenario: 降级到历史文件
- **WHEN** 用户从该模组的 Nexus 历史文件中选择一个旧版本并降级
- **THEN** 本地库中的当前包变为所选旧版本，且不保留降级前的包

#### Scenario: 方案组跟着唯一当前包
- **WHEN** 用户更新或降级一个已被至少一个方案组引用的模组
- **THEN** 这些方案组的名单仍包含该 UniqueID，且它们展示的版本等于本地库替换后的当前版本

### Requirement: 换版本时同步已部署副本
若该 UniqueID 当前存在于游戏 `Mods/`，更新或降级 MUST 在替换本地库之后，用同一套用户数据规则更新该部署目录：先收取相对旧清单有差异的文件，再用新包替换部署目录，再盖回用户数据。若部署副本替换失败，系统 MUST 把本地库和该部署目录都恢复到替换前。该 UniqueID 不在游戏 `Mods/` 中时，系统 MUST NOT 为了换版本而改写 `Mods/`。

#### Scenario: 已部署的模组一起换成新包
- **WHEN** 用户更新或降级一个当前已在游戏 `Mods/` 中的模组，且玩家改过它的 `config.json`
- **THEN** 游戏 `Mods/` 中该模组变为新包，且 `config.json` 仍是玩家修改后的内容

#### Scenario: 未部署则不改游戏目录
- **WHEN** 用户更新或降级一个不在游戏 `Mods/` 中的模组
- **THEN** 本地库变为新包，游戏 `Mods/` 的内容不变

#### Scenario: 同步部署失败则两边都还原
- **WHEN** 本地库已经换成新包，但替换游戏 `Mods/` 中的对应目录失败
- **THEN** 本地库和该部署目录都回到替换前的文件，且界面向用户报告失败

### Requirement: 删除本地库中的模组
系统 SHALL 允许用户删除本地库中的模组。删除 MUST 从所有方案组的名单中移除该 UniqueID。删除 MUST NOT 删除该 UniqueID 的共享用户数据。

#### Scenario: 删除已被方案组引用的模组
- **WHEN** 用户删除一个已加入至少一个方案组的本地模组
- **THEN** 该模组不再出现在本地库，且每个方案组的名单都不再包含它

#### Scenario: 删除后用户数据仍在
- **WHEN** 用户删除一个曾经产生过 `config.json` 改动的模组
- **THEN** 该 UniqueID 的共享用户数据目录仍保留
