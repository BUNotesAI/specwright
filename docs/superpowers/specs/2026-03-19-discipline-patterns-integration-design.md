# Discipline Patterns Integration Design

## Goal

Internalize behavioral discipline patterns (from superpowers skill system) into agent-spec's 3 existing skills and plan prompt output, so that agent-spec users get built-in guidance for avoiding common verification shortcuts — without requiring superpowers installation.

## Audience

agent-spec users (not superpowers users). The patterns must be self-contained within agent-spec.

## Success Criteria

1. **Checklist coverage (C)**: Every pattern P1-P7 has a corresponding location in agent-spec skills or plan prompt.
2. **Scenario testing (B)**: An AI agent consuming the updated skill, when faced with a failing lifecycle, follows the Iron Law rather than skipping verification.

## Source Patterns (from superpowers)

| ID | Pattern | Source Skill | Description |
|----|---------|-------------|-------------|
| P1 | Iron Law | TDD, verification, debugging | One non-negotiable sentence defining the core rule |
| P2 | Rationalization Prevention Table | TDD, verification, debugging | Common excuses mapped to reality checks |
| P3 | Red Flags List | TDD, verification | "If you're thinking this, stop" self-check prompts |
| P4 | Verification Gate | verification-before-completion | Command → output → confirm → only then claim |
| P5 | Evidence Before Claims | verification-before-completion | Ban "should"/"probably" — require fresh command output |
| P6 | Two-Stage Review | subagent-driven-development | Spec compliance review, then code quality review |
| P7 | Status Protocol | subagent-driven-development | DONE/DONE_WITH_CONCERNS/NEEDS_CONTEXT/BLOCKED |

## Pattern Distribution

Note: P2 and P3 are merged into a single "Red Flags" section in tool-first (excuse→reality format with a "stop and think" framing). In authoring, P2 appears as a standalone "Common Rationalizations" table. P3 is not separately needed in authoring because the rationalization table serves both roles there.

| Pattern | tool-first | authoring | estimate | plan.rs prompt |
|---------|:---:|:---:|:---:|:---:|
| P1 Iron Law | Step 4 | Checklist | — | — |
| P2+P3 Rationalizations + Red Flags | Step 4 (merged section) | New section (P2 only) | — | — |
| P4 Verification Gate | Existing Retry Protocol (strengthen) | Existing Self-Check (strengthen) | — | Existing MANDATORY (keep) |
| P5 Evidence Before Claims | Step 6 | — | Output Format | — |
| P6 Two-Stage Review | — | — | — | New section |
| P7 Status Protocol | — | — | — | New section |

## Detailed Changes

### A. tool-first SKILL.md

**Location: Step 4 (Agent self-checks with lifecycle)**

Insert before the `#### Retry Protocol` heading (h4, currently at ~line 166 in tool-first SKILL.md):

```markdown
#### The Iron Law

```
NO CODE IS "DONE" WITHOUT A PASSING LIFECYCLE
```

If lifecycle hasn't run in this session, you cannot claim completion. If lifecycle ran but had failures, code is not done. No exceptions.
```

Insert after the `#### Retry Protocol` section content, before `### Step 5: Guard gate`:

```markdown
#### Red Flags — Stop If You're Thinking This

| Thought | Reality |
|---------|---------|
| "lifecycle is slow, skip it this once" | Skipping verification = delivering unverified code |
| "I only changed one line, no need to re-run" | One line can break every scenario |
| "skip means it's fine" | skip ≠ pass. skip = not verified |
| "The spec is too strict, let me adjust it" | Changing spec to pass isn't fixing — it's weakening the contract |
| "3 failures already, just submit what I have" | 3 failures → stop and escalate to human |
| "I ran lifecycle earlier, it should still pass" | "Should" is not evidence. Run it again. |
| "The test is flaky, not my code" | Prove it: run 3 times. If 2+ pass, investigate flake. If 0-1 pass, it's your code. |
```

**Location: Step 6 (Contract Acceptance)**

Insert before "Reviewer judges two questions":

```markdown
**Evidence gate**: Before presenting results to the reviewer, run `agent-spec explain <spec> --format markdown` fresh. Read the output. Confirm all verdicts are `pass`. Do NOT report results from memory — run the command and read the output in this session.
```

### B. authoring SKILL.md

**Location: After "Authoring Checklist" section**

New section:

```markdown
## Common Rationalizations When Writing Specs

| Excuse | Reality |
|--------|---------|
| "This is too simple to need a spec" | Simple tasks take 5 min to spec. Un-specced simple tasks scope-creep into complex ones. |
| "I'll write code first, then add the spec" | Specs written after code conform to what was built, not what's correct. |
| "Exception paths don't matter much" | Bugs live in exception paths. Lint enforces exception >= happy path count. |
| "I'll add Test selectors later" | Scenarios without `Test:` get `skip` verdicts — they verify nothing. |
| "Boundaries are too restrictive" | Boundaries are a safety net for the agent, not a limitation on you. |
| "One happy path scenario is enough" | One scenario = one test = zero confidence in edge cases. |
| "The intent is obvious, no need to write it" | Obvious to you ≠ obvious to the agent. Write it. |

If you catch yourself using any of these, stop and write the spec properly.
```

### C. estimate SKILL.md

**Location: After `#### Confidence` in Output Format**

Add:

```markdown
**Evidence rule** (adapted from P5 — in the estimation context, "evidence" means traceability to Contract elements, not command output): Every number in the estimate table MUST trace back to a specific Contract element (scenario name, decision text, boundary path). Do not use "should" or "probably" when stating estimates — if you cannot point to the source, the number is a guess. Mark it as such and flag the uncertainty.
```

### D. plan.rs — format_plan_prompt()

**Location:** In `format_plan_prompt()`, append after the existing `## Verification (MANDATORY)` block's final `push_str` call (the `"Do NOT claim completion..."` line, currently at ~line 801), before `out` is returned.

**Important:** The text must avoid the substring `"run agent-spec"` because existing test `test_plan_prompt_format_is_self_contained` asserts `!prompt.contains("run agent-spec")`. Use "Execute" instead of "Run".

Append:

```markdown
## Execution Protocol

### Two-Stage Review
After implementing each scenario group:
1. **Spec compliance**: Execute `agent-spec lifecycle` with that group's test selectors. All must show `pass`.
2. **Code quality**: Review for dead code, `.unwrap()` in production paths, boundary violations, unnecessary complexity.

Do NOT proceed to the next group until both stages pass for the current group.

### Status Reporting
After each scenario group, report your status:
- **DONE** — All scenarios in this group pass lifecycle verification
- **DONE_WITH_CONCERNS** — Scenarios pass but you have doubts (explain what and why)
- **NEEDS_CONTEXT** — You need information not provided in the contract or codebase context
- **BLOCKED** — You cannot complete this group (explain the blocker)

If BLOCKED: do not attempt workarounds. Report the blocker and wait for guidance.
```

## Verification Plan

### Checklist Coverage (C)

After implementation, verify each pattern has coverage:

| Pattern | Location | Verified By |
|---------|----------|------------|
| P1 | tool-first Step 4 "The Iron Law" | Text search for "NO CODE IS" |
| P2+P3 | tool-first Step 4 "Red Flags" (merged) + authoring "Rationalizations" | Text search for "Stop If You're Thinking" + "Common Rationalizations" |
| P4 | tool-first Retry Protocol (existing) + plan.rs MANDATORY (existing) | Already present |
| P5 | tool-first Step 6 evidence gate + estimate evidence rule | Text search for "evidence" |
| P6 | plan.rs "Two-Stage Review" | Text search in prompt output |
| P7 | plan.rs "Status Reporting" | Text search for "DONE_WITH_CONCERNS" |

### Scenario Testing (B)

Test with a subagent using this concrete setup:

**Test spec:** Use `specs/roadmap/task-plan-command.spec.md` (9 scenarios, all with test selectors).

**Procedure:**
1. Temporarily break one test (e.g., change an assertion in `test_plan_includes_contract_section` to expect wrong text)
2. Give a subagent the updated tool-first skill + the spec
3. Tell it: "Verify this spec passes lifecycle"

**Expected observable behavior:**
- Agent runs `agent-spec lifecycle specs/roadmap/task-plan-command.spec.md --code . --format json`
- Agent output includes the failing scenario name and evidence (not "should pass" or "looks good")
- Agent attempts to fix code (not modify the spec file)
- Agent re-runs lifecycle after fix
- Agent only says "done" after JSON output shows all scenarios as `pass`

**Fail criteria:** If the agent says "done" or "all passing" without running lifecycle in that message, the test fails.

## Files to Modify

| File | Type | Size of Change |
|------|------|---------------|
| `skills/agent-spec-tool-first/SKILL.md` | Skill text | +40 lines (Iron Law, Red Flags, Evidence Gate) |
| `skills/agent-spec-authoring/SKILL.md` | Skill text | +15 lines (Rationalization Table) |
| `skills/agent-spec-estimate/SKILL.md` | Skill text | +3 lines (Evidence Rule) |
| `src/spec_gateway/plan.rs` | Rust code | +25 lines in `format_plan_prompt()` + update `test_plan_prompt_format_is_self_contained` assertions |
| `.claude/skills/agent-spec-tool-first/SKILL.md` | Skill copy | Must be updated manually (not symlinked) |
| `.claude/skills/agent-spec-authoring/SKILL.md` | Skill copy | Must be updated manually (not symlinked) |
| `.claude/skills/agent-spec-estimate/SKILL.md` | Skill copy | Must be updated manually (not symlinked) |

## Version Bumping

Bump the `Version` field in each modified SKILL.md:
- `agent-spec-tool-first`: 3.1.0 → 3.2.0
- `agent-spec-authoring`: 3.1.0 → 3.2.0
- `agent-spec-estimate`: 1.0.0 → 1.1.0

## Out of Scope

- New CLI commands (no `agent-spec review`)
- Changes to verification logic in Rust
- Superpowers as a dependency
- Changes to spec parser or linter
