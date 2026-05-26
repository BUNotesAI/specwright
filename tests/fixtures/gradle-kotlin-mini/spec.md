spec: task
name: "Gradle Kotlin mini fixture"
---

## Intent

Verify that the Gradle runner can execute a Kotlin JUnit 5 test through the real Gradle Wrapper.

## Acceptance Criteria

Scenario: gradle kotlin scenario
  Given a Gradle Kotlin fixture
  When the lifecycle test runner executes the bound test
  Then the scenario passes
