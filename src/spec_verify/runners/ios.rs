use std::collections::HashMap;

use crate::spec_core::{SpecResult, TestSelector};

use super::{
    HostPlatform, PreflightOutcome, RunnerWorkspace, TestCommand, TestRunner, WorkspaceMarkers,
};

/// Built-in iOS XCTest runner for simulator workflows.
pub struct IosRunner {
    simctl: IosSimctl,
}

impl IosRunner {
    /// Creates an iOS runner bound to the system Xcode command-line tools.
    pub fn new() -> Self {
        Self {
            simctl: IosSimctl::System,
        }
    }

    #[cfg(test)]
    fn for_test(simctl: IosSimctl) -> Self {
        Self { simctl }
    }
}

impl Default for IosRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
enum IosSimctl {
    System,
    #[cfg(test)]
    FixedOutput {
        stdout: String,
        success: bool,
    },
}

impl TestRunner for IosRunner {
    fn id(&self) -> &'static str {
        "ios"
    }

    fn detect(&self, markers: &WorkspaceMarkers) -> bool {
        markers.contains("Package.swift") || markers.contains("*.xcodeproj")
    }

    fn source_extensions(&self) -> &'static [&'static str] {
        &["swift"]
    }

    fn build_test_command(
        &self,
        workspace: &RunnerWorkspace,
        selector: &TestSelector,
    ) -> SpecResult<TestCommand> {
        let mut args = vec!["test".to_string()];
        if let Some(scheme) = config_value(workspace, "scheme") {
            args.push("-scheme".to_string());
            args.push(scheme.to_string());
        }
        if let Some(destination) = config_value(workspace, "destination") {
            args.push("-destination".to_string());
            args.push(destination.to_string());
        }
        args.push(format!("-only-testing:{}", only_testing_selector(selector)));

        Ok(TestCommand {
            program: "xcodebuild".to_string(),
            args,
        })
    }

    fn scan_legacy_bindings(
        &self,
        _workspace: &RunnerWorkspace,
    ) -> SpecResult<HashMap<String, String>> {
        Ok(HashMap::new())
    }

    fn preflight(
        &self,
        _workspace: &RunnerWorkspace,
        _selector: &TestSelector,
    ) -> SpecResult<PreflightOutcome> {
        let simctl_result = match &self.simctl {
            IosSimctl::System => std::process::Command::new("xcrun")
                .args(["simctl", "list", "devices", "booted"])
                .output(),
            #[cfg(test)]
            IosSimctl::FixedOutput { stdout, success } => {
                return Ok(simctl_preflight_outcome(*success, stdout));
            }
        };

        match simctl_result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                Ok(simctl_preflight_outcome(output.status.success(), &stdout))
            }
            Err(error) => Ok(PreflightOutcome::MissingCapability {
                capability: "ios-simulator".to_string(),
                reason: format!("failed to run xcrun simctl list devices booted: {error}"),
            }),
        }
    }

    fn recognized_config_keys(&self) -> &'static [&'static str] {
        &["destination", "scheme"]
    }

    fn supported_host_platforms(&self) -> &'static [HostPlatform] {
        &[HostPlatform::MacOS]
    }
}

fn config_value<'a>(workspace: &'a RunnerWorkspace, key: &str) -> Option<&'a str> {
    workspace
        .config
        .get(key)
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
}

fn only_testing_selector(selector: &TestSelector) -> String {
    match selector.package.as_deref() {
        Some(package) if !package.trim().is_empty() => format!("{package}/{}", selector.filter),
        _ => selector.filter.clone(),
    }
}

fn simctl_preflight_outcome(success: bool, stdout: &str) -> PreflightOutcome {
    if success && simctl_has_booted_device(stdout) {
        return PreflightOutcome::Ready;
    }

    PreflightOutcome::MissingCapability {
        capability: "ios-simulator".to_string(),
        reason: "xcrun simctl did not report a booted iOS simulator".to_string(),
    }
}

fn simctl_has_booted_device(stdout: &str) -> bool {
    stdout.lines().any(|line| line.contains("(Booted)"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::spec_core::TestSelector;

    use super::super::{PreflightOutcome, RunnerWorkspace, TestRunner};
    use super::{IosRunner, IosSimctl};

    #[test]
    fn test_ios_preflight_with_booted_simulator_is_ready() {
        let runner = IosRunner::for_test(IosSimctl::FixedOutput {
            stdout: "iPhone 15 (00000000-0000-0000-0000-000000000000) (Booted)\n".to_string(),
            success: true,
        });

        let outcome = runner
            .preflight(&RunnerWorkspace::for_test("."), &selector())
            .unwrap();

        assert_eq!(outcome, PreflightOutcome::Ready);
    }

    #[test]
    fn test_ios_preflight_without_booted_simulator_returns_missing_capability() {
        let runner = IosRunner::for_test(IosSimctl::FixedOutput {
            stdout: "== Devices ==\n-- iOS 18.4 --\n".to_string(),
            success: true,
        });

        let outcome = runner
            .preflight(&RunnerWorkspace::for_test("."), &selector())
            .unwrap();

        assert_eq!(
            outcome,
            PreflightOutcome::MissingCapability {
                capability: "ios-simulator".to_string(),
                reason: "xcrun simctl did not report a booted iOS simulator".to_string()
            }
        );
    }

    fn selector() -> TestSelector {
        TestSelector {
            package: Some("IosMiniTests".to_string()),
            filter: "PaymentTests/testRejectsExpiredCard".to_string(),
            level: None,
            test_double: None,
            targets: None,
        }
    }
}
