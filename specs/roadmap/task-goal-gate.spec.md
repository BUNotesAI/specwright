spec: task
name: "关键场景门禁（Goal Gate）"
inherits: project
tags: [bootstrap, lifecycle, verify, phase7]
depends: []
estimate: 0.5d
---

## Intent

让 lifecycle 区分"普通失败"与"关键门禁被阻塞"，
给 agent 一个更强的未完成信号——critical 场景失败时输出 `gate_blocked`，
驱动 agent 优先解决门禁而非普通 failure。

灵感来源：Attractor 的 `goal_gate=true` 节点机制。

## Decisions

- 通过场景 tags 中的 `critical` 标签标记门禁场景
- 也支持场景名称中的 `（critical）` / `(critical)` 后缀作为简写
- lifecycle JSON 输出新增 `gate_blocked` 布尔字段和 `blocked_gates` 数组
- 当 critical 场景 fail 时退出码为 `2`（区别于普通 fail 的 `1`）
- 无 critical 标签时行为完全不变（向后兼容）

## Boundaries

### Allowed Changes
- src/spec_core/**
- src/spec_gateway/**
- src/spec_report/**
- src/main.rs

### Forbidden
- 不要在无 critical 标签时改变现有退出码语义
- 不要强制所有 spec 必须有 critical 场景
- 不要修改 Verdict 枚举本身

## Completion Criteria

Scenario: critical 场景失败时报告 gate_blocked
  Test:
    Package: agent-spec
    Filter: test_critical_scenario_fail_sets_gate_blocked
  Given 某个任务级 spec 有一个标记为 `critical` 的场景
  When 该场景 verdict 为 `fail`
  Then lifecycle JSON 输出中 `gate_blocked` 为 `true`
  And `blocked_gates` 包含该场景名称

Scenario: critical 场景通过时不触发门禁
  Test:
    Package: agent-spec
    Filter: test_critical_scenario_pass_no_gate_block
  Given 某个任务级 spec 有一个标记为 `critical` 的场景
  When 该场景 verdict 为 `pass`
  Then lifecycle JSON 输出中 `gate_blocked` 为 `false`
  And `blocked_gates` 为空数组

Scenario: 无 critical 标签时行为不变
  Test:
    Package: agent-spec
    Filter: test_no_critical_tag_preserves_existing_behavior
  Given 某个任务级 spec 没有任何 `critical` 标签
  When lifecycle 输出 JSON 结果
  Then 输出中 `gate_blocked` 为 `false`
  And 退出码语义与现有行为一致

Scenario: 场景名称后缀作为 critical 简写
  Test:
    Package: agent-spec
    Filter: test_critical_suffix_in_scenario_name
  Given 某个场景名称为 "用户注册成功（critical）"
  When parser 解析该场景
  Then 该场景被识别为 critical
  And 场景名称中的 `（critical）` 后缀被移除后保留为显示名

Scenario: critical 失败的退出码为 2
  Test:
    Package: agent-spec
    Filter: test_critical_fail_exit_code_is_2
  Given 某个任务级 spec 有 critical 场景且 verdict 为 `fail`
  When lifecycle 命令执行完毕
  Then 退出码为 `2`
