# AGENTS.md

> **对本仓库所有 AI Coding Agent 强制生效**（Cursor、Claude Code、Codex、Gemini、OpenCode 等）。  
> 行为变更的唯一真相来源是 **OpenSpec**（`openspec/`）。  
> **本文件即应用级规则**（进 git）；`.cursor/` 仅本机，不提交。

## 规则分层

| 层 | 路径 | 进 git？ | 作用 |
|----|------|----------|------|
| 行为规格 | `openspec/specs/` | 是 | 产品行为长期真相 |
| **应用级规则** | **本文件 `AGENTS.md`** | **是** | 所有 Agent 共用的强制流程 |
| Claude Code 入口 | `CLAUDE.md` | 是 | 薄指针 → 本文件 |
| OpenSpec CLI 上下文 | `openspec/config.yaml` | 是 | propose/apply 时的项目上下文 |
| Cursor 本地注入 | `.cursor/rules/`（可选） | **否** | 本机 alwaysApply 压缩版，可有可无 |

Cursor 会读根目录 `AGENTS.md`；不必也不应把 `.cursor` 提交进仓库。

<!-- BEGIN: openspec-superpowers-rules -->
## OpenSpec × Superpowers（应用级规则）

> 行为变更的唯一真相来源是 **OpenSpec**（`openspec/`）。本段由 `install-openspec-agent-rules` 注入；改政策只改此处或整文件对齐。

### 强制规则（MUST）

1. **改行为 / 加能力之前先走 OpenSpec**  
   - 新工具、新功能、改契约、改对外可见行为 → 必须先有 OpenSpec change（`/opsx-propose` 或等价 skill），再实现，再 `/opsx-archive`。  
   - 实现必须对齐该 change 下的 `proposal` / `specs` / `tasks`（及所用 schema 要求的其他产物）。

2. **`openspec/specs/` 是长期行为规格**  
   - 不要另起平行的 PRD/story 体系抢「真相来源」。  
   - 不要把规格只写在聊天里就开写代码。

3. **小改可豁免（SHOULD 仍说明理由）**  
   仅当满足全部条件时可直接改代码、不开 change：  
   - bugfix / typo / 文案 / 纯重构且 **不改变可观察行为**  
   - 测试补强 / lint / 依赖小版本且 **无契约变化**  
   - 配置值微调且 **不引入新能力**

4. **高风险实现叠加 Superpowers**  
   加密、网络、数据损坏面大、复杂多文件：优先 `--schema superpowers-bridge`，执行期遵循 TDD / 计划 / 审查 skills。

5. **禁止**  
   - 忽略已有 `openspec/changes/<name>/` 直接大改  
   - 重新引入 BMAD（`_bmad`、bmad-* skills）  
   - 在 `docs/superpowers/` 另建长期规格替代 OpenSpec

### 开工检查清单

在动手改代码前，Agent MUST 自问：

- [ ] 这是行为变更还是豁免类小改？  
- [ ] 若是行为变更：是否已有 active change？没有则先 propose。  
- [ ] 是否读过相关 `openspec/specs/<capability>/spec.md`？  
- [ ] 实现范围是否锁在 tasks 内？

可用 CLI：

```bash
openspec list
openspec list --specs
openspec status --change <name>
openspec validate --specs
```

### 工具入口（同一套流程，不同拼写）

| Agent | 提议 | 实现 | 归档 |
|-------|------|------|------|
| Cursor | `/opsx-propose` 或 skill `openspec-propose` | `/opsx-apply` | `/opsx-archive` |
| Claude Code | `/opsx:propose` | `/opsx:apply` | `/opsx:archive` |
| 通用（读本文件） | 按 `.agents/skills/openspec-*` 或说明「按 OpenSpec propose→apply→archive」 | 同左 | 同左 |

Schema：

- 默认：`spec-driven`（见 `openspec/config.yaml`）  
- 高风险：`superpowers-bridge`  
  `openspec new change <name> --schema superpowers-bridge`

### OpenSpec × Superpowers 分工

- **OpenSpec** = 改什么（delta specs / archive）  
- **Superpowers** = 怎么做（brainstorming、TDD、writing-plans、subagent-driven-development）  
- 口头 brainstorm 收敛后 → 再 OpenSpec propose；**不要**把长期规格写进 `docs/superpowers/`
<!-- END: openspec-superpowers-rules -->

## 项目上下文

- 产品：SVMM — 星露谷物语模组管理器，管理本机模组并浏览 Nexus 模组。  
- 技术栈：Tauri 2、Vue 3、Rust  
- 新能力常见路径：前端在 src/，后端在 src-tauri/src/  
- Living specs 地图见下表；OpenSpec 指令以 `openspec/` 与各工具 `openspec-*` skills/commands 为准

## Living specs

| Capability | 路径 |
|------------|------|
| （在 openspec/specs/ 落地后填写） | `openspec/specs/<capability>/` |

Backlog 清单可参考 `ROADMAP.md`（不是行为规格）。

## 已安装

- OpenSpec CLI + `openspec/`  
- Claude Code：`.claude/commands/opsx/`、`.claude/skills/openspec-*`  
- 通用：`.agents/skills/openspec-*`、`.agents/skills/install-openspec-agent-rules/`  
- Bridge：`openspec/schemas/superpowers-bridge/`（若已安装）  
- **Cursor 本地**（不进 git）：`openspec update` 生成 `/opsx-*`；Superpowers 用 `/add-plugin superpowers`；可选本机 `.cursor/rules/openspec.mdc`（与本文件同政策）
