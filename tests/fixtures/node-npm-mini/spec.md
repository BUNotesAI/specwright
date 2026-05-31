spec: task
name: "Node npm mini fixture"
runner_config: { unit_filter_style: "vitest" }
---

## Intent

Verify that a package-json-only Node workspace can execute install-free npm scripts through the generic Node runner.

## Completion Criteria

Scenario: node npm unit scenario
  Test:
    Filter: renders dashboard
    Level: unit
  Given a package-json-only Node workspace
  When lifecycle verification runs
  Then the unit script executes through npm with the forwarded test-name filter

Scenario: node npm typecheck scenario
  Test:
    Filter: -
    Level: typecheck
  Given a package-json-only Node workspace
  When lifecycle verification runs
  Then the typecheck script executes through npm without a test-name filter

Scenario: node npm build scenario
  Test:
    Filter: -
    Level: build
  Given a package-json-only Node workspace
  When lifecycle verification runs
  Then the build script executes through npm without a test-name filter
