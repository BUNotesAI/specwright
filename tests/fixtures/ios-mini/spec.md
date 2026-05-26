spec: task
name: "iOS XCTest mini fixture"
runner: ios
runner_config: { scheme: "IosMini", destination: "platform=iOS Simulator,name=iPhone 16 Pro" }
---

## Intent

Verify that the iOS runner can execute an XCTest through the real Xcode command-line tools and iOS Simulator.

## Acceptance Criteria

Scenario: ios xctest scenario
  Test:
    Package: IosMiniTests
    Filter: PaymentTests/testRejectsExpiredCard
  Given an iOS XCTest fixture
  When the lifecycle test runner executes the bound test
  Then the scenario passes
