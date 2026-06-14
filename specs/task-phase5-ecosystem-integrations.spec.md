spec: task
name: "Phase 5：Ecosystem Integrations"
inherits: project
tags: [roadmap, planned, phase5, ecosystem]
---

## Intent

让更多 Agent 工具和编排系统可以复用 `agent-spec`，
同时保持核心定位不变：CLI-first、tool-first、agent-agnostic。

## Decisions

- 新的工具集成优先通过 skill / rule / convention 文件交付
- 首批扩展目标是 Codex CLI、Cursor 与 Aider
- `lifecycle` 与 `guard` 的 JSON 输出继续作为编排接口
- `checkpoint` 能力保持可选，且按 VCS 能力渐进增强
- Entire / Symphony 这类深度集成放在技能模板和接口稳定之后

## Boundaries

### Allowed Changes
- .claude/**
- AGENTS.md
- README.md
- src/**
- src/spec_report/**
- specs/**

### Forbidden
- 不要为了单一 Agent 工具改变核心 CLI 契约
- 不要让生态集成抢在 explain / run log / org.spec 之前成为主工作面
- 不要让 checkpoint 成为所有用户的默认依赖

## Completion Criteria

Scenario: 提供更多 Agent 工具的集成模板
  Test:
    Package: agent-spec
    Filter: test_additional_agent_integration_templates_exist
  Given 仓库需要支持多种 Agent 工具
  When 用户查看集成模板目录
  Then 能找到 Claude Code 之外的模板
  And 它们继续遵循 `contract -> lifecycle -> guard` 主路径

Scenario: JSON 输出适合作为编排接口
  Test:
    Filter: test_report_json_exposes_contract_and_verification_summary_for_orchestrators
  Given 外部编排系统读取 lifecycle 或 guard 结果
  When 用户选择 JSON 输出
  Then 输出包含结构化 Contract 与 verification summary
  And 不要求外部系统解析人类文本

Scenario: checkpoint 能力保持可选
  Test:
    Package: agent-spec
    Filter: test_checkpoint_commands_are_optional_and_vcs_aware
  Given 当前仓库可能是 Git、jj 或无 VCS
  When 用户查看 checkpoint 能力
  Then 命令按 VCS 能力渐进增强
  And 不会把 checkpoint 强制注入默认 lifecycle

## Out of Scope

- 把 Entire API 变成硬依赖
- 把编排系统逻辑写进验证核心
