---
name: agent-spec-authoring
description: |
  CRITICAL: Use for writing and editing agent-spec .spec/.spec.md files. Triggers on:
  write spec, create spec, edit spec, new spec, spec authoring, task contract,
  .spec file, .spec.md file, BDD scenario, acceptance criteria, completion criteria,
  test selector, boundary, constraint, intent, decision, out of scope,
  "how to write a spec", "spec format", "spec syntax", "contract quality"
---

# Agent Spec Authoring

> **Version:** 3.4.0 | **Last Updated:** 2026-05-28

You are an expert at writing agent-spec Task Contracts. Help users by:

- **Creating specs**: scaffold new `.spec.md` files with correct structure (`.spec` is also supported).
- **Editing specs**: improve intent, constraints, boundaries, decisions, and scenarios.
- **Writing scenarios**: BDD-style scenarios with explicit test selectors and deterministic steps.
- **Debugging specs**: fix parse errors, lint warnings, and weak quality scores.
- **Self-hosting**: maintain specs for the agent-spec project itself.
- **Runner-aware contracts**: choose Cargo, Maven, Gradle, Android, or iOS execution semantics in frontmatter.

## Language Boundary

Skill files are reusable assets and must be English-only. Do not include non-English examples in this skill file or its references. The `agent-spec` parser may support non-English aliases, but this skill should describe that support in English and emit English examples unless a vault task explicitly requires Chinese output.

When this skill is used inside the harness workflow:

- vault task specs default to English visible prose and English DSL tokens; use Chinese only when explicitly requested or when Chinese text is the content under test;
- code, comments, tests, CLI strings, skills, templates, and git commit messages remain English-only;
- paths, commands, frontmatter keys, test selectors, runner ids, and code identifiers keep their canonical technical spelling.

## CLI Prerequisite Check

Before running any `agent-spec` command, check:

```bash
command -v agent-spec || cargo install agent-spec
```

If `agent-spec` is not installed, tell the user:

```text
agent-spec CLI not found. Install with: cargo install agent-spec
```

## Core Philosophy

A Contract is not a vague issue. It is a precise specification that moves review effort from reading code diffs to defining correctness:

```text
Traditional:  Human reviews 500 lines of code diff.
agent-spec:   Human writes 50-80 lines of Contract.
              Machine verifies code against Contract.
```

The contract defines what is correct. The lifecycle gate checks whether the code satisfies it.

## Required Self-Check

After writing or editing a spec:

```bash
agent-spec parse specs/task.spec.md
agent-spec lint specs/task.spec.md --min-score 0.7
```

Do not hand a spec to an implementation agent if:

- parse shows zero acceptance scenarios;
- lint reports missing explicit test selectors;
- lint score is below the required threshold.

## Contract Sections

Use the supported top-level sections. Keep one section header per line. Do not combine languages in one heading.

| Section | Purpose |
|---|---|
| `## Intent` | What to do and why. |
| `## Constraints` | Must and must-not rules. |
| `## Decisions` | Fixed technical choices. |
| `## Boundaries` | Allowed changes, forbidden changes, and out-of-scope areas. |
| `## Acceptance Criteria` or `## Completion Criteria` | BDD scenarios and test bindings. |
| `## Out of Scope` | Explicitly excluded work. |

The parser may accept localized aliases, but reusable skills and references must stay English-only.

## Hard Syntax Rules

- Use exactly one supported section header per line.
- Write scenarios as bare DSL lines under the acceptance section.
- Prefer `Scenario:` and `Test:` lines over Markdown-heading compatibility forms.
- Do not invent extra top-level sections such as `## Architecture`, `## Milestones`, or `## Quality` inside a task spec.
- Put architecture notes into `Decisions`, `Boundaries`, or a separate design artifact.
- Always run parse and lint after drafting or editing.

## The Four Elements

### 1. Intent

One focused paragraph. Explain what changes, why it matters, and where it fits.

```spec
## Intent

Add a registration endpoint to the existing authentication module. New users register with email and password, and successful registration sends a verification email. This is the first step in the account system and later work will add login and password reset.
```

Rules:

- Keep it to 2-4 sentences.
- Mention existing context.
- Avoid implementation detail unless it is part of the contract.

### 2. Decisions

Already-decided technical choices. Not options to explore.

```spec
## Decisions

- Route: `POST /api/v1/auth/register`.
- Password hashing: bcrypt with cost factor 12.
- Verification token: `crypto.randomUUID()`, persisted for 24 hours.
- Email: use the existing `EmailService`; do not create a new provider.
```

Rules:

- Include specific technologies, versions, parameters, and compatibility choices.
- Every decision should be covered by at least one scenario when it affects behavior.
- Avoid universal claims unless coverage is proportional.

### 3. Boundaries

Bound what may change and what must not change.

```spec
## Boundaries

### Allowed Changes

- crates/api/src/auth/**
- crates/api/tests/auth/**
- migrations/

### Forbidden

- Do not add new npm or Cargo dependencies.
- Do not change the existing login endpoint.
- Do not create a session during registration.

## Out of Scope

- Login.
- Password reset.
- OAuth login.
```

Rules:

- Path globs are mechanically checked by `BoundariesVerifier`.
- Natural-language prohibitions are linted but not file-path enforced.
- If boundaries list multiple entry points, scenarios should reference each one or explain why shared verification covers them.

### 4. Completion Criteria

Scenarios must be deterministic and test-bound.

```spec
## Completion Criteria

Scenario: Registration succeeds
  Test: test_register_returns_201
  Given no user exists with email "alice@example.com"
  When the client submits a valid registration request
  Then the response status is 201
  And the response body contains "user_id"

Scenario: Duplicate email is rejected
  Test: test_register_rejects_duplicate_email
  Given a user already exists with email "alice@example.com"
  When the client submits a registration request with the same email
  Then the response status is 409

Scenario: Weak password is rejected
  Test: test_register_rejects_weak_password
  Given no user exists with email "bob@example.com"
  When the client submits password "123"
  Then the response status is 400
```

Rules:

- Exception scenarios should be at least as numerous as happy path scenarios.
- Every scenario needs an explicit `Test:` selector.
- Steps should assert observable behavior, not internal implementation shape.

## Runner-Aware Frontmatter

Use `runner` when the task contract must bind scenarios to a non-Cargo execution environment or when auto-detection would be ambiguous.

```spec
---
spec: task
name: "iOS XCTest mini fixture"
runner: ios
runner_config: { scheme: "IosMini", destination: "platform=iOS Simulator,name=iPhone 16 Pro" }
---
```

Built-in runner choices:

| Runner | Use when | Notes |
|---|---|---|
| `cargo` | Rust crates and workspaces | Usually detected from `Cargo.toml`. |
| `maven` | Java/Kotlin Maven projects | Detected from `pom.xml`. |
| `gradle` | Java/Kotlin Gradle projects | Detected from Gradle build files. |
| `android` | Android Gradle projects | Use selector `Level: unit` or `Level: instrumented`. |
| `ios` | Swift Package or Xcode XCTest | macOS only; may need `scheme` and `destination`. |

`runner_config` must use inline map syntax: `{ key: "value" }`. Unknown keys are warnings; review them as likely contract bugs.

## Test Selector Patterns

Simple selector:

```spec
Scenario: Happy path
  Test: test_happy_path
  Given precondition
  When action
  Then result
```

Structured selector:

```spec
Scenario: Cross-crate verification
  Test:
    Package: spec-gateway
    Filter: test_contract_prompt_format
  Given a task spec
  When verified
  Then passes
```

Runner level:

```spec
Scenario: Android instrumented flow
  Test:
    Package: app
    Filter: com.example.PaymentTest#rejectsExpiredCard
    Level: instrumented
  Given an Android project
  When lifecycle verification runs
  Then the instrumented test command is selected
```

## Behavior Surface Checklist

For CLI tools, MCP servers, protocols, and parity rewrites, cover observable surfaces explicitly:

- stdout vs stderr;
- machine-readable output;
- output-file side effects;
- local vs remote behavior;
- warm cache vs cold start;
- fallback and precedence order;
- partial failure vs hard failure;
- persisted state changes.

If these surfaces matter, they belong in scenarios or explicit out-of-scope notes.

## Pseudo-Scenario Rule

Scenarios must describe runtime behavior or externally observable interfaces, not source layout preferences.

Structural anti-patterns:

- "file exists";
- "function is exported";
- "module was split into N files";
- "grep finds a literal string";
- "git log contains a trailer".

Behavioral replacements:

- user-visible UI renders expected content;
- CLI returns the expected status and JSON shape;
- public API returns the expected type or error;
- generated boundary files remain byte-equivalent when that is the public compatibility surface.

Commit trailer checks belong in close discipline, not BDD runtime scenarios.

## Common Errors

| Lint warning | Cause | Fix |
|---|---|---|
| `vague-verb` | vague verbs such as "handle" or "manage" | Use a precise verb such as "validate" or "persist". |
| `unquantified` | unmeasured terms such as "fast" | Add a threshold such as "within 200ms". |
| `testability` | assertion cannot be mechanically verified | Assert observable output or state. |
| `coverage` | constraint has no scenario | Add a scenario that exercises it. |
| `determinism` | non-deterministic wording | Use definitive assertions. |
| `implicit-dep` | missing `Test:` selector | Add `Test:` or a structured selector. |
| `explicit-test-binding` | scenario without a test binding | Bind it to a test, command, or evidence artifact. |
| `sycophancy` | biased bug-finding language | State neutral acceptance criteria. |

## Authoring Checklist

Before handing a Contract to an implementation agent, verify:

| # | Check | Why |
|---|---|---|
| 1 | Intent is 2-4 focused sentences | The agent needs clear direction. |
| 2 | Decisions are specific | The agent should not choose core technology. |
| 3 | Boundaries have path globs | Enables mechanical enforcement. |
| 4 | Exception scenarios cover error paths | Forces edge-case thinking upfront. |
| 5 | Every scenario has a `Test:` selector | Required for mechanical verification. |
| 6 | Steps use deterministic wording | Avoids ambiguous verification. |
| 7 | `agent-spec lint` score is at least 0.7 | Quality gate before execution. |

## Escalation

- **Authoring to planning**: after lint passes, run `agent-spec plan <spec> --code . --format prompt`.
- **Authoring to implementation**: switch to `agent-spec-tool-first` after the contract passes lint.
- **Implementation to authoring**: return here if a scenario, boundary, or decision must change.

Update the Contract first, re-lint, then resume implementation. The Contract is a living document until the task is stamped.
