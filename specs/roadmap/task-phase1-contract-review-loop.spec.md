spec: task
name: "Phase 1：Contract Review Loop"
inherits: project
tags: [roadmap, planned, phase1, review]
---

## Intent

把 `specwright` 从“验证工具”升级成“Contract 取代代码 diff 的 review 入口”，
先交付 reviewer 真正会用到的摘要和 PR 集成，而不是先做侵入式 VCS 改写。

## Decisions

- `specwright explain` 是本阶段第一优先级
- `--format markdown` 与 PR description 复用同一套 explain 渲染
- `stamp` 第一版默认安全，不应默认通过 `git commit --amend` 改写历史
- GitHub Actions 示例属于交付的一部分，但保持为文档与样例，不把 GitHub 逻辑硬编码进核心验证管道

## Boundaries

### Allowed Changes
- src/**
- src/spec_report/**
- README.md
- .github/workflows/**
- specs/**

### Forbidden
- 不要先做 destructive `stamp` 再做 `explain`
- 不要让 explain 退化成单纯复制 lifecycle JSON
- 不要把 GitHub CLI 作为 explain 的必需依赖

## Completion Criteria

Scenario: explain 生成人类可读的 Contract 摘要
  Test:
    Package: specwright
    Filter: test_explain_command_renders_contract_review_summary
  Given 某个 task spec 已通过 lifecycle
  When 用户运行 `specwright explain task.spec`
  Then 输出包含 Intent、Decisions、Boundaries 与 Verification Summary
  And 适合作为 reviewer 的一屏摘要

Scenario: explain 生成 PR description markdown
  Test:
    Package: specwright
    Filter: test_explain_markdown_output_is_suitable_for_pr_description
  Given 某个 task spec 需要生成 PR 说明
  When 用户运行 `specwright explain task.spec --format markdown`
  Then 输出为结构化 markdown
  And 不要求用户额外拼装 Contract 摘要

Scenario: stamp 默认安全且支持预览
  Test:
    Package: specwright
    Filter: test_stamp_dry_run_outputs_trailers_without_rewriting_history
  Given 某个 commit 对应的 Contract 已通过验证
  When 用户运行 `specwright stamp --dry-run`
  Then 输出包含将要写入的 trailer
  And 默认不直接改写 commit 历史

## Out of Scope

- run log 与 explain --history
- jj change scope
- 真实 AI backend
