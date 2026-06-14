spec: task
name: "Phase 0：Contract 保真度修正"
inherits: project
tags: [roadmap, planned, phase0, contract]
---

## Intent

在继续扩展 review 流程、run log 或 AI 能力之前，
先把 `agent-spec` 的合同面修到足够保真，避免后续功能建立在失真的 Task Contract 上。

## Decisions

- 最小 Phase 0 先补齐祖先 `Constraints` 与 `Decisions` 的继承
- `TaskContract` 应把 `Must`、`Must Not`、`Decisions` 区分为一等语义
- `contract` 的主输出应保留 Completion Criteria 里的 step table 与 test selector

## Boundaries

### Allowed Changes
- src/spec_core/**
- src/spec_parser/**
- src/spec_gateway/**
- src/**
- specs/**
- README.md

### Forbidden
- 不要继续把 `Must` 当成 `Decisions`
- 不要只修 JSON 输出而忽略默认文本 `contract` 输出
- 不要在保真度修正完成前优先实现 `stamp`、run log 或真实 AI backend

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
    Package: agent-spec
    Filter: test_contract_output_preserves_step_tables_and_test_selectors
  Given 某个 Completion Criteria 场景带有 step table 与结构化 `测试:` selector
  When CLI 渲染 `agent-spec contract`
  Then 默认输出保留这些结构化信息
  And Claude Code 的 tool-first 路径不再丢失关键验收上下文

## Out of Scope

- `agent-spec explain`
- run log 与执行历史
- 真实 AI backend
