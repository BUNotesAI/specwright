spec: task
name: "人类审核场景（Human Review）"
inherits: project
tags: [bootstrap, verify, parser, phase9]
depends: [task-complexity-gate]
estimate: 0.5d
---

## Intent

让场景可以声明"测试通过后仍需人类确认"，
区分"机器可验证"与"需要人类判断"两类验收条件，
同时保持默认行为全自动（向后兼容）。

灵感来源：Attractor 的 Interviewer 接口抽象（AutoApprove / Console / Queue）。

## Decisions

- 场景新增 `审核:` / `Review:` 字段，取值 `auto`（默认）或 `human`
- `human` 审核的场景测试通过后 verdict 为 `pending_review`（新增 verdict）
- `--review-mode` 控制如何处理 `pending_review`：
  - `auto`（默认）：`pending_review` 视为 `pass`
  - `strict`：`pending_review` 视为非通过
- Verdict 枚举新增 `PendingReview` 变体
- `pending_review` 不参与 gate_blocked 判定

## Boundaries

### Allowed Changes
- src/spec_core/**
- src/spec_parser/**
- src/spec_verify/**
- src/spec_gateway/**
- src/spec_report/**
- src/main.rs

### Forbidden
- 不要在 `--review-mode auto` 下改变现有行为
- 不要让 `pending_review` 阻塞无 `审核: human` 的场景
- 不要移除对 `pass/fail/skip/uncertain` 四种 verdict 的支持

## Completion Criteria

Scenario: human 审核场景测试通过后为 pending_review
  Test:
    Package: agent-spec
    Filter: test_human_review_scenario_produces_pending_review
  Given 某个场景声明 `审核: human` 且测试通过
  When lifecycle 执行该场景
  Then verdict 为 `pending_review`

Scenario: auto 模式下 pending_review 视为通过
  Test:
    Package: agent-spec
    Filter: test_auto_review_mode_treats_pending_as_pass
  Given 某个场景 verdict 为 `pending_review`
  When lifecycle 使用默认 `--review-mode auto`
  Then 最终 `passed` 为 `true`（假设无其他失败）

Scenario: strict 模式下 pending_review 为非通过
  Test:
    Package: agent-spec
    Filter: test_strict_review_mode_treats_pending_as_not_pass
  Given 某个场景 verdict 为 `pending_review`
  When lifecycle 使用 `--review-mode strict`
  Then 最终 `passed` 为 `false`

Scenario: parser 正确解析审核字段
  Test:
    Package: agent-spec
    Filter: test_parse_review_field_in_scenario
  Given 某个场景声明 `审核: human`
  When parser 解析该场景
  Then AST 中 `review` 字段为 `Human`
