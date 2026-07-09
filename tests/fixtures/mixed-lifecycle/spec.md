spec: task
name: "Mixed lifecycle fixture"
runner: cargo
runners:
  node:
    root: web
    packages: { admin: "apps/admin" }
    config: { package_manager: "npm", unit_filter_style: "vitest" }
---

## Intent

Verify that one lifecycle run can merge default Cargo scenarios and routed Node package scenarios.

## Completion Criteria

Scenario: mixed cargo scenario
  Test:
    Filter: backend_smoke_test
    Level: unit
  Given a Rust crate in the mixed fixture
  When lifecycle verification runs
  Then the Cargo test passes

Scenario: mixed routed admin scenario
  Test:
    Package: admin
    Filter: admin smoke
    Level: unit
  Given a routed Node admin package in the mixed fixture
  When lifecycle verification runs
  Then the admin package test passes through npm from the package directory
