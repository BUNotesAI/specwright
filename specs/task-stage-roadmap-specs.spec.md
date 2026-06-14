spec: task
name: "分阶段落盘后续改进路线图"
inherits: project
tags: [bootstrap, roadmap, planning]
---

## Intent

把新的改进计划重写成一组可自举的 task specs，
让后续阶段不再只存在于长文计划里，而是作为仓库内可被 `agent-spec contract` 消费的路线图合同存在。

## Decisions

- 未来阶段的 roadmap task specs 存放在 `specs/roadmap/`
- roadmap specs 在被提升到顶层 `specs/` 前，不进入默认 top-level guard
- roadmap 按 concern 分阶段拆分：Phase 0 Contract fidelity、Phase 1 review loop、Phase 2 traceability、Phase 3 governance、Phase 4 AI、Phase 5 ecosystem、Phase 6 advanced verification
- 嵌套 roadmap specs 仍应继承顶层 `specs/project.spec`

## Boundaries

### Allowed Changes
- src/spec_parser/**
- src/**
- specs/**

### Forbidden
- 不要把未来 backlog spec 直接堆到顶层 `specs/` 并让默认 guard 变红
- 不要把多个阶段重新压回一个大而全的“万能 spec”
- 不要在路线图拆分时改动当前验证语义

## Completion Criteria

Scenario: 嵌套 roadmap spec 继续继承顶层 project 规则
  Test:
    Filter: resolves_parent_from_nested_spec_directory_via_ancestor_specs_dir
  Given 某个 roadmap task spec 位于 `specs/roadmap/`
  When parser 默认解析其继承链
  Then 顶层 `specs/project.spec` 仍可被发现
  And 用户不需要为 roadmap 目录单独配置搜索路径

Scenario: Phase 0 与 Phase 1 roadmap spec 已拆分并表达正确优先级
  Test:
    Package: agent-spec
    Filter: test_roadmap_phase_zero_and_one_specs_exist_and_capture_priorities
  Given 仓库内存在 roadmap task specs
  When 用户查看前两个 phase spec
  Then Phase 0 优先修 Contract fidelity
  And Phase 1 聚焦 explain、markdown PR 摘要与安全版 stamp

Scenario: 后续 roadmap spec 按 concern 分层
  Test:
    Package: agent-spec
    Filter: test_roadmap_later_phase_specs_exist_and_are_split_by_concern
  Given 用户继续查看后续 phase spec
  When 用户对比 Phase 2 到 Phase 6
  Then 过程追踪、Spec 治理、AI 扩展、生态集成与高级验证被拆成独立任务
  And `phase:` front matter 没有被纳入主路线

Scenario: roadmap README 说明 staging 与 promotion 规则
  Test:
    Package: agent-spec
    Filter: test_roadmap_readme_documents_promotion_rule
  Given 用户查看 `specs/roadmap/README.md`
  When 用户阅读 roadmap 目录说明
  Then 说明文档指出 roadmap specs 暂不进入默认 guard
  And 说明它们在实现启动时会被提升到顶层 `specs/`
