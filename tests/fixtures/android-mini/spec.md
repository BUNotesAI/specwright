spec: task
name: "Android Kotlin mini fixture"
runner: android
---

## Intent

Verify that the Android runner can execute a Kotlin unit test through the real Gradle Wrapper, Android Gradle Plugin, and Android SDK.

## Acceptance Criteria

Scenario: android kotlin unit scenario
  Test:
    Package: :app
    Filter: com.example.PaymentRulesTest#approvesValidCard
    Level: unit
  Given an Android Kotlin fixture
  When the lifecycle test runner executes the bound unit test
  Then the scenario passes
