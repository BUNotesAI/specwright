spec: task
name: "Remote Source Parity"
inherits: project
tags: [rewrite, parity, cli]
---

## Intent

Keep the Rust CLI behavior aligned with the existing JavaScript CLI for remote-source reads.
This contract focuses on observable behavior rather than internal structure, so that cache state,
output mode, and source-type regressions are caught before coding drifts from parity.

## Decisions

- `get` must preserve the documented lookup order: local source -> cache -> bundled content -> remote fetch
- `--json` and human output are separate observable modes and both must remain stable
- cold start behavior must be verified explicitly; parity work cannot assume a pre-warmed cache

## Boundaries

### Allowed Changes
- src/commands/get.rs
- src/core/cache.rs
- tests/**

### Forbidden
- Do not change the published CLI flags
- Do not weaken the remote lookup order for easier implementation

## Completion Criteria

Scenario: human mode returns doc content from cached remote source
  Test: test_get_human_mode_uses_cached_remote_content
  Given a remote source registry and cached doc content
  When the user runs `get` without `--json`
  Then stdout contains the doc body
  And stderr remains available for diagnostics only

Scenario: json mode returns structured payload
  Test: test_get_json_mode_returns_structured_payload
  Given a remote source registry and cached doc content
  When the user runs `get --json`
  Then stdout contains only JSON
  And the payload includes `id`, `type`, and `content`

Scenario: cold start falls back to bundled content before remote fetch
  Test: test_get_cold_start_prefers_bundled_content_before_http
  Given no cached doc content and bundled content for the target entry
  When the user runs `get`
  Then bundled content is returned
  And no remote HTTP request is required

Scenario: remote fetch failure returns a stable error
  Test: test_get_remote_fetch_failure_reports_stable_error
  Given no cached or bundled content and the remote source returns HTTP 404
  When the user runs `get`
  Then the command fails
  And the error explains that the remote content could not be fetched

