# agent-spec Integration for Codex / OpenAI Agents

> This file provides Codex with the same guidance that Claude Code gets via `.claude/skills/`.
> Two workflows: **Tool-First** (using the CLI) and **Authoring** (writing .spec/.spec.md files).

---

## Part 1: Tool-First Workflow

### Core Mental Model

**Review point displacement**: Human attention moves from "reading code diffs" to "writing contracts".

```
Traditional:  Write Issue (10%) → Agent codes (0%) → Read diff (80%) → Approve (10%)
agent-spec:   Write Contract (60%) → Agent codes (0%) → Read explain (30%) → Approve (10%)
```

### Quick Reference

| Command | Purpose | When to Use |
|---------|---------|-------------|
| `agent-spec init` | Scaffold new spec | Starting a new task |
| `agent-spec contract <spec>` | Render Task Contract | Before coding - read the execution plan |
| `agent-spec lint <files>` | Spec quality check | After writing spec |
| `agent-spec lifecycle <spec> --code .` | Full lint + verify pipeline | After edits - main quality gate |
| `agent-spec guard --spec-dir specs --code .` | Repo-wide check | Pre-commit / CI - all specs at once |
| `agent-spec explain <spec> --format markdown` | PR-ready review summary | Contract Acceptance |
| `agent-spec explain <spec> --history` | Execution history | See retry count |
| `agent-spec stamp <spec> --dry-run` | Preview git trailers | Traceability |
| `agent-spec verify <spec> --code .` | Raw verification only | Verify without lint gate |
| `agent-spec resolve-ai <spec> --decisions <file>` | Merge AI decisions | Caller mode |

### The Seven-Step Workflow

1. **Human writes Task Contract** — structured spec with Intent, Decisions, Boundaries, Completion Criteria
2. **Quality gate** — `agent-spec lint specs/task.spec --min-score 0.7`
3. **Agent reads Contract** — `agent-spec contract specs/task.spec`
4. **Agent self-checks with lifecycle** — retry loop until all scenarios pass
5. **Guard gate** — `agent-spec guard --spec-dir specs --code .` (pre-commit / CI)
6. **Contract Acceptance** — `agent-spec explain specs/task.spec --format markdown` (human reviews)
7. **Stamp and archive** — `agent-spec stamp specs/task.spec --dry-run`

### Retry Protocol

When `lifecycle` fails:

1. Run: `agent-spec lifecycle <spec> --code . --format json`
2. Parse JSON output, find each scenario's `verdict` and `evidence`
3. For `fail`: the bound test ran and failed — read evidence, fix code
4. For `skip`: test not found — check `Test:` selector matches a real test name
5. For `uncertain`: AI verification pending — review manually or enable AI backend
6. **Fix code based on evidence. Do NOT modify the spec file.**
7. Re-run lifecycle
8. After 3 consecutive failures on the same scenario, stop and escalate to the human

### Verdict Interpretation

| Verdict | Meaning | Action |
|---------|---------|--------|
| `pass` | Scenario verified | No action needed |
| `fail` | Scenario failed verification | Read evidence, fix code |
| `skip` | Test not found or not run | Add missing test or fix selector |
| `uncertain` | AI stub / manual review needed | Review manually or enable AI backend |

**Key rule: `skip` != `pass`**. All four verdicts are distinct.

### Change Set Options

| Flag | Behavior | Default |
|------|----------|---------|
| `--change <path>` | Explicit file/dir for boundary checking | (none) |
| `--change-scope staged` | Git staged files | guard default |
| `--change-scope worktree` | All git working tree changes | (none) |
| `--change-scope jj` | Jujutsu VCS changes | (none) |
| `--change-scope none` | No change detection | lifecycle/verify default |

### AI Verification: Caller Mode

When `--ai-mode caller` is used, the calling Agent acts as the AI verifier:

**Step 1**: `agent-spec lifecycle specs/task.spec --code . --ai-mode caller --format json`
- Output includes `"ai_pending": true` and `"ai_requests_file"` if scenarios need AI review

**Step 2**: Read pending requests, analyze each scenario, write decisions JSON, then merge:
```bash
agent-spec resolve-ai specs/task.spec --code . --decisions decisions.json
```

### Common Errors

| Error | Cause | Solution |
|-------|-------|----------|
| Guard reports N specs failing | Specs have lint or verify issues | Run `lifecycle` on each failing spec |
| `skip` verdict | Test selector doesn't match | Check `Test:` / `Filter:` in spec |
| Quality score below threshold | Lint warnings | Fix vague verbs, add quantifiers |
| Boundary violation | Changed file outside allowed paths | Update Boundaries or revert change |
| Agent keeps failing | Criteria too vague or strict | Improve Completion Criteria |

---

## Part 2: Authoring Workflow

### Spec File Structure

```spec
spec: task           # Level: org, project, task
name: "Task Name"
inherits: project    # Parent spec (optional)
tags: [feature, api]
---

## Intent
One focused paragraph: what to do and why.

## Decisions
- Specific fixed technical choices (tech, version, params)

## Boundaries

### Allowed Changes
- src/module/**
- tests/**

### Forbidden
- Do not add new dependencies
- Do not modify existing public API

## Out of Scope
- Feature X (deferred to next task)

## Completion Criteria

Scenario: Happy path
  Test: test_happy_path
  Given precondition
  When action
  Then expected result

Scenario: Error path 1
  Test: test_error_case
  Given error condition
  When action
  Then error response
```

### Section Reference

| Section | Chinese Header | English Header | Purpose |
|---------|---------------|----------------|---------|
| Intent | `## 意图` | `## Intent` | What to do and why |
| Constraints | `## 约束` | `## Constraints` | Must / Must NOT rules |
| Decisions | `## 已定决策` / `## 决策` | `## Decisions` | Fixed technical choices |
| Boundaries | `## 边界` | `## Boundaries` | Allowed / Forbidden / Out-of-scope |
| Acceptance Criteria | `## 验收标准` / `## 完成条件` | `## Acceptance Criteria` / `## Completion Criteria` | BDD scenarios |
| Out of Scope | `## 排除范围` | `## Out of Scope` | Explicitly excluded items |

### BDD Step Keywords

| English | Chinese | Usage |
|---------|---------|-------|
| `Given` | `假设` | Precondition |
| `When` | `当` | Action |
| `Then` | `那么` | Expected result |
| `And` | `并且` | Additional step |
| `But` | `但是` | Negative additional step |

### Test Selector Patterns

Simple: `Test: test_name`

Structured:
```spec
Test:
  Filter: test_specific_name
```

Chinese equivalents:
```spec
测试: test_name

测试:
  过滤: test_specific_name
```

### Key Authoring Rules

1. **Exception scenarios >= happy path scenarios** — forces edge-case thinking upfront
2. **Every scenario must have a `Test:` selector** — required for mechanical verification
3. **Decisions must be specific** (tech, version, params) — Agent shouldn't choose technology
4. **Boundaries must have path globs** — enables mechanical enforcement
5. **Use deterministic wording** — "returns 201" not "should return 201"
6. **Lint score >= 0.7** before handing to Agent

### Three-Layer Inheritance

```
org.spec(.md) → project.spec(.md) → task.spec(.md)
```

Constraints and decisions inherit downward. Both `.spec` and `.spec.md` extensions are supported; `.spec.md` is preferred for new files (enables Markdown preview in editors and GitHub).

### Conventions

- Task specs live in `specs/`
- Roadmap specs go in `specs/roadmap/`, promote to `specs/` when active
- Verdicts: pass, fail, skip, uncertain — all four are distinct
- **skip ≠ pass**: skipped scenarios block the pipeline
