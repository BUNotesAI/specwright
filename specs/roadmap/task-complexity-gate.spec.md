spec: task
name: "代码质量门禁（Complexity Gate）"
inherits: project
tags: [bootstrap, verify, phase8]
depends: [task-goal-gate]
estimate: 1d
---

## Intent

在 lifecycle 的验证阶段增加可选的代码质量检查，
防止 agent 写出"所有测试通过但极其臃肿"的实现。
质量约束从 spec 的 Constraints 中提取，支持行数比和复杂度指标。

灵感来源：Autoresearch 的简洁性标准——"0.001 改善 + 20 行丑代码 = 不值得"。

## Decisions

- 新增 `ComplexityVerifier`，与现有 4 个 verifier 并列
- 质量约束从 Constraints 的 `Must` 类别中识别特定关键词
- `--layers` 增加 `complexity` 选项
- 不引入外部工具依赖，使用 git diff 统计行数变化
- 产生隐式场景 `[complexity] code quality gate`

## Boundaries

### Allowed Changes
- src/spec_verify/**
- src/spec_core/**
- src/spec_gateway/**
- src/main.rs

### Forbidden
- 不要让 complexity verifier 在无质量约束时产生任何 verdict
- 不要强制依赖 clippy 或其他外部 lint 工具
- 不要修改现有 verifier 的行为

## Completion Criteria

Scenario: 行数比超标时 fail
  Test:
    Package: agent-spec
    Filter: test_complexity_verifier_fails_on_line_ratio_exceeded
  Given 某个 spec 声明"新增行数不超过删除行数的 3 倍"
  When 变更集净增 100 行、删除 10 行
  Then `[complexity]` 场景 verdict 为 `fail`
  And evidence 包含实际行数比

Scenario: 无质量约束时无 verdict
  Test:
    Package: agent-spec
    Filter: test_complexity_verifier_silent_without_constraints
  Given 某个 spec 的 Constraints 中没有质量相关关键词
  When lifecycle 执行 complexity 层
  Then 不产生任何额外场景或 verdict

Scenario: 行数比达标时 pass
  Test:
    Package: agent-spec
    Filter: test_complexity_verifier_passes_on_acceptable_ratio
  Given 某个 spec 声明"新增行数不超过删除行数的 3 倍"
  When 变更集净增 20 行、删除 10 行
  Then `[complexity]` 场景 verdict 为 `pass`

Scenario: 使用 git diff 统计行数变化
  Test:
    Package: agent-spec
    Filter: test_complexity_verifier_uses_git_diff_stats
  Given 某个 spec 声明行数比约束
  When ComplexityVerifier 计算变更统计
  Then 统计来源为 git diff 的 `--stat` 输出
  And 不依赖外部 lint 工具
