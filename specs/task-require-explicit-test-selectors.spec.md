spec: task
name: "缺少显式测试绑定时阻止通过"
inherits: project
tags: [bootstrap, lint, verify, quality-gate]
---

## Intent

把显式 `测试:` selector 从推荐做法提升为默认质量门槛，
避免任务合约仍然依赖隐式场景名匹配或遗留注释映射。

## Decisions

- 缺少显式 selector 的任务级场景会产生 `error` 级 lint
- `quality_gate` 在存在 `error` 级 lint 时直接失败
- 旧版 `// @spec:` 只作为 verifier 兼容层，不再满足 lint 要求

## Boundaries

### Allowed Changes
- src/spec_core/**
- src/spec_lint/**
- src/spec_gateway/**
- src/**
- specs/**

### Forbidden
- 不要把缺少 selector 只当成 info 级提示
- 不要让 `quality_gate(0.0)` 绕过 error 级 lint
- 不要移除旧版 `// @spec:` verifier fallback

## Completion Criteria

Scenario: 缺少显式绑定的任务场景触发 lint 错误
  Test: test_explicit_test_binding_linter_requires_task_scenario_selectors
  Given 某个任务级 spec 的场景没有声明 `测试:` selector
  When lint pipeline 检查该 spec
  Then 产生 `explicit-test-binding` 规则
  And 诊断级别为 `error`

Scenario: 显式绑定的任务场景通过 lint
  Test: test_explicit_test_binding_linter_accepts_explicit_selector
  Given 某个任务级 spec 的场景声明了 `测试:` selector
  When lint pipeline 检查该 spec
  Then 不会产生 `explicit-test-binding` 错误

Scenario: error 级 lint 阻止质量闸门通过
  Test: test_quality_gate_fails_on_error_lint_issue
  Given 某个任务级 spec 缺少显式测试绑定
  When gateway 运行 `quality_gate(0.0)`
  Then 质量闸门仍然失败
  And 失败原因说明存在 error 级 lint
