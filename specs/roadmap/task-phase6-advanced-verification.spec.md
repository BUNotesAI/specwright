spec: task
name: "Phase 6：Advanced Verification"
inherits: project
tags: [roadmap, planned, phase6, verification]
---

## Intent

把验证金字塔做完整，但保持它是探索性、显式启用、成本可见的高级能力，
而不是把高成本验证塞进每次默认 lifecycle。

## Decisions

- 验证层级以显式 `layers` 开关暴露
- 成本报告按层输出 token、时长与命中场景
- Contract 确定性度量保持实验性质，不进入默认 CI

## Boundaries

### Allowed Changes
- src/**
- src/spec_core/**
- src/spec_gateway/**
- src/spec_report/**
- README.md
- specs/**

### Forbidden
- 不要让高成本层默认开启
- 不要把实验性确定性度量写进基础质量门槛
- 不要模糊各验证层的成本边界

## Completion Criteria

Scenario: lifecycle 支持显式验证层选择
  Test:
    Package: agent-spec
    Filter: test_lifecycle_layers_flag_selects_verification_stack
  Given 用户只想运行部分验证层
  When lifecycle 接收 `--layers lint,boundary,test`
  Then 只运行指定层
  And 报告中保留每层的独立结果

Scenario: 成本报告按层输出
  Test:
    Filter: test_cost_report_breaks_down_tokens_time_and_layers
  Given 某次生命周期执行同时使用了 test 与 AI 层
  When 用户请求成本报告
  Then 输出包含每层的 token、时间与汇总成本
  And 用户能看到高成本层是否值得开启

Scenario: 确定性度量保持实验功能
  Test:
    Package: agent-spec
    Filter: test_measure_determinism_is_explicitly_experimental
  Given 用户希望评估 Contract 方差
  When 用户查看 `measure-determinism`
  Then 命令被标注为实验性
  And 不会进入默认的 lifecycle 或 guard

## Out of Scope

- 默认启用多 Agent 实现比较
- 把成本报告变成强制门禁
