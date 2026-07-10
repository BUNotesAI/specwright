spec: task
name: "Mixed lifecycle filter-miss fixture"
runner: cargo
runners:
  node:
    root: web
    packages: { admin: "apps/admin" }
    config: { package_manager: "npm", unit_filter_style: "vitest" }
---

## Intent

Prove that a routed Vitest filter which selects no test fails the lifecycle.

## Completion Criteria

Scenario: mixed routed admin filter miss
  Test:
    Package: admin
    Filter: missing admin smoke
    Level: unit
  Given a routed Node admin package with tests that do not match the filter
  When lifecycle verification runs
  Then the scenario fails because Vitest executed zero tests
