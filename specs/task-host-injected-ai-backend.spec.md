spec: task
name: "宿主注入 AI backend"
inherits: project
tags: [bootstrap, ai, gateway, embed, phase4]
---

## Intent

让 `specwright` 保持 provider-agnostic，
由嵌入它的宿主 agent 注入自己的 AI backend，而不是在 `specwright` 内部配置 provider。

## Decisions

- `spec-gateway` 暴露接受 `AiBackend` 的验证入口
- `specwright` CLI 继续只保留 `off` / `stub`，不承载 provider 配置
- provider、模型、鉴权和超时等配置由宿主 agent 负责

## Boundaries

### Allowed Changes
- src/spec_gateway/**
- src/spec_verify/**
- specs/**
- README.md

### Forbidden
- 不要在 `specwright` 内部引入 provider 配置模型
- 不要要求宿主 agent 先把 provider 转换成 CLI 参数再调用 gateway
- 不要破坏现有 `stub` 模式和默认 `off` 行为

## Completion Criteria

Scenario: gateway 支持注入自定义 AI backend
  Test:
    Filter: test_verify_with_injected_ai_backend_uses_host_backend
  Given 某个宿主 agent 提供自定义 `AiBackend`
  When gateway 使用该 backend 执行验证
  Then 验证结果来自该 backend
  And `AiAnalysis` 证据保留 backend 返回的 model 与 reasoning

Scenario: 默认 gateway 入口仍不依赖外部 provider
  Test:
    Filter: test_verify_default_keeps_uncovered_scenarios_skipped
  Given 某个未被覆盖的场景
  When gateway 使用默认验证入口
  Then 场景仍然是 `skip`
  And 不要求传入 provider 配置
