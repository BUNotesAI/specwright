# specwright

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

> **Fork of [ZhangHanDong/agent-spec](https://github.com/ZhangHanDong/agent-spec)** — an AI-native BDD/spec verification tool. This fork extends it; see [What this fork adds](#what-this-fork-adds).

`specwright` (*spec* + *-wright*, a "spec-crafter") is an AI-native BDD/spec verification tool: **humans review a contract, agents implement against it, and the machine verifies whether the code satisfies it.** It installs the `specwright` CLI (forked from upstream's `agent-spec` binary).

## What this fork adds

- **Polyglot test runners** — a pluggable `TestRunner` layer with built-ins for **Cargo, Maven, Gradle (Java/Kotlin), Android, iOS, Node/TypeScript, and CMake/CTest** (Pytest/Go on the roadmap).
- **English-only DSL (v2.0.0, breaking)** — structural keywords, section headers, and selectors are English-only; the parser **hard-rejects Chinese keyword aliases** with a clear error (`keywords must be English; '场景:' is not recognized — use 'Scenario:'`). Description free text (scenario names, step prose, quoted params) may still be any language.
- **Declarative mixed-runner routing (v2.1.0)** — one task spec can keep Cargo as the default runner while routing selected `Package:` tokens to another runner such as Node/TypeScript.
- **External verification (v2.2.0)** — scenarios may declare external CI evidence, remain strictly non-passing by default, and later be resolved from a versioned evidence manifest.
- **JSON verdict normalization (v2.2.0, breaking)** — the pre-existing human-review verdict is now serialized as `pending_review` instead of `pendingreview`; all multiword verdict values use snake_case.
- **Agent-facing CLI help (v2.2.1)** — `verify`, `lifecycle`, and `resolve-evidence --help` document caller AI mode, external-evidence policy, and CTest build prerequisites directly in the binary.
- **No hollow passes** — a test binding that resolves to **zero** tests *fails* instead of silently passing; `skip` and all-`#[ignore]` never count as `pass`.

## How it works (summary)

A **Task Contract** is a spec with four parts:

- `Intent` — what to do, and why
- `Decisions` — technical choices already fixed
- `Boundaries` — what may change, what must not (path entries are mechanically enforced)
- `Completion Criteria` — BDD scenarios with explicit `Test:` bindings → deterministic pass/fail

`contract` is the planning surface; `lifecycle` is the one-command quality gate (lint + verify + report).

## Install

### Prebuilt binaries (recommended)

Prebuilt binaries do not require a Rust toolchain or a source checkout. The
commands below pin the exact release tag so downstream automation cannot change
without an explicit version update. `/usr/local/bin` must be writable by the
current user; otherwise run the `tar` side of the pipeline with appropriate
administrator privileges.

macOS on Apple Silicon:

```bash
curl -fsSL https://github.com/BUNotesAI/specwright/releases/download/v2.2.1/specwright-aarch64-apple-darwin.tar.gz | tar -xz -C /usr/local/bin
```

Linux on x86_64 (recommended static musl build):

```bash
curl -fsSL https://github.com/BUNotesAI/specwright/releases/download/v2.2.1/specwright-x86_64-unknown-linux-musl.tar.gz | tar -xz -C /usr/local/bin
```

Verify the installed version:

```bash
specwright --version   # specwright 2.2.1
```

The same Release also provides `x86_64-unknown-linux-gnu` and
`aarch64-unknown-linux-gnu` archives. Every archive has a sibling
`.tar.gz.sha256` file. To verify an archive before extracting it:

```bash
archive=specwright-x86_64-unknown-linux-musl.tar.gz
base=https://github.com/BUNotesAI/specwright/releases/download/v2.2.1
curl -fsSLO "$base/$archive"
curl -fsSLO "$base/$archive.sha256"
sha256sum -c "$archive.sha256"
tar -xzf "$archive" -C /usr/local/bin
```

On macOS, use `shasum -a 256 -c "$archive.sha256"` for the checksum step.

### Cargo fallback (requires Rust)

Install from the current repository default branch:

```bash
cargo install --git https://github.com/BUNotesAI/specwright --locked
```

For a reproducible source build, pin the same release tag:

```bash
cargo install \
  --git https://github.com/BUNotesAI/specwright \
  --tag v2.2.1 \
  --locked
```

### Version policy

Downstream workflows should pin an exact release tag and treat major version
`2.x` as a hard compatibility gate. Every release must have a new `v*` tag.
Breaking selector or DSL changes require a new version and tag. Published tags
and assets are immutable: this project does not replace an existing asset,
repoint a published tag, or silently ship an untagged upgrade to pinned binary
installations.

For development from a local source checkout:

```bash
cargo install --path .
specwright --version   # 2.2.1
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

Keywords are English-only; description text may be any language. For a non-Cargo project, set `runner: maven | gradle | android | ios | node | ctest` in the frontmatter (or let it auto-detect from workspace markers).

### External verification

Use an external scenario when the result must come from CI or another evidence
producer and cannot run on the current machine:

```spec
Scenario: HarmonyOS release build
  Verification: external
  Evidence: harmony-release-build
  Tags: [SPC-12, tier3, DCR-04]
  Given the release commit is submitted to CI
  When the signed build finishes
  Then the evidence manifest records the build verdict
```

`Evidence` is required and unique within the spec. External scenarios do not
need `Test:` bindings. Their initial verdict is `external_pending`; strict mode
is the default and remains non-passing. Intermediate stages may opt in without
losing the pending count or result list:

```bash
specwright lifecycle task.spec.md --code . --external-mode allow-pending --format json
```

At close, import a complete versioned manifest. The manifest binds the exact
spec name and SHA-256, the repository `HEAD`, every scenario and Evidence ID,
an artifact URL and SHA-256 digest, a `pass` or `fail` verdict, and either a
producer identity or attestation:

```bash
specwright resolve-evidence task.spec.md --code . --manifest evidence.json --format json
```

Unknown, duplicate, or missing Evidence IDs fail the command. Resolution only
replaces `external_pending` and never overwrites a mechanical result.

For a prepared CMake/CTest project, configure and build the test tree before verification, then point `runner_config.build_dir` at that repository-relative directory:

```bash
cmake -S . -B build
cmake --build build
specwright lifecycle specs/native.spec.md --code . --format json
```

```spec
spec: task
name: "Native rules"
runner: ctest
runner_config: { build_dir: "build" }
---

## Completion Criteria

Scenario: Native rules pass
  Test:
    Filter: ^native_rules$
  Given the project registered its test with `add_test()` or `gtest_discover_tests()`
  When lifecycle verification runs
  Then CTest executes the selected compiled test
```

The CTest runner requires CMake/CTest 3.17 or newer, runs `ctest --output-on-failure --no-tests=error -R <Filter>` from the checked build directory, and never configures or builds the project. `build_dir` defaults to `build`; empty, absolute, and parent-traversing values are rejected. `Package:` is used only to choose an explicitly routed CTest slot in a mixed repository and is never passed to CTest.

For a mixed Rust + TypeScript repository, keep Cargo as the default runner and route explicit package tokens to Node:

```spec
spec: task
name: "Rust API plus admin UI"
runner: cargo
runners:
  node:
    root: web
    packages: { admin: "apps/admin" }
    config: { package_manager: "bun", unit_filter_style: "vitest" }
---

## Completion Criteria

Scenario: Rust API test passes
  Test:
    Package: api
    Filter: test_register_api_returns_201_for_new_user
  Given the API crate has the registration test
  When lifecycle verification runs
  Then Cargo executes that selector

Scenario: Admin page renders
  Test:
    Package: admin
    Filter: renders settings page
    Level: unit
  Given the admin package has a Vitest test script
  When lifecycle verification runs
  Then specwright runs `bun run test -- -t renders\ settings\ page` in `web/apps/admin`
```

Routing is explicit. A scalar `runner: node` spec still rejects `Package:` selectors; use the `runners:` block when a single spec needs Cargo plus Node package scenarios. `packages` maps scenario `Package:` tokens to paths relative to the route `root`. specwright does not auto-detect package routes or infer workspaces.

Operational notes for routed Node specs:

- Routed Node source discovery is scoped to the route `root`; Node source outside that subtree is not scanned for bindings.
- Legacy `@spec` bindings discovered under a routed Node route execute from the route root, not from an inferred package root; mixed specs should prefer explicit `Test:` selectors with routed `Package:` tokens.
- Node zero-match detection trusts the last structured Vitest `Tests` summary, counts only passed and failed tests as executed, and treats explicit no-test output as zero execution; custom reporters keep exit-code semantics and may emit an unparseable-output warning.
- Narrow `--code <file>` inputs do not auto-expand to route roots. Verify routed specs with a project or route-root directory scope.

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
| `resolve-evidence` | Validate a versioned external evidence manifest and resolve pending scenarios |
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
