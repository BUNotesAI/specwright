spec: task
name: "AiVerifier 可插拔 backend 接口"
inherits: project
tags: [bootstrap, verify, ai, gateway, phase4]
---

## Intent

把 `AiVerifier` 从写死的 stub 逻辑升级成可插拔 backend 接口，
为后续接入真实模型 provider 做准备，同时保持当前默认行为不变。

## Decisions

- 引入 `AiBackend` 抽象，输入为结构化 `AiRequest`，输出为结构化 `AiDecision`
- `StubAiBackend` 继续作为内置 backend，保持当前 `stub` 模式语义
- `AiVerifier` 通过 backend 产生结果，而不是直接硬编码 `AiAnalysis`

## Boundaries

### Allowed Changes
- src/spec_core/**
- src/spec_verify/**
- src/spec_gateway/**
- specs/**
- README.md

### Forbidden
- 不要改变默认 `AiMode::Off` 行为
- 不要移除现有 `stub` 模式
- 不要把 backend 输出退化成非结构化字符串

## Completion Criteria

Scenario: Stub backend 返回结构化 AI 决策
  Test:
    Filter: test_stub_ai_backend_returns_uncertain_decision
  Given 某个场景被提交给 `StubAiBackend`
  When backend 生成 AI 决策
  Then 返回结构化 `AiDecision`
  And verdict 为 `uncertain`

Scenario: AiVerifier 使用 backend 响应构造结果
  Test:
    Filter: test_ai_verifier_with_custom_backend_uses_backend_response
  Given 某个自定义 backend 返回结构化 AI 决策
  When `AiVerifier` 使用该 backend 验证场景
  Then 结果中的 verdict 与证据来自 backend 响应
  And reasoning 被保留到 `AiAnalysis`

Scenario: AI request 包含场景与代码上下文
  Test:
    Filter: test_build_ai_request_includes_scenario_and_code_paths
  Given 某个场景和代码路径被交给 `AiVerifier`
  When verifier 构造 `AiRequest`
  Then request 中包含 `spec_name`、`scenario_name` 和步骤文本
  And request 中包含代码路径上下文
