spec: task
name: "保留步骤表格输入"
inherits: project
tags: [bootstrap, parser, dsl]
---

## Intent

把 BDD 中紧跟在步骤后的表格输入保留到 AST 里，
为后续把 Given/When/Then 场景自动转成测试代码打下基础。

## Decisions

- parser 把缩进表格行绑定到前一条步骤
- AST 与 JSON 解析输出都保留表格单元格
- 表格回归测试使用 HTTP 请求风格示例

## Boundaries

### Allowed Changes
- src/spec_core/**
- src/spec_parser/**

### Forbidden
- 不要把表格行拆成新的步骤
- 不要丢弃步骤后的结构化输入
- 不要影响没有表格的旧场景

## Completion Criteria

Scenario: When 步骤携带请求表格
  Test: test_parse_step_table_and_preserve_json_output
  Given 某个 `When` 步骤后面跟随请求字段表格
  When parser 生成 AST
  Then AST 中只有一条 `When` 步骤
  And 该 `When` 步骤附带表格单元格

Scenario: JSON 输出保留表格
  Test: test_parse_step_table_and_preserve_json_output
  Given 上述场景被 `specwright parse --format json` 解析
  When 用户查看 JSON 输出
  Then JSON 中包含表格单元格
  And 表格行不会以独立步骤出现

Scenario: 无表格场景保持兼容
  Test: test_parse_scenario_without_table_stays_unchanged
  Given 某个旧 spec 只有普通 Given/When/Then 行
  When parser 解析该旧 spec
  Then 旧场景的步骤数量不变
  And 旧场景的步骤文本保持不变
