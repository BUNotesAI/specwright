spec: task
name: "Task Contract 成为默认执行入口"
inherits: project
tags: [bootstrap, contract, gateway, cli]
---

## Intent

把 `TaskContract` 变成 `specwright` 的默认执行入口，
让 agent 在计划阶段默认消费 Contract，而不是历史遗留的简化 brief。

## Decisions

- `SpecGateway::plan()` 返回默认执行用的 `TaskContract`
- `SpecGateway::brief()` 继续保留，但仅作为兼容层
- `specwright brief` 继续可用，但作为 `specwright contract` 的兼容别名

## Boundaries

### Allowed Changes
- src/spec_gateway/**
- src/**
- specs/**

### Forbidden
- 不要删除现有 `brief()` API
- 不要让 `brief` 与 `contract` 输出出现语义漂移
- 不要破坏现有 task spec 的解析与验证行为

## Completion Criteria

Scenario: Gateway 计划阶段返回 Task Contract
  Test: test_plan_returns_task_contract
  Given 某个任务级 spec 已被加载
  When 调用 `SpecGateway::plan()`
  Then 返回值是 `TaskContract`
  And 输出使用 `Task Contract` 标题

Scenario: Brief 命令是 contract 兼容别名
  Test: test_brief_output_matches_contract_output
  Given 同一个任务级 spec
  When CLI 分别渲染 `brief` 与 `contract`
  Then 两者输出保持一致
  And 输出继续使用 `Task Contract` 结构
