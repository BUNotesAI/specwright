spec: task
name: "Maven mini fixture"
runner: maven
---

## Intent

Verify that the Maven runner can execute a Java JUnit 5 test through the real Maven Wrapper.

## Acceptance Criteria

Scenario: maven java scenario
  Given a Maven Java fixture
  When the lifecycle test runner executes the bound test
  Then the scenario passes
