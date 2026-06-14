spec: task
name: "Cargo lifecycle command program baseline"
---

## Intent

Verify that default Cargo runner lifecycle JSON preserves the historical
`Evidence::TestOutput` shape while still exercising a bound scenario.

## Completion Criteria

Scenario: Default Cargo lifecycle omits command_program
  Test:
    Package: specwright
    Filter: test_cargo_command_program_evidence_is_omitted_for_json_compatibility
  Given a task spec with a bound Cargo unit test
  When lifecycle verification runs with the default Cargo runner
  Then the scenario passes
  And the emitted TestOutput evidence omits command_program
