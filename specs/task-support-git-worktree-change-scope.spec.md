spec: task
name: "Guard 支持 git worktree change scope"
inherits: project
tags: [bootstrap, cli, git, boundaries, guard, phase4]
---

## Intent

让 `specwright guard` 在需要更强边界校验时，
可以从整个 git worktree 推导 change set，而不只局限于 staged index。

## Decisions

- `guard` 新增 `--change-scope`，首批支持 `staged` 与 `worktree`
- 默认 scope 仍然是 `staged`，保持 pre-commit 语义稳定
- `worktree` scope 包含 staged、未暂存和未跟踪文件

## Boundaries

### Allowed Changes
- src/**
- specs/**
- README.md

### Forbidden
- 不要改变默认 `guard` 行为为 worktree
- 不要让显式 `--change` 失去最高优先级
- 不要在 worktree 模式下漏掉未跟踪文件

## Completion Criteria

Scenario: worktree scope 包含 staged、未暂存和未跟踪文件
  Test:
    Package: specwright
    Filter: test_resolve_guard_change_paths_reads_worktree_git_changes
  Given 某个临时 git 仓库同时存在 staged、未暂存和未跟踪变更
  When `guard` 使用 `worktree` change scope 解析 change set
  Then 返回结果包含这三类路径

Scenario: 默认 staged scope 不包含未暂存改动
  Test:
    Package: specwright
    Filter: test_resolve_guard_change_paths_ignores_unstaged_changes_in_default_staged_scope
  Given 某个临时 git 仓库存在 staged 和未暂存改动
  When `guard` 使用默认 `staged` scope 解析 change set
  Then 返回结果只包含 staged 路径
  And 未暂存改动不会被纳入

Scenario: 显式 change 参数优先于 scope 自动发现
  Test:
    Package: specwright
    Filter: test_resolve_guard_change_paths_prefers_explicit_changes
  Given 用户显式传入 `custom/file.rs`
  When `guard` 同时配置 `worktree` scope
  Then 返回结果继续使用显式传入的路径
  And 不依赖 git 自动发现
