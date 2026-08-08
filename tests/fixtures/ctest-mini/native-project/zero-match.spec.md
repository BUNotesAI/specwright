spec: task
name: "CTest zero-match fixture"
runner: ctest
runner_config: { build_dir: "build" }
---

## Completion Criteria

Scenario: CTest zero match scenario
  Test:
    Filter: ^no_such_ctest_test$
    Level: unit
  Given a configured CTest build tree
  When lifecycle verification selects a missing test name
  Then CTest reports zero matching tests as failure
