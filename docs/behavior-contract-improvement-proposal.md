# Agent-Spec Improvement Proposal: From Valid Contracts to Complete Behavior Contracts

## Summary

The recent `chub-rs` rewrite surfaced a recurring failure mode in `agent-spec` practice:
contracts can be syntactically valid, lint-clean, and still miss critical user-visible behavior.

`agent-spec` has already become much better at preventing malformed specs and structurally weak contracts.
The next step is to improve its ability to detect when a contract looks complete but still leaves important observable behavior unbound.

This proposal focuses on that next step.

## Problem Statement

The `chub-rs` work revealed three distinct classes of misses:

1. Some important behaviors were never specified.
Examples: `get -o/--output` semantics, stdout behavior when writing files, JSON-vs-human output differences.

2. Some behaviors were specified as decisions, but not turned into executable scenarios.
Examples: remote read fallback order, cache/bundle/HTTP precedence, startup bootstrap behavior.

3. Some behaviors were specified and had test selectors, but the tests did not actually verify the production path.
Examples: HTTP non-2xx handling was expressed in the contract, but the test only validated an injected error path rather than the real `reqwest` response path.

These failures share one root cause:
`agent-spec` currently does a better job checking contract structure than checking behavioral completeness and verification strength.

## What This Suggests About Agent-Spec

### 1. Decision coverage is necessary but not sufficient

A decision can be mentioned by a scenario without being meaningfully verified.
This is especially true for decisions about:

- output behavior
- precedence and fallback order
- local vs remote behavior
- cache and bootstrap behavior
- env var priority
- timeout policy
- failure handling at external I/O boundaries

`agent-spec` should treat these as high-risk observable decisions, not ordinary implementation details.

### 2. Thin edge behavior is easy to miss

Contracts naturally focus on primary workflows.
But many user-facing regressions happen in thin edge surfaces such as:

- `-o/--output`
- `--json`
- stdout/stderr cleanliness
- empty cache / cold start
- partial failure behavior
- multi-source ambiguity
- file side effects

These behaviors are small in code size but large in user impact.
They need explicit contract support.

### 3. Test selector presence does not guarantee verification strength

A contract can say:

```spec
测试: update_rejects_non_2xx_registry_response
```

and still be backed by a weak test that never touches the real production path.

That means the current selector model is useful for traceability, but too weak to express verification depth.

### 4. Rewrite/parity work needs a different contract style from feature work

A greenfield feature contract asks: "what should exist?"
A parity migration contract asks: "what observable behavior must not drift?"

These are different authoring problems.
`agent-spec` should reflect that difference in its templates, lint rules, and skill guidance.

## Proposal

## A. Add Behavioral Completeness Linters

Introduce new lint rules aimed at observable behavior gaps.

### 1. `observable-decision-coverage`

When a decision contains keywords that imply externally visible behavior, require at least one matching scenario.

High-risk keywords should include:

- `stdout`
- `stderr`
- `--json`
- `-o`
- `output`
- `fallback`
- `precedence`
- `priority`
- `cache`
- `local`
- `remote`
- `bundle`
- `timeout`
- `env`
- `force`

This should be stricter than current decision coverage. The goal is not simple mention overlap, but explicit behavioral verification.

### 2. `output-mode-coverage`

If a contract mentions multiple output modes or flags such as `--json`, `-o`, or stdout/stderr rules, require scenarios that cover each mode.

Example expectation:

- human mode covered
- JSON mode covered
- file-output mode covered
- mixed mode conflicts covered when relevant

### 3. `precedence-fallback-coverage`

If a decision contains ordered behavior such as:

- `A -> B -> C`
- `priority`
- `fallback`
- `prefer`
- `otherwise`

then require at least one scenario validating that ordering.

This is directly applicable to cache/bundle/remote lookup chains and env-var precedence.

### 4. `external-io-error-strength`

If a scenario is about HTTP, filesystem, process, or protocol failure behavior, require explicit evidence that the test touches a realistic boundary.

This can start as a warning, not an error.
The rule can look for clues like:

- local stub server
- temp dir / fixture files
- parser invoked on real bytes
- CLI or integration layer selector naming conventions

The rule does not need to fully prove strength; it only needs to flag suspiciously weak scenarios.

## B. Extend Contract Semantics for Test Strength

Today, `测试:` binds a name. That is useful, but underspecified.

Add optional scenario annotations such as:

```spec
场景: update 拒绝 HTTP 4xx/5xx 响应
  测试: update_rejects_non_2xx_registry_response
  层级: integration
  替身: local_http_stub
  命中: commands/update
```

Possible fields:

- `层级` / `Level`: `unit`, `integration`, `cli`, `e2e`
- `替身` / `Test Double`: `none`, `fixture_fs`, `local_http_stub`, `mock_only`
- `命中` / `Targets`: the subsystem or entry point meant to be exercised

These fields should be optional at first, but heavily encouraged for high-risk I/O behavior.

## C. Add Rewrite/Parity Contract Templates

Introduce a dedicated template for migration and compatibility work.

That template should prompt authors to cover a behavior matrix by default:

- command x output mode
- local x remote source
- warm cache x cold start
- success x partial failure x hard failure
- single entry x multi-entry x ambiguous entry
- CLI x MCP entry points when both exist

This will help contract authors avoid thinking only in terms of modules and functions.

## D. Improve Skill Guidance

### 1. `agent-spec-authoring`

Add a section called `Behavior Surface Checklist` for CLI/tools/protocol software.

Checklist items should include:

- Are stdout and stderr both specified where relevant?
- Is `--json` behavior specified?
- Are file-writing flags specified?
- Is cold-start behavior specified?
- Is fallback/precedence order turned into scenarios?
- Are partial failures specified?
- Are local and remote paths both covered?
- Are side effects on disk specified?

### 2. `agent-spec-tool-first`

Add a review step after `parse + lint`:

- Ask: which user-visible behaviors remain unbound?
- Ask: which decisions still read like prose rather than executable contracts?
- Ask: which scenarios depend on external I/O and may be backed by weak tests?

This should become part of the standard contract review workflow, especially for parity work.

## E. Add a Formal Review Heuristic: Unbound Observable Behavior

Add a small review framework to agent-spec docs and skills:

Before approving a contract, reviewers must ask:

- What can a user observe from stdout, stderr, files, network calls, and persisted state?
- Which of those observations are not yet tied to a scenario?
- Which flags or modes are mentioned in help/docs but absent from scenarios?
- Which fallback or precedence decisions are stated but not tested?

This heuristic is simple, but it directly targets the class of misses seen in `chub-rs`.

## Recommended Rollout

### Phase 1: Skill and Template Changes

Low risk, fast feedback.

- Update `agent-spec-authoring` with the behavior-surface checklist
- Update `agent-spec-tool-first` with the unbound-observable-behavior review step
- Add a rewrite/parity example contract to references

### Phase 2: New Warning-Level Linters

Start as warnings so authors can adapt.

- `observable-decision-coverage`
- `output-mode-coverage`
- `precedence-fallback-coverage`
- `external-io-error-strength`

### Phase 3: Optional Scenario Metadata

Add parser support for optional `层级/替身/命中` metadata lines.
Use lint to encourage them on high-risk scenarios.

### Phase 4: Promote Selected Warnings to Errors

After real-world usage stabilizes, promote the most useful rules.
Prefer upgrading only the linters with low false-positive rates.

## Expected Benefits

If adopted, these changes should reduce a specific class of failures:

- contracts that are structurally valid but behaviorally incomplete
- parity migrations that miss thin but important user-visible behavior
- tests that technically satisfy a selector but fail to prove the intended guarantee

In short, this moves `agent-spec` forward from:

- "Is this spec parseable and reasonably complete?"

Toward:

- "Does this spec actually bind the observable behavior that matters?"

## Addendum: Round 4 Findings (2026-03-11)

A fourth review round on `chub-rs` exposed three additional failure classes
not covered by the original proposal:

### A5. Flag Combination Coverage

When a CLI command has multiple flags that affect output behavior (`-o`, `--json`,
`--full`, multi-ID), per-flag testing misses interaction bugs. For example:

- `get a -o out.md` passes (single ID writes file correctly)
- `get a b -o out.md` fails (multi-ID overwrites file, losing first result)

Implemented as linter #14: `flag-combination-coverage`. Warns when 2+ output-affecting
flags are mentioned in decisions but no scenario tests a combination of them.

### A6. Platform Decision Tagging

When rewriting from one platform to another (JS → Rust), decisions may reference
platform-specific concepts (npm, dist/, bundled dist) without marking them as
platform-dependent. This creates phantom requirements that persist through reviews.

Implemented as linter #15: `platform-decision-tag`. Flags untagged references to
npm, pip, cargo install, dist/, etc. Suggests adding `[JS-only]`, `[platform-specific]`,
or explicit "not applicable" markers.

### A7. Architectural Invariants

Some behaviors depend on processing patterns, not per-feature logic. For example,
JS collects all results into an array then handles output in one pass, while a
naive Rust port might process-and-output per-item. This architectural difference
is invisible to single-item tests but breaks on combinations.

This is not yet implemented as a linter (hard to detect mechanically), but is
documented in the behavior surface checklist in `agent-spec-authoring`.

## Implementation Status

| Item | Status |
|------|--------|
| Skill updates (Phase 1) | Done — behavior surface checklist added |
| `observable-decision-coverage` (Phase 2) | Done — linter #10b |
| `output-mode-coverage` (Phase 2) | Done — linter #10c |
| `precedence-fallback-coverage` (Phase 2) | Done — linter #10d |
| `external-io-error-strength` (Phase 2) | Done — linter #10e |
| `verification-metadata-suggestion` (Phase 2) | Done — linter #10f |
| `flag-combination-coverage` (Addendum) | Done — linter #14 |
| `platform-decision-tag` (Addendum) | Done — linter #15 |
| Scenario metadata (Phase 3) | Done — parser supports Level/TestDouble/Targets |
| Rewrite/parity template (Phase 1) | Done — `--template rewrite-parity` in CLI |
| Promote warnings to errors (Phase 4) | Pending |

## Practical Next Step

The remaining high-value items are:

1. create a rewrite/parity example contract as a template
2. promote `flag-combination-coverage` and `platform-decision-tag` from Info/Warning
   to Warning/Error after real-world usage stabilizes
3. consider an `architectural-invariant` linter if a mechanical detection heuristic
   can be found (collect-then-output vs per-item-output patterns)
