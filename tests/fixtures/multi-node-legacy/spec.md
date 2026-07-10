spec: task
name: "Multiple Node route legacy binding fixture"
runner: cargo
runners:
  node:
    root: web/apps/admin
    packages: {}
    config: { package_manager: "npm", unit_filter_style: "vitest" }
  node:
    root: web/apps/portal
    packages: {}
    config: { package_manager: "npm", unit_filter_style: "vitest" }
---

## Intent

Verify that a legacy binding is owned by the Node route containing its source file.

## Completion Criteria

Scenario: portal legacy scenario
  Given the binding comment exists only under the portal route
  When lifecycle verification scans both Node routes
  Then the portal route executes the legacy-bound test from its own route root
