spec: task
name: "CTest missing executable fixture"
runner: ctest
runner_config: { build_dir: "build" }
---

## Completion Criteria

Scenario: missing registered executable scenario
  Test:
    Filter: ^ctest_missing_executable$
    Level: unit
  Given a registered CTest entry whose executable is missing
  When lifecycle verification selects that entry
  Then CTest reports the missing executable as failure
