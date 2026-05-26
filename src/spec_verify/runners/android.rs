use std::collections::HashMap;

use crate::spec_core::{SpecError, SpecResult, TestSelector};

use super::jvm::scan_workspace_bindings;
use super::{
    HostPlatform, PreflightOutcome, RunnerWorkspace, TestCommand, TestRunner, WorkspaceMarkers,
};

/// Built-in Android test runner.
pub struct AndroidRunner {
    host_platform: HostPlatform,
    adb: AndroidAdb,
}

impl AndroidRunner {
    /// Creates an Android runner bound to the current host and system ADB.
    pub fn new() -> Self {
        Self {
            host_platform: current_host_platform(),
            adb: AndroidAdb::System,
        }
    }

    #[cfg(test)]
    fn for_test(host_platform: HostPlatform, adb: AndroidAdb) -> Self {
        Self { host_platform, adb }
    }
}

impl Default for AndroidRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
enum AndroidAdb {
    System,
    #[cfg(test)]
    FixedOutput {
        stdout: String,
        success: bool,
    },
}

impl TestRunner for AndroidRunner {
    fn id(&self) -> &'static str {
        "android"
    }

    fn detect(&self, markers: &WorkspaceMarkers) -> bool {
        markers.contains("AndroidManifest.xml")
            && (markers.contains("build.gradle") || markers.contains("build.gradle.kts"))
    }

    fn source_extensions(&self) -> &'static [&'static str] {
        &["java", "kt"]
    }

    fn build_test_command(
        &self,
        workspace: &RunnerWorkspace,
        selector: &TestSelector,
    ) -> SpecResult<TestCommand> {
        match selector.level.as_deref() {
            None | Some("unit") => Ok(TestCommand {
                program: android_gradle_program(workspace),
                args: vec![
                    android_gradle_task(selector.package.as_deref(), "testDebugUnitTest"),
                    "--tests".to_string(),
                    selector.filter.replace('#', "."),
                ],
            }),
            Some("instrumented") => Ok(TestCommand {
                program: android_gradle_program(workspace),
                args: vec![
                    android_gradle_task(selector.package.as_deref(), "connectedAndroidTest"),
                    format!(
                        "-Pandroid.testInstrumentationRunnerArguments.class={}",
                        selector.filter
                    ),
                ],
            }),
            Some(level) => Err(SpecError::Verification(format!(
                "unknown Android test level `{level}`; expected one of [\"unit\", \"instrumented\"]"
            ))),
        }
    }

    fn scan_legacy_bindings(
        &self,
        workspace: &RunnerWorkspace,
    ) -> SpecResult<HashMap<String, String>> {
        Ok(scan_workspace_bindings(workspace))
    }

    fn preflight(
        &self,
        _workspace: &RunnerWorkspace,
        selector: &TestSelector,
    ) -> SpecResult<PreflightOutcome> {
        if !self.requires_device(selector) {
            return Ok(PreflightOutcome::Ready);
        }

        if self.host_platform == HostPlatform::Windows {
            return Ok(PreflightOutcome::MissingCapability {
                capability: "android-instrumented-on-windows".to_string(),
                reason:
                    "Android instrumented tests are not supported on Windows hosts; run on Linux or macOS"
                        .to_string(),
            });
        }

        let adb_result = match &self.adb {
            AndroidAdb::System => {
                let program = std::env::var("ANDROID_HOME")
                    .or_else(|_| std::env::var("ANDROID_SDK_ROOT"))
                    .map(|sdk| format!("{sdk}/platform-tools/adb"))
                    .unwrap_or_else(|_| "adb".to_string());
                std::process::Command::new(program).arg("devices").output()
            }
            #[cfg(test)]
            AndroidAdb::FixedOutput { stdout, success } => {
                return Ok(adb_preflight_outcome(*success, stdout));
            }
        };

        match adb_result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                Ok(adb_preflight_outcome(output.status.success(), &stdout))
            }
            Err(error) => Ok(PreflightOutcome::MissingCapability {
                capability: "adb-device".to_string(),
                reason: format!("failed to run adb devices: {error}"),
            }),
        }
    }

    fn recognized_config_keys(&self) -> &'static [&'static str] {
        &["gradle_args", "instrumentation_runner"]
    }

    fn requires_device(&self, selector: &TestSelector) -> bool {
        selector.level.as_deref() == Some("instrumented")
    }

    fn supported_host_platforms(&self) -> &'static [HostPlatform] {
        &[
            HostPlatform::Linux,
            HostPlatform::MacOS,
            HostPlatform::Windows,
        ]
    }
}

fn android_gradle_program(workspace: &RunnerWorkspace) -> String {
    if workspace.markers.contains("gradlew") {
        "./gradlew".to_string()
    } else if workspace.markers.contains("gradlew.bat") {
        "gradlew.bat".to_string()
    } else {
        "gradle".to_string()
    }
}

fn android_gradle_task(package: Option<&str>, task: &str) -> String {
    let module = package.unwrap_or(":app");
    if module.starts_with(':') {
        format!("{module}:{task}")
    } else {
        format!(":{module}:{task}")
    }
}

fn adb_preflight_outcome(success: bool, stdout: &str) -> PreflightOutcome {
    if success && adb_devices_has_active_device(stdout) {
        return PreflightOutcome::Ready;
    }

    PreflightOutcome::MissingCapability {
        capability: "adb-device".to_string(),
        reason: "adb devices did not report an active device".to_string(),
    }
}

fn adb_devices_has_active_device(stdout: &str) -> bool {
    stdout.lines().any(|line| {
        let mut fields = line.split_whitespace();
        let Some(_serial) = fields.next() else {
            return false;
        };
        matches!(fields.next(), Some("device"))
    })
}

fn current_host_platform() -> HostPlatform {
    match std::env::consts::OS {
        "linux" => HostPlatform::Linux,
        "macos" => HostPlatform::MacOS,
        "windows" => HostPlatform::Windows,
        _ => HostPlatform::Other,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::spec_core::TestSelector;

    use super::super::{HostPlatform, PreflightOutcome, RunnerWorkspace, TestRunner};
    use super::{AndroidAdb, AndroidRunner};

    #[test]
    fn test_android_preflight_missing_adb_returns_missing_capability() {
        let runner = AndroidRunner::for_test(
            HostPlatform::Linux,
            AndroidAdb::FixedOutput {
                stdout: "List of devices attached\n\n".to_string(),
                success: true,
            },
        );

        let outcome = runner
            .preflight(&RunnerWorkspace::for_test("."), &instrumented_selector())
            .unwrap();

        assert_eq!(
            outcome,
            PreflightOutcome::MissingCapability {
                capability: "adb-device".to_string(),
                reason: "adb devices did not report an active device".to_string()
            }
        );
    }

    #[test]
    fn test_android_preflight_windows_instrumented_skips_unconditionally() {
        let runner = AndroidRunner::for_test(
            HostPlatform::Windows,
            AndroidAdb::FixedOutput {
                stdout: "List of devices attached\nemulator-5554\tdevice\n".to_string(),
                success: true,
            },
        );

        let outcome = runner
            .preflight(&RunnerWorkspace::for_test("."), &instrumented_selector())
            .unwrap();

        assert_eq!(
            outcome,
            PreflightOutcome::MissingCapability {
                capability: "android-instrumented-on-windows".to_string(),
                reason:
                    "Android instrumented tests are not supported on Windows hosts; run on Linux or macOS"
                        .to_string()
            }
        );

        let unit_outcome = runner
            .preflight(&RunnerWorkspace::for_test("."), &unit_selector())
            .unwrap();
        assert_eq!(unit_outcome, PreflightOutcome::Ready);
    }

    fn instrumented_selector() -> TestSelector {
        TestSelector {
            package: Some(":app".to_string()),
            filter: "com.example.PaymentRulesTest#approvesValidCard".to_string(),
            level: Some("instrumented".to_string()),
            test_double: None,
            targets: None,
        }
    }

    fn unit_selector() -> TestSelector {
        TestSelector {
            level: None,
            ..instrumented_selector()
        }
    }
}
