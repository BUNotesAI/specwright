use std::collections::HashMap;

use crate::spec_core::{SpecResult, TestSelector};

use super::jvm::scan_workspace_bindings;
use super::{RunnerWorkspace, TestCommand, TestRunner, WorkspaceMarkers};

/// Built-in Maven test runner.
pub struct MavenRunner;

impl TestRunner for MavenRunner {
    fn id(&self) -> &'static str {
        "maven"
    }

    fn detect(&self, markers: &WorkspaceMarkers) -> bool {
        markers.contains("pom.xml")
    }

    fn source_extensions(&self) -> &'static [&'static str] {
        &["java", "kt"]
    }

    fn build_test_command(
        &self,
        workspace: &RunnerWorkspace,
        selector: &TestSelector,
    ) -> SpecResult<TestCommand> {
        let mut args = vec!["test".to_string()];
        if let Some(package) = &selector.package {
            args.push("-pl".to_string());
            args.push(package.clone());
        }
        args.push(format!("-Dtest={}", selector.filter));

        Ok(TestCommand {
            program: maven_program(workspace),
            args,
        })
    }

    fn scan_legacy_bindings(
        &self,
        workspace: &RunnerWorkspace,
    ) -> SpecResult<HashMap<String, String>> {
        Ok(scan_workspace_bindings(workspace))
    }
}

fn maven_program(workspace: &RunnerWorkspace) -> String {
    if workspace.markers.contains("mvnw") {
        "./mvnw".to_string()
    } else if workspace.markers.contains("mvnw.cmd") {
        "mvnw.cmd".to_string()
    } else {
        "mvn".to_string()
    }
}
