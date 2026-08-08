spec: task
name: "Native CTest pass fixture"
runner: ctest
runner_config: { build_dir: "build" }
---

## Completion Criteria

Scenario: native CTest pass scenario
  Test:
    Filter: ^ctest_pass$
    Level: unit
  Given a configured CTest build tree
  When lifecycle verification selects the passing native test
  Then CTest runs the compiled C executable
