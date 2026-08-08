spec: task
name: "Native CTest failure fixture"
runner: ctest
runner_config: { build_dir: "build" }
---

## Completion Criteria

Scenario: native CTest failure scenario
  Test:
    Filter: ^ctest_fail$
    Level: unit
  Given a configured CTest build tree
  When lifecycle verification selects the failing native test
  Then CTest reports the executable failure
