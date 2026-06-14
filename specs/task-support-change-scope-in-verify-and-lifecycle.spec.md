spec: task
name: "verify 与 lifecycle 支持可选 change scope"
inherits: project
tags: [bootstrap, cli, git, boundaries, lifecycle, verify, phase4]
---

## Intent

让单任务验证入口也能直接消费 git 变更语义，
在需要边界校验时无需手工枚举 `--change`，同时保持当前默认行为稳定。

## Decisions

- `verify` 与 `lifecycle` 新增 `--change-scope`
- 默认 scope 为 `none`，不自动推导 git 变更
- 显式 `--change` 继续拥有最高优先级

## Boundaries

### Allowed Changes
- src/**
- specs/**
- README.md

### Forbidden
- 不要把 `verify` 或 `lifecycle` 的默认行为改成自动读取 git 变更
- 不要让 `--change-scope` 覆盖显式 `--change`
- 不要让 `none` scope 在 git 仓库里偷偷读取 staged 或 worktree

## Completion Criteria

Scenario: lifecycle 在 worktree scope 下读取整棵工作区变更
  Test:
    Package: specwright
    Filter: test_resolve_command_change_paths_reads_worktree_git_changes
  Given 某个临时 git 仓库同时存在 staged、未暂存和未跟踪变更
  When `lifecycle` 使用 `worktree` change scope 解析 change set
  Then 返回结果包含这三类路径

Scenario: verify 默认 none scope 保持空 change set
  Test:
    Package: specwright
    Filter: test_resolve_command_change_paths_returns_empty_for_none_scope
  Given 某个临时 git 仓库存在 staged 变更
  When `verify` 使用默认 `none` scope 解析 change set
  Then 返回空 change set
  And 不依赖 git 自动发现

Scenario: 显式 change 参数继续优先于自动 scope
  Test:
    Package: specwright
    Filter: test_resolve_command_change_paths_prefers_explicit_changes
  Given 用户显式传入 `custom/file.rs`
  When `verify` 同时配置 `worktree` scope
  Then 返回结果继续使用显式传入的路径
  And 不依赖 git 自动发现
