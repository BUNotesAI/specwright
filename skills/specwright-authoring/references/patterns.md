# Specwright Authoring Patterns

This reference is English-only because skill references are reusable assets.

## Section Headers

Use one supported top-level section per line:

| Section | Purpose |
|---|---|
| `## Intent` | What to do and why. |
| `## Constraints` | Must and must-not rules. |
| `## Decisions` | Fixed technical choices. |
| `## Boundaries` | Allowed, forbidden, and out-of-scope changes. |
| `## Acceptance Criteria` | BDD scenarios. |
| `## Completion Criteria` | BDD scenarios. |
| `## Out of Scope` | Explicitly excluded work. |

The parser may support localized aliases, but this reusable reference keeps examples in English.

## Invalid Near Misses

Do not emit combined or invented headings:

```spec
## Intent / Requirements
## Completion Criteria / Done
## Milestones
## Quality
## Architecture
```

Put architecture into Decisions, Boundaries, or a separate design artifact.

## Frontmatter

```spec
spec: task
name: "Add Refund API"
inherits: project
tags: [payment, refund]
runner: cargo # or node, maven, gradle, android, ios
runner_config: {}
---
```

## Complete Example

```spec
spec: task
name: "User Registration API"
inherits: project
runner: cargo
---

## Intent

Add a registration endpoint to the existing authentication module. New users register with email and password, and successful registration sends a verification email.

## Decisions

- Route: `POST /api/v1/auth/register`.
- Password hashing: bcrypt with cost factor 12.
- Verification token: `crypto.randomUUID()`, persisted for 24 hours.
- Email: use the existing `EmailService`.

## Boundaries

### Allowed Changes

- crates/api/src/auth/**
- crates/api/tests/auth/**
- migrations/

### Forbidden

- Do not add new npm or Cargo dependencies.
- Do not change the existing login endpoint.
- Do not create a session during registration.

## Completion Criteria

Scenario: Registration succeeds
  Test: test_register_returns_201_for_new_user
  Given no user exists with email "alice@example.com"
  When the client submits a valid registration request
  Then the response status is 201
  And the response body contains "user_id"
  And `EmailService.sendVerification` was called

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

## Out of Scope

- Login.
- Password reset.
- OAuth login.
```

## Structured Selectors

```spec
Scenario: Cross-crate verification
  Test:
    Package: spec-gateway
    Filter: test_contract_prompt_format
  Given a task spec
  When verified
  Then passes
```

Use `Level` for runners with multiple execution modes:

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

Use `runner: node` for TypeScript and JavaScript package-script projects. TanStack Start, Vite, Vitest, Jest, Playwright, and Bun projects all use the generic Node runner rather than framework-specific runner ids.

```spec
spec: task
name: "TypeScript package-script checks"
inherits: project
runner: node
runner_config: { package_manager: "pnpm", unit_filter_style: "vitest" }
---

## Intent

Verify the TypeScript project through its package scripts.

## Completion Criteria

Scenario: Unit test passes
  Test:
    Filter: renders settings page
    Level: unit
  Given the project has a package.json test script
  When lifecycle verification runs
  Then the selected package manager runs the unit test script

Scenario: Typecheck passes
  Test:
    Filter: -
    Level: typecheck
  Given the project has a package.json typecheck script
  When lifecycle verification runs
  Then the selected package manager runs the typecheck script
```

Node v1 does not support `Package` selectors or `runner_config.workspace_filter`; use separate specs or script-level filtering for monorepos.

## Step Tables

Use tables for structured input:

```spec
Scenario: Batch validation
  Test: test_batch_validation
  Given the following input records:
    | name  | email           | valid |
    | Alice | alice@test.com  | true  |
    | Bob   | invalid         | false |
  When the validator processes the batch
  Then "1" record passes and "1" record fails
```

## Behavior Not Structure

Good scenarios verify observable behavior:

- CLI output and exit status;
- API response shape;
- UI text visible to the user;
- generated public boundary files;
- persisted state changes.

Poor scenarios verify implementation preferences:

- file count;
- helper function names;
- source grep hits;
- module split shape;
- commit log content.

Move author preferences into design documents, Decisions prose, or close review checklists instead of BDD scenarios.

## Lint Rules

| Rule | Trigger | Fix |
|---|---|---|
| `vague-verb` | vague verbs | Use precise verbs such as "validate" or "persist". |
| `unquantified` | broad performance claims | Add numbers. |
| `testability` | unobservable assertions | Assert output, status, or state. |
| `coverage` | uncovered constraints | Add a scenario. |
| `determinism` | non-definitive wording | Use definitive assertions. |
| `implicit-dep` | missing `Test:` selector | Add a selector. |
| `explicit-test-binding` | scenario without a binding | Bind it to test, command, or evidence. |
| `sycophancy` | biased bug-finding language | State neutral criteria. |
