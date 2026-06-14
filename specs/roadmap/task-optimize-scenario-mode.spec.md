spec: task
name: "开放式优化场景模式"
inherits: project
tags: [bootstrap, parser, lifecycle, phase9]
depends: [task-checkpoint-resume]
estimate: 0.5d
---

## Intent

支持"持续改善"类场景——测试达标后 lifecycle 不终止优化，
而是在报告中标记该场景为"可继续优化"，
让 agent 在"规格满足"和"持续探索"之间选择。

灵感来源：Autoresearch 的"永不停止"循环模式。

## Decisions

- 场景新增 `模式:` / `Mode:` 字段，取值 `standard`（默认）或 `optimize`
- `optimize` 模式场景 pass 后，lifecycle 的 `passed` 仍为 `true`
- JSON 输出新增 `optimization_candidates` 数组，列出可继续改善的场景
- `optimize` 场景的 fail 仍然导致 `passed: false`（达标是最低要求）

## Boundaries

### Allowed Changes
- src/spec_core/**
- src/spec_parser/**
- src/spec_gateway/**
- src/spec_report/**

### Forbidden
- 不要让 `optimize` 模式改变 fail 的语义
- 不要在 `standard` 模式下产生 `optimization_candidates`

## Completion Criteria

Scenario: optimize 场景 pass 后出现在 optimization_candidates
  Test:
    Package: agent-spec
    Filter: test_optimize_scenario_pass_listed_as_candidate
  Given 某个场景声明 `模式: optimize` 且 verdict 为 `pass`
  When lifecycle 输出 JSON
  Then `optimization_candidates` 包含该场景名称
  And `passed` 为 `true`

Scenario: optimize 场景 fail 不影响 passed 判定
  Test:
    Package: agent-spec
    Filter: test_optimize_scenario_fail_blocks_pass
  Given 某个场景声明 `模式: optimize` 且 verdict 为 `fail`
  When lifecycle 输出 JSON
  Then `passed` 为 `false`

Scenario: parser 正确解析模式字段
  Test:
    Package: agent-spec
    Filter: test_parse_mode_field_in_scenario
  Given 某个场景声明 `模式: optimize`
  When parser 解析该场景
  Then AST 中 `mode` 字段为 `Optimize`
