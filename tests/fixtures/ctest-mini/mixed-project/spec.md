spec: task
name: "Mixed Cargo and CTest fixture"
runner: cargo
runners:
  ctest:
    root: native
    packages: { cppslot: "." }
    config: { build_dir: "build" }
---

## Completion Criteria

Scenario: mixed Cargo default scenario
  Test:
    Filter: cargo_default_slot_passes
    Level: unit
  Given a Cargo default runner slot
  When lifecycle verification runs the scenario without Package
  Then Cargo executes the Rust test

Scenario: mixed routed CTest scenario
  Test:
    Package: cppslot
    Filter: ^ctest_route_pass$
    Level: unit
  Given a routed CTest slot
  When lifecycle verification selects Package cppslot
  Then CTest executes the registered native test
