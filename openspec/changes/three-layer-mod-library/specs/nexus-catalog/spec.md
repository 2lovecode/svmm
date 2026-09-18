## ADDED Requirements

### Requirement: 按分类查询 Nexus 模组
系统 SHALL 允许用户按 Nexus 分类列出星露谷物语模组。

#### Scenario: 选择分类后列出该分类模组
- **WHEN** 用户在 Nexus 目录选择一个分类
- **THEN** 系统展示该分类下的模组列表

### Requirement: 按 Nexus 模组 ID 查询
系统 SHALL 允许用户用 Nexus 模组 ID 精确查询一个模组。

#### Scenario: ID 存在
- **WHEN** 用户提交一个有效的 Nexus 模组 ID
- **THEN** 系统展示该模组

#### Scenario: ID 不存在
- **WHEN** 用户提交的 Nexus 模组 ID 在 Nexus 上不存在
- **THEN** 系统展示未找到，且不写入本地库

### Requirement: 按关键词查询
系统 SHALL 允许用户用关键词搜索 Nexus 模组名称或摘要。

#### Scenario: 关键词命中
- **WHEN** 用户提交非空关键词
- **THEN** 系统展示名称或摘要与该关键词匹配的模组

### Requirement: 对照本地库展示安装状态
系统 MUST 用本地库（而不是游戏 `Mods/`）判断每条 Nexus 结果的状态，且状态只能是未下载、已有、有更新三者之一。

#### Scenario: 本地库没有该模组
- **WHEN** Nexus 结果对应的模组不在本地库中
- **THEN** 该条目状态为未下载

#### Scenario: 本地库版本与 Nexus 最新版一致
- **WHEN** 本地库已有该模组，且记录的版本不低于 Nexus 当前最新文件版本
- **THEN** 该条目状态为已有

#### Scenario: Nexus 有更新的文件版本
- **WHEN** 本地库已有该模组，且 Nexus 最新文件版本高于本地库记录的版本
- **THEN** 该条目状态为有更新

#### Scenario: 只存在于游戏 Mods 目录不算已有
- **WHEN** 某模组出现在游戏 `Mods/` 中，但不在本地库中
- **THEN** Nexus 目录仍将该条目标为未下载
