spec: task
name: "强化 rewrite/parity 合同写作"
inherits: project
tags: [contract-quality, skills, templates, parity, phase-next]
---

## Intent

让 `agent-spec` 在 rewrite/parity 类任务上更容易写出真正绑定外部行为的合同。
本任务聚焦于 skill、模板和参考示例，把“未绑定的可观察行为”检查内建进 authoring 工作流。

## Decisions

- 本轮不新增 DSL 语法，只通过 skills、模板和参考 spec 提升写作质量
- `agent-spec-authoring` 必须新增 `Behavior Surface Checklist`
- `agent-spec-tool-first` 必须新增 `Unbound Observable Behavior` 审查步骤
- 仓库应新增一份 rewrite/parity 示例 task spec，演示行为矩阵写法
- checklist 默认覆盖 stdout/stderr、`--json`、`-o/--output`、cold start、cache miss、fallback、local/remote、partial failure

## Boundaries

### Allowed Changes
- skills/agent-spec-authoring/**
- skills/agent-spec-tool-first/**
- .claude/skills/**
- README.md
- docs/**
- specs/**

### Forbidden
- 不要在 skill 中重新定义一套与 parser 不兼容的 spec 语法
- 不要只改 README 而忽略 skill 主体
- 不要把 rewrite/parity 指南写成抽象原则而没有具体检查清单

## Completion Criteria

Scenario: authoring skill 包含行为面检查清单
  Test:
    Filter: test_authoring_skill_includes_behavior_surface_checklist
  Given 用户正在为 CLI 或 MCP 工具编写 task spec
  When 查看 `agent-spec-authoring` skill
  Then skill 明确要求检查 stdout/stderr、`--json`、`-o/--output`、fallback、cold start 和文件副作用
  And 把这些行为作为合同写作前的必查项

Scenario: tool-first skill 包含未绑定可观察行为审查步骤
  Test:
    Filter: test_tool_first_skill_mentions_unbound_observable_behavior_review_step
  Given 用户已经完成 `agent-spec parse` 和 `agent-spec lint`
  When 查看 `agent-spec-tool-first` skill
  Then skill 额外要求审查还有哪些 stdout、stderr、文件、网络和持久化行为未被场景绑定
  And 说明这一步特别适用于 rewrite/parity 任务

Scenario: 仓库提供 rewrite/parity 示例合同
  Test:
    Filter: test_rewrite_parity_example_spec_exists_and_covers_behavior_matrix
  Given 仓库提供参考 spec
  When 用户查看 rewrite/parity 示例 task spec
  Then 示例覆盖 command x output mode、local x remote、warm cache x cold start 等行为矩阵
  And 示例中的每个矩阵维度都绑定了显式测试选择器

Scenario: README 说明 rewrite/parity 合同的写法与普通功能合同不同
  Test:
    Filter: test_readme_documents_rewrite_parity_contract_authoring_guidance
  Given 用户阅读项目文档
  When 查找 rewrite 或 parity 相关说明
  Then README 提到这类任务需要先绑定可观察行为矩阵
  And 指向对应的 skill 或示例 spec

Scenario: skill 明确指出遗漏行为矩阵时合同不应交付
  Test:
    Filter: test_skill_guidance_rejects_parity_contracts_missing_behavior_matrix
  Given 某个 rewrite task spec 只描述主流程功能，没有覆盖 stdout、`--json`、fallback 或 cold start
  When 用户按照 skill 自检该合同
  Then skill 明确要求补齐这些未绑定的可观察行为
  And 不把该合同视为可直接交付给 agent

Scenario: skill 不会把普通功能合同误判为 parity 合同
  Test:
    Filter: test_skill_guidance_does_not_require_behavior_matrix_for_non_parity_tasks
  Given 某个普通增量功能 task spec 不以 rewrite、migration 或 parity 为目标
  When 用户按照 skill 自检该合同
  Then skill 不要求补齐 rewrite/parity 行为矩阵
  And 不把普通功能合同错误标记为缺失 parity 覆盖

## Out of Scope

- parser 或 AST 的语法扩展
- 新增 lint 规则本身
- AI verifier 对 rewrite/parity 合同的专门推理
