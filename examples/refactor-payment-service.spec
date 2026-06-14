spec: task
name: "重构退款服务但保持外部契约"
tags: [example, review, refactor, contract]
---

## Intent

在不改变外部 API 契约的前提下重构退款服务内部结构，
把校验规则与编排逻辑拆开，降低后续代码审查的认知负担。

## Decisions

- 保留现有退款 HTTP 接口与 JSON 字段命名
- 把退款校验抽到独立的 `RefundPolicy`
- 用显式集成测试覆盖成功路径和拒绝路径

## Boundaries

### Allowed Changes
- crates/refund-service/**
- crates/refund-policy/**
- tests/refund_service_contract.rs

### Forbidden
- 不要修改已有响应 JSON 的字段名
- 不要改动数据库 migration
- 不要引入 `panic!`、`.unwrap()` 或 `todo!`

## Completion Criteria

Scenario: 全额退款保持现有返回结构
  Test: test_refund_service_keeps_existing_success_payload
  Given 存在一笔金额为 "100.00" 元的已完成交易 "TXN-001"
  When 用户对 "TXN-001" 发起全额退款
  Then 响应状态码为 202
  And 响应体包含字段 "refund_id"

Scenario: 超限退款返回稳定错误码
  Test: test_refund_service_rejects_refund_exceeding_original_amount
  Given 存在一笔金额为 "100.00" 元的已完成交易 "TXN-002"
  When 用户对 "TXN-002" 发起 "150.00" 元的退款
  Then 响应状态码为 422
  And 响应体包含错误码 "REFUND_EXCEEDS_ORIGINAL"

## Out of Scope

- 管理员权限系统
- 第三方支付渠道接入
- 后台运营报表
