spec: task
name: "退款功能"
inherits: project
tags: [payment, refund]
---

## Intent

为支付网关添加退款功能，支持全额和部分退款。
退款需要管理员权限，且必须在原始交易后 90 天内发起。

## Constraints

- 退款金额不得超过原始交易金额
- 退款操作需要管理员权限
- 退款必须在原交易后 90 天内发起
- 退款状态机: pending -> processing -> completed | failed
- 退款接口响应时间不超过 500ms

## Acceptance Criteria

Scenario: 全额退款
  Test: test_full_refund_flow
  Given 存在一笔金额为 "100.00" 元的已完成交易 "TXN-001"
  And 当前用户具有管理员权限
  When 用户对 "TXN-001" 发起全额退款
  Then 退款状态变为 "processing"
  And 原始交易状态变为 "refunding"

Scenario: 部分退款
  Test: test_partial_refund_flow
  Given 存在一笔金额为 "100.00" 元的已完成交易 "TXN-002"
  When 用户对 "TXN-002" 发起 "30.00" 元的部分退款
  Then 剩余可退金额为 "70.00" 元
  And 允许后续再次部分退款

Scenario: 退款拒绝 - 超期
  Test: test_refund_rejects_expired_transaction
  Given 存在一笔 91 天前完成的交易 "TXN-003"
  When 用户对 "TXN-003" 发起退款
  Then 系统拒绝退款
  And 返回错误信息包含 "超过退款期限"

Scenario: 退款拒绝 - 金额超限
  Test: test_refund_rejects_amount_exceeding_original
  Given 存在一笔金额为 "100.00" 元的已完成交易 "TXN-004"
  When 用户对 "TXN-004" 发起 "150.00" 元的退款
  Then 系统拒绝退款
  And 返回错误码 "REFUND_EXCEEDS_ORIGINAL"

## Out of Scope

- 登录功能
- 密码重置
- 第三方支付对接
