spec: project
name: "agent-spec 项目规则"
tags: [bootstrap, project]
---

> Historical archive: this is agent-spec's early Chinese-language roadmap project spec, kept for history. The active project contract for harness tasks is `vault/docs/project.spec.md` (English). New specs default to English prose and English DSL tokens; any Chinese DSL below is retained only as legacy parser-compatibility coverage (content under test), not as a recommended authoring style.

## Intent

把 `agent-spec` 做成一个面向新时代 code review 的控制面工具：
人用自然语言写 BDD/Spec，agent 依据 Spec 实现代码，机器依据 Spec 给出可追踪的验证结果。

## Constraints

### Must
- 任务级规格文件存放在 `specs/`
- 公开 CLI 与 gateway 行为必须有回归测试
- DSL 语法变更必须同时更新 AST、解析输出和回归测试
- 验证结果必须区分 `pass`、`fail`、`skip`、`uncertain`
- 任务级完成条件中的每个场景应显式声明 `Test:` selector(英文为默认;中文 `测试:` 仅作 legacy 兼容)
- 任务级边界应支持对显式 change set 的机械验证
- 测试选择器应支持结构化字段，而不仅是裸字符串过滤器
- guard 应支持可选择的 git change scope，而不局限于 staged index
- verify 与 lifecycle 应支持可选的 git change scope，同时保持默认行为稳定
- AI verifier 的 `uncertain` 结果应附带结构化 `AiAnalysis` 证据
- AI verifier 应通过可插拔 backend 接口产生结构化分析结果
- agent-spec 应保持 provider-agnostic，由宿主 agent 注入 AI backend
- 项目应提供 Claude Code 的 project-local skills，且主路径是 tool-first
- 长期路线图 task spec 应暂存于 `specs/roadmap/`，只有提升到顶层 `specs/` 后才进入默认 guard
- Task Contract 应区分 `Must`、`Must Not` 与 `Decisions`
- 默认文本 `contract` 输出应保留结构化 Completion Criteria 细节

### Must Not
- 不要把 `skip` 记为 `pass`
- 不要要求普通磁盘用例手工提供继承搜索路径
- 不要丢弃 BDD 步骤里的结构化输入

## Acceptance Criteria

Scenario: 从磁盘加载任务规格
  Test: test_load_resolves_inherited_constraints_from_spec_directory
  Given `specs/` 中同时存在 `project.spec` 和任务级 `.spec`
  When `SpecGateway::load` 读取该任务级 `.spec`
  Then 计划阶段返回的 Task Contract 中包含继承得到的项目级约束
  And 用户不需要手工提供继承搜索路径

Scenario: 报告验证结果
  Test: test_pass_plus_skip_is_not_passing
  Given 某个验证报告同时包含 `pass` 和 `skip`
  When lifecycle 生成最终决策
  Then 最终决策不会把 `skip` 记为 `pass`
  And 输出继续保留 `pass`、`fail`、`skip`、`uncertain` 这四类 verdict

Scenario: 解析结构化步骤输入
  Test: test_parse_step_table_and_preserve_json_output
  Given 某个 `When` 步骤后跟随表格输入
  When parser 生成 AST 和 JSON 解析输出
  Then AST 与 JSON 中都保留该表格输入
  And 这些表格行不会被拆成新的步骤

Scenario: 校验显式变更集边界
  Test: test_boundaries_verifier_rejects_change_outside_allowed_paths
  Given 某个任务合约声明只允许修改 `src/spec_parser/**`
  When verifier 检查显式变更路径 `crates/spec-gateway/src/lifecycle.rs`
  Then 验证结果为非通过
  And 失败原因指出该路径不在允许边界内

Scenario: Guard 自动推导 staged 变更集
  Test: test_resolve_guard_change_paths_reads_staged_git_changes
  Given 某个临时 git 仓库中存在 staged 文件 `src/lib.rs`
  When `guard` 在未传入 `--change` 的情况下解析 change set
  Then 返回结果包含 `src/lib.rs`
  And 用户不需要手工枚举 change 路径

Scenario: 解析结构化测试选择器
  Test: test_parse_structured_test_selector_block
  Given 某个场景使用 `测试:` 块声明 `包` 和 `过滤`(legacy alias,content under test)
  When parser 生成 AST 和 JSON
  Then 结构化测试选择器字段会被保留
  And 旧的单行 `测试:` 写法继续兼容

Scenario: Guard 支持 worktree 级变更集
  Test: test_resolve_guard_change_paths_reads_worktree_git_changes
  Given 某个临时 git 仓库同时存在 staged、未暂存和未跟踪变更
  When `guard` 使用 `worktree` change scope 解析 change set
  Then 返回结果包含这三类路径
  And 默认 `staged` scope 不会意外纳入未暂存改动

Scenario: Lifecycle 可选接入 git 变更集
  Test: test_resolve_command_change_paths_reads_worktree_git_changes
  Given 某个临时 git 仓库同时存在 staged、未暂存和未跟踪变更
  When `lifecycle` 使用 `worktree` change scope 解析 change set
  Then 返回结果包含这三类路径
  And 默认 `none` scope 保持空 change set

Scenario: AI stub 模式输出 uncertain 与证据
  Test: test_verify_with_ai_mode_stub_marks_uncovered_scenarios_uncertain
  Given 某个任务级 spec 的场景未被机械 verifier 覆盖
  When gateway 使用 `AiMode::Stub` 执行验证
  Then 场景 verdict 为 `uncertain`
  And 结果包含 `AiAnalysis` 证据

Scenario: Stub backend 返回结构化 AI 决策
  Test: test_stub_ai_backend_returns_uncertain_decision
  Given 某个场景被提交给 `StubAiBackend`
  When backend 生成 AI 决策
  Then 返回结构化 `AiDecision`
  And verdict 为 `uncertain`

Scenario: Gateway 支持注入宿主 AI backend
  Test: test_verify_with_injected_ai_backend_uses_host_backend
  Given 某个宿主 agent 提供自定义 `AiBackend`
  When gateway 使用该 backend 执行验证
  Then 验证结果来自该 backend
  And `AiAnalysis` 证据保留 backend 返回的 model 与 reasoning

Scenario: Claude Code tool-first skill 已就位
  Test: test_claude_code_tool_first_skill_exists_and_mentions_contract_lifecycle_guard
  Given 仓库内提供 Claude Code project-local skills
  When 用户查看 tool-first skill
  Then 该 skill 明确指向 `contract`、`lifecycle`、`guard`
  And 把 CLI 路径定义为主集成方式

Scenario: 继承链保留项目级约束与已定决策
  Test: test_load_resolves_full_project_contract_from_spec_directory
  Given `project.spec` 声明了 Constraints 与 Decisions
  When task spec 通过默认继承链加载
  Then Task Contract 包含这些继承得到的规则与已定决策
  And 用户不需要手工提供额外搜索路径

Scenario: contract 输出保留结构化验收信息
  Test: test_contract_output_preserves_step_tables_and_test_selectors
  Given 某个场景带有 step table 与结构化 `测试:` selector
  When CLI 渲染 `agent-spec contract`
  Then 默认输出保留这些结构化信息
  And Agent 主路径不会丢失关键验收上下文

Scenario: 路线图 task specs 使用 staging 目录
  Test: test_roadmap_readme_documents_promotion_rule
  Given 仓库内提供未来阶段的 roadmap task specs
  When 用户查看 `specs/roadmap/README.md`
  Then 文档说明 roadmap specs 暂不进入默认 guard
  And 文档说明它们仍会继承顶层 `specs/project.spec`
