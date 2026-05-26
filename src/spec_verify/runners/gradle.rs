use std::collections::HashMap;

use crate::spec_core::{SpecResult, TestSelector};

use super::jvm::scan_workspace_bindings;
use super::{RunnerWorkspace, TestCommand, TestRunner, WorkspaceMarkers};

/// Built-in Gradle test runner.
pub struct GradleRunner;

impl TestRunner for GradleRunner {
    fn id(&self) -> &'static str {
        "gradle"
    }

    fn detect(&self, markers: &WorkspaceMarkers) -> bool {
        markers.contains("build.gradle") || markers.contains("build.gradle.kts")
    }

    fn source_extensions(&self) -> &'static [&'static str] {
        &["java", "kt"]
    }

    fn build_test_command(
        &self,
        workspace: &RunnerWorkspace,
        selector: &TestSelector,
    ) -> SpecResult<TestCommand> {
        let test_task = selector
            .package
            .as_deref()
            .map(gradle_test_task)
            .unwrap_or_else(|| "test".to_string());
        Ok(TestCommand {
            program: gradle_program(workspace),
            args: vec![
                test_task,
                "--tests".to_string(),
                selector.filter.replace('#', "."),
            ],
        })
    }

    fn scan_legacy_bindings(
        &self,
        workspace: &RunnerWorkspace,
    ) -> SpecResult<HashMap<String, String>> {
        Ok(scan_workspace_bindings(workspace))
    }
}

fn gradle_program(workspace: &RunnerWorkspace) -> String {
    if workspace.markers.contains("gradlew") {
        "./gradlew".to_string()
    } else if workspace.markers.contains("gradlew.bat") {
        "gradlew.bat".to_string()
    } else {
        "gradle".to_string()
    }
}

fn gradle_test_task(package: &str) -> String {
    if package.starts_with(':') {
        format!("{package}:test")
    } else {
        format!(":{package}:test")
    }
}
