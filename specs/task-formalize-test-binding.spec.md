spec: task
name: "正式化场景到测试的绑定"
inherits: project
tags: [bootstrap, verify, parser, contract]
---

## Intent

把任务合约里的完成条件与 Rust 测试之间的绑定，
从临时的 `// @spec:` 注释约定升级为 spec 内可声明的正式机制。

## Decisions

- 场景可用 `测试:` / `Test:` 显式声明 Rust test selector
- `TestVerifier` 优先使用场景内显式 selector
- 旧的 `// @spec:` 注释继续保留为兼容 fallback

## Boundaries

### Allowed Changes
- src/spec_core/**
- src/spec_parser/**
- src/spec_verify/**
- src/**
- specs/**

### Forbidden
- 不要移除现有 `// @spec:` 兼容能力
- 不要要求所有旧 spec 一次性迁移
- 不要把测试绑定继续留在只靠源码注释的状态

## Completion Criteria

Scenario: 场景可显式声明测试选择器
  Test: test_parse_scenario_with_explicit_test_selector
  Given 某个场景块包含 `测试:` 行
  When parser 解析该场景
  Then AST 中保留 `test_selector`
  And JSON 输出中也包含该 selector

Scenario: 显式测试选择器优先于旧注释映射
  Test: test_explicit_scenario_selector_takes_precedence_over_legacy_comment_binding
  Given 同一个场景同时存在显式 selector 和旧注释映射
  When TestVerifier 解析绑定关系
  Then 显式 selector 优先
  And 不再依赖场景名去匹配测试函数

Scenario: 旧版注释绑定继续兼容
  Test: test_legacy_comment_binding_is_used_when_no_explicit_selector_exists
  Given 某个场景没有显式 selector
  When TestVerifier 解析绑定关系
  Then 旧版 `// @spec:` 映射仍然可用
  And 现有自举规格不需要一次性全部迁移
