spec: task
name: "修复继承链解析入口"
inherits: project
tags: [bootstrap, parser, gateway, cli]
---

## Intent

让磁盘上的任务级 `.spec` 能直接继承同目录的 `project.spec`，
使 `agent-spec` 可以先用自己的项目规则约束自己，再执行具体任务。

## Decisions

- 磁盘入口优先从当前 spec 文件所在目录解析父级规格
- 继承链修复必须同时覆盖 gateway 与 CLI 行为
- 计划阶段产出的 Task Contract 必须包含继承得到的项目级约束

## Boundaries

### Allowed Changes
- src/spec_parser/**
- src/spec_gateway/**
- src/**

### Forbidden
- 不要硬编码仓库绝对路径
- 不要让普通磁盘用例手工传入 `search_dirs`
- 不要破坏 `from_input` 这种无文件路径的入口

## Completion Criteria

Scenario: 同目录继承 project 规格
  Test: test_load_resolves_inherited_constraints_from_spec_directory
  Given 临时目录中存在 `project.spec` 和 `task.spec`
  And `task.spec` 声明 `inherits: project`
  When `SpecGateway::load` 读取 `task.spec`
  Then 解析结果中包含来自 `project.spec` 的约束
  And Task Contract 输出中也包含这些继承约束

Scenario: 磁盘入口不需要手工搜索路径
  Test: resolves_parent_from_source_directory_when_no_search_dirs_are_provided
  Given 用户直接运行 `agent-spec contract task.spec`
  When 该 `task.spec` 与 `project.spec` 位于同一目录
  Then CLI 可以完成继承解析
  And 用户不需要再手工提供 `search_dirs`

Scenario: 内存入口保持原样
  Test: test_full_lifecycle
  Given 调用 `SpecGateway::from_input`
  When 输入内容本身不包含继承链
  Then 该入口继续可用
  And 行为与修复前保持一致
