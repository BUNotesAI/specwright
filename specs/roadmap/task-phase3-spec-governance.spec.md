spec: task
name: "Phase 3：Spec Governance"
inherits: project
tags: [roadmap, planned, phase3, governance]
---

## Intent

把 `specwright` 从单个 Task Contract 的验证器，
扩展成项目级别的 Spec 治理工具，但仍然保持 CLI-first 和确定性优先。

## Decisions

- 支持 `org.spec -> project.spec -> task.spec` 的三层继承
- `lint --quality` 负责给出 testability 与 spec smell 报告
- `lint --cross-check` 只做机械矛盾检测，不做启发式“猜测冲突”
- 本阶段不把 `phase:` 字段写进 spec front matter

## Boundaries

### Allowed Changes
- src/spec_core/**
- src/spec_parser/**
- src/spec_lint/**
- src/**
- src/spec_gateway/**
- README.md
- specs/**

### Forbidden
- 不要在没有修好完整继承前直接叠加 `org.spec`
- 不要把 workflow 状态写进 `.spec` 头部
- 不要让 `cross-check` 变成不可解释的启发式评分器

## Completion Criteria

Scenario: org.spec 参与三层继承链
  Test:
    Filter: test_load_resolves_org_project_task_chain
  Given 仓库同时存在 `org.spec`、`project.spec` 与 task spec
  When gateway 加载 task spec
  Then Task Contract 包含组织级与项目级的继承规则
  And 近层规则覆盖远层规则

Scenario: lint 报告 Spec 质量
  Test:
    Filter: test_quality_report_scores_testability_and_smells
  Given 某个 Contract 含有明确 Test binding 与若干 spec smell
  When 用户运行 `specwright lint --quality`
  Then 输出包含 testability、smell 与整体评分
  And 评分依据可解释

Scenario: lint 检测跨 spec 机械矛盾
  Test:
    Filter: test_cross_check_reports_boundary_and_decision_conflicts
  Given 同目录下多个 spec 在 Boundaries 或 Decisions 上存在机械冲突
  When 用户运行 `specwright lint --cross-check`
  Then 输出指出冲突的 spec 与规则
  And 不把主观建议伪装成确定性冲突

## Out of Scope

- run log
- `phase:` front matter
- 对抗性 AI 验证
