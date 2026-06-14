spec: task
name: "支持 .spec.md 双扩展名"
inherits: project
tags: [enhancement, ux, backward-compat]
---

## Intent

让 `.spec` 文件可以使用 `.spec.md` 扩展名，使 GitHub、VS Code 等工具自动识别为
Markdown 并渲染预览。采用"双认 + 新建默认"策略：guard/resolver 同时接受
`.spec` 和 `.spec.md`，`init` 命令默认生成 `.spec.md`，不强制迁移旧文件。

## Decisions

- guard 扫描目录时同时匹配 `.spec` 和 `.spec.md` 扩展名
- resolver 继承查找时，每个候选名同时尝试 `.spec.md` 和 `.spec` 两种后缀，`.spec.md` 优先
- `init` 命令默认生成 `.spec.md` 文件
- 当同一 basename 同时存在 `.spec` 和 `.spec.md` 时，guard 产生 warning
- 不使用 `Path::extension()`（只返回最后一段），改用 `file_name().ends_with(".spec.md")` 或等价判断
- boundary checker 中 `.spec.md` 也被识别为 spec 文件路径

## Boundaries

### Allowed Changes
- src/main.rs
- src/spec_parser/resolver.rs
- src/spec_verify/boundaries.rs
- src/spec_lint/linters.rs
- src/spec_lint/pipeline.rs

### Forbidden
- 不要删除对 `.spec` 扩展名的支持
- 不要重命名仓库内现有的 `.spec` 文件
- 不要修改 spec parser 的语法解析逻辑（只改文件发现和加载路径）

## Out of Scope

- 批量迁移现有 `.spec` 文件为 `.spec.md`
- 文档和 skill 文件中的 `.spec` 引用更新（后续独立任务）
- CI workflow 中的 `*.spec` glob 更新（后续独立任务）

## Completion Criteria

Scenario: guard 发现 .spec.md 文件
  Test: test_guard_discovers_spec_md_files
  Given specs 目录下存在 `task.spec.md` 文件
  When 执行 guard 扫描
  Then `task.spec.md` 被包含在待检查文件列表中

Scenario: guard 同时发现 .spec 和 .spec.md 文件
  Test: test_guard_discovers_both_spec_and_spec_md
  Given specs 目录下同时存在 `a.spec` 和 `b.spec.md`
  When 执行 guard 扫描
  Then 两个文件都被包含在待检查文件列表中

Scenario: resolver 优先查找 .spec.md 继承文件
  Test: test_resolver_prefers_spec_md_over_spec
  Given 目录下同时存在 `project.spec` 和 `project.spec.md`
  When task spec 声明 `inherits: project`
  Then resolver 加载 `project.spec.md`

Scenario: resolver 回退到 .spec 继承文件
  Test: test_resolver_falls_back_to_spec_when_no_spec_md
  Given 目录下只存在 `project.spec`（无 `.spec.md`）
  When task spec 声明 `inherits: project`
  Then resolver 加载 `project.spec`

Scenario: init 默认生成 .spec.md 文件
  Test: test_init_creates_spec_md_by_default
  When 执行 `specwright init --level task --name test-task`
  Then 生成的文件名为 `test-task.spec.md`

Scenario: boundary checker 识别 .spec.md 为 spec 路径
  Test: test_boundary_checker_recognizes_spec_md
  Level: unit
  Given boundary 条目包含 `specs/task.spec.md`
  When boundary checker 判断是否为源码路径
  Then 返回 true

Scenario: 同名 .spec 和 .spec.md 共存时 guard 警告
  Test: test_lint_warns_on_duplicate_spec_extensions
  Given specs 目录下同时存在 `task.spec` 和 `task.spec.md`
  When 执行 guard
  Then 输出包含重复扩展名警告

Scenario: 文件发现使用 file_name 判断而非 Path::extension
  Test: test_spec_md_not_matched_by_extension_alone
  Level: unit
  Given 存在文件 `task.spec.md`
  When 使用 `Path::extension()` 检查
  Then 返回 `"md"` 而非 `"spec"`
  And guard 的 `is_spec_file()` 仍正确识别该文件

Scenario: 非 spec 的 .md 文件不被误识别
  Test: test_plain_md_files_not_matched_as_spec
  Given specs 目录下存在 `notes.md` 和 `task.spec.md`
  When 执行 guard 扫描
  Then `notes.md` 不被包含在待检查文件列表中
  And `task.spec.md` 被包含

Scenario: resolver 找不到继承文件时报错
  Test: test_resolver_errors_when_no_spec_or_spec_md_found
  Given 目录下不存在 `project.spec` 也不存在 `project.spec.md`
  When task spec 声明 `inherits: project`
  Then resolver 返回 InheritanceNotFound 错误
