spec: task
name: "Markerless CTest consumer fixture"
runner: ctest
runner_config: { build_dir: ".specwright/ctest-build" }
---

## Completion Criteria

Scenario: markerless host script scenario
  Test:
    Filter: ^markerless_host_test$
    Level: unit
  Given a repository-root code scope with only a nested test-owned CMake registration
  When lifecycle verification uses explicit CTest selection
  Then the markerless root fallback executes the registered host script
