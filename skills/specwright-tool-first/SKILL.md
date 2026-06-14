---
name: specwright-tool-first
description: |
  CRITICAL: Use for specwright CLI tool workflow. Triggers on:
  specwright, contract, lifecycle, guard, verify, explain, stamp, checkpoint, plan,
  spec verification, task contract, spec quality, lint spec, run log,
  "how to verify", "how to use specwright", "spec failed", "guard failed",
  contract review, contract acceptance, PR review, code review workflow,
  plan context, codebase scan, task sketch, implementation plan
---

# Specwright Tool-First Workflow

> **Version:** 3.4.0 | **Last Updated:** 2026-05-31

You are an expert at using `specwright` as a CLI tool for contract-driven AI coding. Help users by:
- **Planning**: Render task contracts with `contract`, generate plan context with `plan`
- **Implementing**: Follow contract Intent, Decisions, Boundaries
- **Verifying**: Run `lifecycle` / `guard` to check code against specs
- **Reviewing**: Use `explain` for human-readable summaries, `stamp` for git trailers
- **Debugging**: Interpret verification failures and fix code accordingly
- **Runner-aware workflows**: Select Cargo, Maven, Gradle, Android, iOS, or Node/TypeScript runners through spec frontmatter or CLI overrides

## IMPORTANT: CLI Prerequisite Check

**Before running any `specwright` command, Claude MUST check:**

```bash
command -v specwright || cargo install specwright
```

If `specwright` is not installed, inform the user:
> `specwright` CLI not found. Install with: `cargo install specwright`

## Core Mental Model

**The key shift**: Review point displacement. Human attention moves from "reading code diffs" to "writing contracts".

```
Traditional:  Write Issue (10%) → Agent codes (0%) → Read diff (80%) → Approve (10%)
specwright:   Write Contract (60%) → Agent codes (0%) → Read explain (30%) → Approve (10%)
```

Humans define "what is correct" (Contract). Machines verify "is the code correct" (lifecycle). Humans do final "Contract Acceptance" — not Code Review.

## Language Boundary

Skill files and skill references are reusable assets and must be English-only. Use English examples in this skill. When this skill is used inside the harness workflow, specwright task `spec.md` defaults to English (visible prose and DSL tokens); Chinese only on explicit request or as content under test. Code, comments, tests, CLI strings, skills, templates, and git commit messages remain English-only.

## Quick Reference

| Command | Purpose | When to Use |
|---------|---------|-------------|
| `specwright init` | Scaffold new spec | Starting a new task |
| `specwright contract <spec>` | Render Task Contract | Before coding - read the execution plan |
| `specwright plan <spec> --code .` | Generate plan context | Before coding - codebase scan + task sketch |
| `specwright lint <files>` | Spec quality check | After writing spec, before giving to Agent |
| `specwright lifecycle <spec> --code .` | Full lint + verify pipeline | After edits - main quality gate |
| `specwright guard --spec-dir specs --code .` | Repo-wide check | Pre-commit / CI - all specs at once |
| `specwright explain <spec> --format markdown` | PR-ready review summary | Contract Acceptance - paste into PR |
| `specwright explain <spec> --history` | Execution history | See how many retries the Agent needed |
| `specwright stamp <spec> --dry-run` | Preview git trailers | Before committing - traceability |
| `specwright verify <spec> --code .` | Raw verification only | When you want verify without lint gate |
| `specwright checkpoint status` | VCS-aware status | Check uncommitted state |

## Runner-Aware Verification

`specwright` can execute task scenarios through built-in language runners. Prefer spec frontmatter when the contract owns the runner choice, and use CLI overrides only for local diagnosis or one-off verification.

```spec
spec: task
name: "iOS XCTest fixture"
runner: ios
runner_config: { scheme: "IosMini", destination: "platform=iOS Simulator,name=iPhone 16 Pro" }
---
```

Built-in runners:

| Runner | Detection markers | Test command shape | Notes |
|---|---|---|---|
| `cargo` | `Cargo.toml` | `cargo test -q [-p <package>] <filter>` | Default Rust path. Default Cargo JSON remains byte-equivalent unless warnings or runner overrides are present. |
| `maven` | `pom.xml`, prefers `mvnw` when present | `mvn test [-pl <package>] -Dtest=<filter>` | Uses JVM binding scan for Java/Kotlin scenarios. |
| `gradle` | `build.gradle` / `build.gradle.kts`, prefers `gradlew` when present | `gradle :<package>:test --tests <filter>` | Maven/Gradle mixed workspaces require exactly one wrapper family or an explicit runner. |
| `android` | `AndroidManifest.xml` plus Gradle markers | Gradle unit or instrumented task selected by `Test.level` | `level: instrumented` requires ADB and a connected device/emulator; missing capability becomes Skip. |
| `ios` | `Package.swift` or `*.xcodeproj` | `xcodebuild test -scheme <scheme> -destination <destination> -only-testing:<package>/<filter>` | macOS only. Requires Xcode and a booted iOS Simulator; missing capability becomes Skip. |
| `node` | `package.json` | `<package-manager> run <script> [filter args]` | Generic JavaScript/TypeScript package-script runner. There is no TanStack Start-specific runner; TanStack Start projects use `runner: node`. |

Structured selectors can include `Package`, `Filter`, and `Level`:

```spec
Scenario: Android instrumented flow
  Test:
    Package: app
    Filter: com.example.PaymentTest#rejectsExpiredCard
    Level: instrumented
```

Known runner config keys:

| Runner | Keys |
|---|---|
| `ios` | `scheme`, `destination` |
| `node` | `package_manager`, `unit_script`, `typecheck_script`, `lint_script`, `build_script`, `e2e_script`, `unit_filter_style`, `workspace_filter` |

Unknown `runner_config` keys are non-blocking warnings in the verification context. Treat spelling mistakes such as `destinaiton` as review findings even when the lifecycle status still passes.

Node/TypeScript runner v1 behavior:

- Use `runner: node` for TypeScript, JavaScript, Vite, Vitest, Jest, Playwright, Bun, and TanStack Start package-script verification. Do not use framework-specific runner ids such as `vitest`, `jest`, `playwright`, or `tanstack-start`; unknown framework ids should be corrected to `runner: node`.
- Package manager precedence is `runner_config.package_manager` > `package.json.packageManager` > a single lockfile marker > `npm`. Supported values are `npm`, `pnpm`, `yarn`, and `bun`; invalid values and multiple lockfiles without a package-manager decision fail verification.
- Script mapping defaults are `Level: unit` -> `test`, `typecheck` -> `typecheck`, `lint` -> `lint`, `build` -> `build`, and `e2e` -> `e2e`. Override these with the corresponding `*_script` runner config key.
- Unit filters require `runner_config.unit_filter_style`: `vitest` emits `-- -t <escaped-filter>`, `jest` emits `-- --testNamePattern <escaped-filter>`, `playwright` emits `-- --grep <escaped-filter>`, and `none` requires `Filter: -`.
- Non-unit levels (`typecheck`, `lint`, `build`, `e2e`) require the no-filter sentinel `Filter: -`.
- `Package` selectors and `runner_config.workspace_filter` are out of scope for Node v1 and fail loudly. Use separate specs or script-level filtering for monorepos.
- Missing package-manager executables on `PATH` are converted to Skip through `MissingCapability`. Missing or unreadable `package.json`, missing required scripts, and invalid runner config fail verification.
- `Level: e2e` is opt-in and reports a browser capability skip by default; it is not part of the default close gate.

## Documentation

Refer to the local files for detailed command patterns:
- `./references/commands.md` - Complete CLI command reference with all flags

## IMPORTANT: Documentation Completeness Check

**Before answering questions, Claude MUST:**
1. Read `./references/commands.md` for exact command syntax
2. If file read fails: Inform user "references/commands.md is missing, answering from SKILL.md patterns"
3. Still answer based on SKILL.md patterns + built-in knowledge

## The Seven-Step Workflow

### Step 1: Human writes Task Contract (human attention: 60%)

Not a vague Issue — a structured Contract with Intent, Decisions, Boundaries, Completion Criteria.

```bash
specwright init --level task --lang en --name "User Registration API"
# Then fill in the four elements in the generated .spec.md file
```

For rewrite, migration, or parity tasks, prefer the parity-aware scaffold:

```bash
specwright init --level task --template rewrite-parity --lang en --name "CLI Parity Contract"
```

**Key principle**: Exception scenarios >= happy path scenarios. 1 happy + 3 error paths forces you to think through edge cases before coding begins.

### Step 2: Contract quality gate

Check Contract quality before handing to Agent. Like "code review" but for the Contract itself.

```bash
specwright parse specs/user-registration.spec
specwright lint specs/user-registration.spec --min-score 0.7
```

Catches: malformed structure, zero-scenario acceptance sections, vague verbs, unquantified constraints, non-deterministic wording, missing test selectors, sycophancy bias, uncovered constraints, uncovered decisions (decision-coverage), unbound observable behavior decisions (observable-decision-coverage), uncovered output modes (output-mode-coverage), unverified precedence/fallback chains (precedence-fallback-coverage), weak mock-only I/O error scenarios (external-io-error-strength), missing verification-strength metadata on I/O scenarios (verification-metadata-suggestion), missing error paths (error-path), universal claims with insufficient scenarios (universal-claim), boundary entry points without matching scenarios (boundary-entry-point), untested flag combinations (flag-combination-coverage), untagged platform-specific decisions (platform-decision-tag).

**Required self-checks before coding:**
- `specwright parse` must show the expected section count and a non-zero scenario count for task specs.
- If `Acceptance Criteria: 0 scenarios` appears, stop and rewrite the spec before running `contract` or `lifecycle`.
- The parser accepts Markdown-heading forms like `### Scenario:` and `### Test:` for compatibility, but authoring should still emit bare `Scenario:` and `Test:` lines by default. Do not invent extra top-level sections like `## Milestones`.

**Unbound Observable Behavior review:**
- After `parse + lint`, ask which stdout, stderr, file, network, cache, and persisted-state behaviors are still unbound.
- If the task is a rewrite, migration, or parity effort, also ask whether the contract covers:
  - command x output mode
  - local x remote
  - warm cache x cold start
  - fallback / precedence order
  - partial failure vs hard failure
- If any of these surfaces are still only described in prose, switch back to authoring mode and add scenarios before coding.

Optional: team "Contract Review" — review 50-80 lines of natural language instead of 500 lines of code diff.

### Step 3: Agent reads Contract, generates plan, and codes

Agent consumes the structured contract and generates plan context:

```bash
# Read the contract
specwright contract specs/user-registration.spec

# Generate plan context with codebase scan
specwright plan specs/user-registration.spec --code . --format prompt
```

The `plan` command outputs three blocks:
- **Contract** — the full task contract (same as `contract` command)
- **Codebase Context** — files in Allowed Changes paths with summaries, pub signatures, and test function names
- **Task Sketch** — scenarios grouped by dependency order for implementation sequencing

Use `--format prompt` to get a self-contained prompt for AI plan generation. Use `--depth full` to include pub API signatures.

Agent is triple-constrained:
- **Decisions** tell it "how to do it" (no technology shopping)
- **Boundaries** tell it "what to touch" (no unauthorized file changes)
- **Completion Criteria** tell it "when it's done" (all bound tests must pass)

### Step 4: Agent self-checks with lifecycle (automatic retry loop)

```bash
specwright lifecycle specs/user-registration.spec \
  --code . --change-scope worktree --format json --run-log-dir .specwright/runs
```

Four verification layers run in sequence:
1. **lint** — re-check Contract quality (prevent spec tampering)
2. **StructuralVerifier** — pattern match Must NOT constraints against code
3. **BoundariesVerifier** — check changed files are within Allowed Changes
4. **TestVerifier** — execute tests bound to each scenario

```
Agent retry loop (no human needed):
  Code → lifecycle → FAIL (2/5) → read failure_summary → fix → lifecycle → FAIL (4/5) → fix → lifecycle → PASS (5/5) ✓
```

Run logs record this history — "this Contract took 3 tries to pass".

#### The Iron Law

```
NO CODE IS "DONE" WITHOUT A PASSING LIFECYCLE
```

If lifecycle hasn't run in this session, you cannot claim completion. If lifecycle ran but had failures, code is not done. No exceptions.

#### Retry Protocol

When lifecycle fails, follow this exact sequence:

1. Run: `specwright lifecycle <spec> --code . --format json`
2. Parse JSON output, find each scenario's `verdict` and `evidence`
3. For `fail`: the bound test ran and failed — read evidence to understand why, fix code
4. For `skip`: the bound test was not found — check `Test:` selector matches a real test name
5. For `uncertain`: AI verification pending — review manually or enable AI backend
6. **Fix code based on evidence. Do NOT modify the spec file** — changing the Contract to make verification pass is sycophancy, not a fix
7. Re-run lifecycle
8. After 3 consecutive failures on the same scenario, stop and escalate to the human

**Critical rule**: The spec defines "what is correct". If the code doesn't match, fix the code. If the spec itself is wrong, switch to authoring mode and update the Contract explicitly — never silently weaken acceptance criteria.

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

### Step 5: Guard gate (pre-commit / CI)

```bash
# Pre-commit hook
specwright guard --spec-dir specs --code . --change-scope staged

# CI (GitHub Actions)
specwright guard --spec-dir specs --code . --change-scope worktree
```

Runs lint + verify on ALL specs against current changes. Blocks commit/PR if any spec fails.

### Step 6: Contract Acceptance replaces Code Review (human attention: 30%)

Human reviews a Contract-level summary, not a code diff:

```bash
specwright explain specs/user-registration.spec --code . --format markdown
```

**Evidence gate**: Before presenting results to the reviewer, run `specwright explain <spec> --format markdown` fresh. Read the output. Confirm all verdicts are `pass`. Do NOT report results from memory — run the command and read the output in this session.

Reviewer judges two questions:
1. **Is the Contract definition correct?** (Intent, Decisions, Boundaries make sense?)
2. **Did all verifications pass?** (4/4 pass including error paths?)

If both "yes" → approve. This is 10x faster than reading code diffs.

Check retry history if needed:

```bash
specwright explain specs/user-registration.spec --code . --history
```

#### Assisting Contract Acceptance

When helping a human review a completed task:

1. Run `specwright explain <spec> --code . --format markdown` and present the output
2. If human asks about retry history: run with `--history` flag
3. If human asks about specific failures: run `specwright lifecycle <spec> --code . --format json` and extract the relevant scenario results
4. If human approves: run `specwright stamp <spec> --code . --dry-run` and present the trailers

### Step 7: Stamp and archive

```bash
specwright stamp specs/user-registration.spec --dry-run
# Output: Spec-Name: User Registration API
#         Spec-Passing: true
#         Spec-Summary: 4/4 passed, 0 failed, 0 skipped, 0 uncertain
```

Establishes Contract → Commit traceability chain.

## Verdict Interpretation

| Verdict | Meaning | Action |
|---------|---------|--------|
| `pass` | Scenario verified | No action needed |
| `fail` | Scenario failed verification | Read evidence, fix code |
| `skip` | Test not found or not run | Add missing test or fix selector |
| `uncertain` | AI stub / manual review needed | Review manually or enable AI backend |

**Key rule: `skip` != `pass`**. All four verdicts are distinct.

## VCS Awareness

specwright auto-detects the VCS from the project root. Behavior differs between git and jj:

| Condition | Behavior |
|-----------|----------|
| `.jj/` exists (even with `.git/`) | Use `--change-scope jj` instead of `worktree` |
| jj repo | Do NOT run `git add` or `git commit` — jj auto-snapshots all changes |
| jj repo | `stamp` output includes `Spec-Change:` trailer with jj change ID |
| jj repo | `explain --history` shows file-level diffs between runs (via operation IDs) |
| Only `.git/` | Use standard git commands (`--change-scope staged` or `worktree`) |
| Neither | Change scope detection unavailable; use `--change <path>` explicitly |

## Change Set Options

| Flag | Behavior | Default |
|------|----------|---------|
| `--change <path>` | Explicit file/dir for boundary checking | (none) |
| `--change-scope staged` | Git staged files | guard default |
| `--change-scope worktree` | All git working tree changes | (none) |
| `--change-scope jj` | Jujutsu VCS changes | (none) |
| `--change-scope none` | No change detection | lifecycle/verify default |

## Advanced Features

### Verification Layers

```bash
# Run only specific layers
specwright lifecycle specs/task.spec --code . --layers lint,boundary,test
# Available: lint, boundary, test, ai
```

### Run Logging

```bash
specwright lifecycle specs/task.spec --code . --run-log-dir .specwright/runs
specwright explain specs/task.spec --history
```

### AI Mode

```bash
specwright verify specs/task.spec --code . --ai-mode off      # default - no AI
specwright verify specs/task.spec --code . --ai-mode stub      # testing only
specwright lifecycle specs/task.spec --code . --ai-mode caller # agent-as-verifier
```

### AI Verification: Caller Mode

When `--ai-mode caller` is used, the calling Agent acts as the AI verifier. This is a two-step protocol:

**Step 1: Emit AI requests**

```bash
specwright lifecycle specs/task.spec --code . --ai-mode caller --format json
```

If any scenarios are skipped (no mechanical verifier covered them), the output JSON includes:
- `"ai_pending": true`
- `"ai_requests_file": ".specwright/pending-ai-requests.json"`

The pending requests file contains `AiRequest` objects with scenario context, code paths, contract intent, and constraints.

**Step 2: Resolve with external decisions**

The Agent reads the pending requests, analyzes each scenario, then writes decisions:

```json
[
  {
    "scenario_name": "scenario name",
    "model": "claude-agent",
    "confidence": 0.92,
    "verdict": "pass",
    "reasoning": "All steps verified by code analysis"
  }
]
```

Then merges them back:

```bash
specwright resolve-ai specs/task.spec --code . --decisions decisions.json
```

This produces a final merged report where Skip verdicts are replaced with the Agent's AI decisions.

**When to use caller mode:**
- When the calling Agent (Claude, Codex, etc.) can read and reason about code
- For scenarios that can't be verified by tests alone (design intent, code quality)
- When you want the Agent to be both implementor and verifier

## When to Use / When NOT to Use

| Scenario | Use specwright? | Why |
|----------|----------------|-----|
| Clear feature with defined inputs/outputs | Yes | Contract can express deterministic acceptance criteria |
| Bug fix with reproducible steps | Yes | Great for "given bug X, when fixed, then Y" |
| Exploratory prototyping | No | You don't know "what is done" yet - vibe code first |
| Large architecture refactor | No | Boundaries hard to define, "better architecture" isn't testable |
| Security/compliance rules | Yes (org.spec) | Encode rules once, enforce mechanically everywhere |

### Gradual Adoption

```
Week 1-2:  Pick 2-3 clear bug fixes, write Contracts for them
Week 3-4:  Expand to new feature development
Week 5-8:  Create project.spec with team coding standards
Month 3+:  Consider org.spec for cross-project governance
```

## Common Errors

| Error | Cause | Solution |
|-------|-------|----------|
| Guard reports N specs failing | Specs have lint or verify issues | Run `lifecycle` on each failing spec individually |
| `skip` verdict on scenario | Test selector doesn't match any test | Check `Test:` / `Package:` / `Filter:` in spec |
| Quality score below threshold | Too many lint warnings | Fix vague verbs, add quantifiers, improve testability |
| Boundary violation detected | Changed file outside allowed paths | Either update Boundaries or revert the change |
| `uncertain` on all AI scenarios | Using `--ai-mode stub` or no backend | Expected — review manually |
| Agent keeps failing lifecycle | Contract criteria too vague or too strict | Improve Completion Criteria specificity |

## Command Priority

| Preference | Use | Instead of |
|------------|-----|------------|
| `contract` | Render task contract | `brief` (legacy alias) |
| `plan` | Contract + codebase + sketch | Manual code exploration |
| `lifecycle` | Full pipeline | `verify` alone (misses lint) |
| `guard` | Repo-wide | Multiple individual `lifecycle` calls |
| `--change` | Explicit paths known | `--change-scope` when paths are known |
| CLI commands | Tool-first approach | `spec-gateway` library API |

## When to Switch to Authoring Mode

During implementation, if you discover:
- A missing exception path that should be in Completion Criteria
- A Boundary that's too restrictive (need to modify more files than allowed)
- A Decision that needs to change (technology choice was wrong)

Switch to `specwright-authoring` skill, update the Contract FIRST, re-run `specwright lint` to validate the change, then resume implementation. Do NOT silently work outside the Contract's boundaries.

## Escalation

Switch to library integration only when:
- Embedding `specwright` into another Rust agent runtime
- Testing `spec-gateway` internals
- Injecting a host `AiBackend` via `verify_with_backend(Arc<dyn AiBackend>)`
