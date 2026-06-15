# specwright

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

> **Fork of [ZhangHanDong/agent-spec](https://github.com/ZhangHanDong/agent-spec)** — an AI-native BDD/spec verification tool. This fork extends it; see [What this fork adds](#what-this-fork-adds).

`specwright` (*spec* + *-wright*, a "spec-crafter") is an AI-native BDD/spec verification tool: **humans review a contract, agents implement against it, and the machine verifies whether the code satisfies it.** It installs the `specwright` CLI (forked from upstream's `agent-spec` binary).

## What this fork adds

- **Polyglot test runners** — a pluggable `TestRunner` layer with built-ins for **Cargo, Maven, Gradle (Java/Kotlin), Android, iOS, and Node/TypeScript** (Pytest/Go on the roadmap).
- **English-only DSL (v2.0.0, breaking)** — structural keywords, section headers, and selectors are English-only; the parser **hard-rejects Chinese keyword aliases** with a clear error (`keywords must be English; '场景:' is not recognized — use 'Scenario:'`). Description free text (scenario names, step prose, quoted params) may still be any language.
- **No hollow passes** — a test binding that resolves to **zero** tests *fails* instead of silently passing; `skip` and all-`#[ignore]` never count as `pass`.

## How it works (summary)

A **Task Contract** is a spec with four parts:

- `Intent` — what to do, and why
- `Decisions` — technical choices already fixed
- `Boundaries` — what may change, what must not (path entries are mechanically enforced)
- `Completion Criteria` — BDD scenarios with explicit `Test:` bindings → deterministic pass/fail

`contract` is the planning surface; `lifecycle` is the one-command quality gate (lint + verify + report).

## Install

```bash
cargo install --path .
specwright --version   # 2.0.0
```

## Example

```spec
spec: task
name: "User Registration API"
tags: [api, contract]
---

## Intent
Implement a deterministic user registration API an agent can code against.

## Decisions
- Use `POST /api/v1/users/register` as the only public entrypoint
- Persist a user only after password hashing succeeds

## Boundaries
### Allowed Changes
- crates/api/**
### Forbidden
- Do not change the existing login endpoint contract

## Completion Criteria

Scenario: Successful registration
  Test:
    Package: api
    Filter: test_register_api_returns_201_for_new_user
  Given no user with email "alice@example.com" exists
  When the client submits the registration request
  Then the response status is 201
```

Keywords are English-only; description text may be any language. For a non-Cargo project, set `runner: maven | gradle | android | ios | node` in the frontmatter (or let it auto-detect from workspace markers).

## Author and verify

```bash
# scaffold a task contract (add --template rewrite-parity for rewrite/parity tasks)
specwright init --level task --name "User Registration API"

# the main quality gate: lint + verify + report
specwright lifecycle specs/your-task.spec.md --code . --format json

# lint all specs + verify against the current change set
specwright guard

# human-readable contract review (Contract Acceptance — replaces code review)
specwright explain specs/your-task.spec.md --code .
```

### Rewrite/parity contracts

When you are reimplementing existing behavior (a rewrite or a cross-language port),
scaffold with `--template rewrite-parity` and contract the **observable** behavior so
regressions are caught before the code drifts. The worked example
[`examples/rewrite-parity-contract.spec`](examples/rewrite-parity-contract.spec) pins
the two parity axes that rewrites usually break: **command x output mode** (e.g. each
command's human output vs `--json` payload) and **local x remote** (the documented
source lookup order — local source -> cache -> bundled -> remote, including cold start).

## Commands

| Command | Purpose |
|---------|---------|
| `parse` | Parse `.spec`/`.spec.md` files and show the AST |
| `lint` | Analyze spec quality (vague verbs, missing test selectors, coverage gaps) |
| `verify` | Verify code against a single spec |
| `contract` | Render the Task Contract view |
| `plan` | Generate plan context: Contract + codebase scan + Task Sketch |
| `lifecycle` | Run lint + verify + report (the main quality gate) |
| `guard` | Lint all specs and verify against the current change set |
| `explain` | Generate a human-readable contract review summary |
| `stamp` | Preview git trailers for a verified contract (`--dry-run`) |
| `resolve-ai` | Merge external AI decisions into a verification report (caller mode) |
| `checkpoint` | Preview VCS-aware checkpoint status (Git / jj) |
| `graph` | Generate a spec dependency graph (`--format dot` or `svg`) |
| `install-hooks` | Install git hooks for automatic checking |
| `measure-determinism` | [experimental] Measure contract verification variance |
| `brief` | Compatibility alias for `contract` |

## Layout and contributing

- Specs live in `specs/` (future-phase specs staged in `specs/roadmap/`); runnable examples in [`examples/`](examples).
- Agent skills under [`skills/`](skills), including the **tool-first** workflow skill `specwright-tool-first`. For Claude Code, copy them into `.claude/skills/` (they are manual copies, not symlinked); other agents use their own skills directory (for example `~/.codex/skills/`).
- To contribute: write a task contract for your change, implement it, then run `specwright lifecycle` and `specwright guard` before committing.

## License

MIT — same as upstream [ZhangHanDong/agent-spec](https://github.com/ZhangHanDong/agent-spec).
