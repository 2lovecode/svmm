# CLAUDE.md

本仓库对 **Claude Code** 与其他 Agent 使用同一套项目规则。

<!-- BEGIN: openspec-superpowers-rules -->
**必读并遵守：** [@AGENTS.md](./AGENTS.md)（应用级规则，进 git）

要点（OpenSpec × Superpowers）：

1. 行为变更 / 新能力 → **OpenSpec**（`/opsx:propose` → `/opsx:apply` → `/opsx:archive`）
2. 长期规格只认 `openspec/specs/`
3. 高风险实现用 `superpowers-bridge` + Superpowers 执行纪律
4. 勿使用已移除的 BMAD 流程

OpenSpec skills 位于 `.claude/skills/openspec-*`；命令位于 `.claude/commands/opsx/`。
<!-- END: openspec-superpowers-rules -->
