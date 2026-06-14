spec: task
name: "代码质量检查"
tags: [quality]
---

## Intent

确保代码库不使用危险的方法调用。

## Constraints

### Must Not

- 生产代码中禁止使用 `.unwrap()`
- 禁止使用 `panic!` 宏
- 禁止使用 `todo!` 宏

## Acceptance Criteria

Scenario: 无 unwrap 调用
  Test: test_no_unwrap_calls_exist
  Given 代码库已编译通过
  When 扫描所有源代码文件
  Then 不应存在 .unwrap() 调用
