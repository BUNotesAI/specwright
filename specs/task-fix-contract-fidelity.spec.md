spec: task
name: "修正 Contract 保真度"
inherits: project
tags: [bootstrap, contract, phase0]
---

## Intent

在继续扩展 review flow 之前，
先修正 `specwright` 的合同面失真问题，让 Agent 读取到的 Task Contract 更接近真实规格。

## Decisions

- 本轮只补最小 Phase 0：祖先 `Constraints` 与 `Decisions` 的继承
- `TaskContract` 应区分 `Must`、`Must Not` 与 `Decisions`
- 默认文本 `contract` 输出应保留 step table 与结构化 `测试:` selector

## Boundaries

### Allowed Changes
- src/spec_core/**
- src/spec_parser/**
- src/spec_gateway/**
- src/**
- specs/**

### Forbidden
- 不要把 `Must` 再次回填进 `Decisions`
- 不要把这轮范围膨胀成完整 `Boundaries` 继承
- 不要只修 JSON 输出而忽略默认文本 `contract`

## Completion Criteria

Scenario: 继承链保留项目级约束与已定决策
  Test:
    Filter: test_load_resolves_full_project_contract_from_spec_directory
  Given `project.spec` 声明了 Constraints 与 Decisions
  When task spec 通过默认继承链加载
  Then 计划阶段的 Task Contract 包含这些继承得到的规则与已定决策
  And 不要求用户手工提供额外搜索路径

Scenario: Task Contract 区分 Must 与 Decisions
  Test:
    Filter: test_task_contract_keeps_must_must_not_and_decisions_distinct
  Given 某个 task spec 同时声明 Must、Must Not 与已定决策
  When gateway 构造 Task Contract
  Then 输出中保留这三类不同语义
  And 不再把 Must 合并进 Decisions

Scenario: contract 输出保留结构化验收信息
  Test:
    Package: specwright
    Filter: test_contract_output_preserves_step_tables_and_test_selectors
  Given 某个 Completion Criteria 场景带有 step table 与结构化 `测试:` selector
  When CLI 渲染 `specwright contract`
  Then 默认输出保留这些结构化信息
  And Claude Code 的 tool-first 路径不再丢失关键验收上下文

## Out of Scope

- 完整 `Boundaries` 继承
- `specwright explain`
- run log 与真实 AI backend
