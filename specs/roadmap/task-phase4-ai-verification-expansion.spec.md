spec: task
name: "Phase 4：AI Verification Expansion"
inherits: project
tags: [roadmap, planned, phase4, ai]
---

## Intent

在保持确定性验证优先的前提下，
把 AI 验证从 stub 提升成真正可用的辅助层，用来处理机械验证无法覆盖的剩余场景。

## Decisions

- provider 选择与鉴权继续留在宿主或适配层，不把 provider 配置塞回核心 Contract 模型
- 先增强 `AiRequest` 的上下文保真度，再接真实 backend
- sycophancy-aware lint 先于 adversarial 模式交付
- adversarial 模式保持显式 opt-in，且位于普通 AI 验证之后

## Boundaries

### Allowed Changes
- src/spec_core/**
- src/spec_lint/**
- src/spec_verify/**
- src/spec_gateway/**
- src/**
- README.md
- specs/**

### Forbidden
- 不要让 AI 层改变默认的确定性验证路径
- 不要把 provider/model/auth 配置写死进 Task Contract
- 不要在 prompt 中用“必须找出 bug”这类诱导性表达

## Completion Criteria

Scenario: AI request 打包完整验证上下文
  Test:
    Filter: test_build_ai_request_includes_contract_change_set_and_evidence_context
  Given 某个场景需要 AI 验证
  When verifier 构造 `AiRequest`
  Then 请求包含 Contract、change set 与相关证据上下文
  And 不只剩下场景名和裸步骤文本

Scenario: lint 检测 sycophancy 风险
  Test:
    Filter: test_sycophancy_linter_flags_bug_finding_bias
  Given 某个 Spec 使用“找出所有 bug”这类诱导性语句
  When 用户运行 lint
  Then 输出指出 sycophancy 风险
  And 给出中性重写建议

Scenario: adversarial 验证保持显式 opt-in
  Test:
    Package: specwright
    Filter: test_adversarial_verification_is_disabled_by_default
  Given 用户仅启用普通 AI 验证
  When lifecycle 或 verify 执行
  Then 不会自动触发多 Agent 对抗流程
  And 对抗性验证只在显式参数下运行

## Out of Scope

- 把 provider 配置作为核心 CLI 契约的一部分
- 默认开启 adversarial 模式
