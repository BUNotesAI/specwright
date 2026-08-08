use std::collections::HashMap;
use std::path::{Component, Path};

use crate::spec_core::{SpecError, SpecResult, TestSelector};

use super::{RunnerWorkspace, TestCommand, TestRunner, WorkspaceMarkers};

/// Built-in CTest runner for prepared CMake build trees.
pub struct CTestRunner;

impl TestRunner for CTestRunner {
    fn id(&self) -> &'static str {
        "ctest"
    }

    fn detect(&self, markers: &WorkspaceMarkers) -> bool {
        markers.contains("CMakeLists.txt")
    }

    fn build_test_command(
        &self,
        workspace: &RunnerWorkspace,
        selector: &TestSelector,
    ) -> SpecResult<TestCommand> {
        let root = workspace.root.as_ref().ok_or_else(|| {
            SpecError::Verification("ctest runner requires a resolved workspace root".into())
        })?;
        let build_dir = workspace
            .config
            .get("build_dir")
            .map(String::as_str)
            .unwrap_or("build");
        let build_path = checked_build_dir(build_dir)?;

        Ok(TestCommand {
            program: "ctest".into(),
            args: vec![
                "--output-on-failure".into(),
                "--no-tests=error".into(),
                "-R".into(),
                selector.filter.clone(),
            ],
            cwd: Some(root.join(build_path)),
        })
    }

    fn scan_legacy_bindings(
        &self,
        _workspace: &RunnerWorkspace,
    ) -> SpecResult<HashMap<String, String>> {
        Ok(HashMap::new())
    }

    fn recognized_config_keys(&self) -> &'static [&'static str] {
        &["build_dir"]
    }
}

fn checked_build_dir(build_dir: &str) -> SpecResult<&Path> {
    if build_dir.trim().is_empty() {
        return Err(SpecError::Verification(
            "ctest runner_config.build_dir must not be empty".into(),
        ));
    }

    let path = Path::new(build_dir);
    if path.is_absolute() {
        return Err(SpecError::Verification(format!(
            "ctest runner_config.build_dir `{build_dir}` must be repository-relative"
        )));
    }
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(SpecError::Verification(format!(
            "ctest runner_config.build_dir `{build_dir}` must not contain parent-directory components"
        )));
    }

    Ok(path)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use crate::spec_core::TestSelector;

    use super::super::{RunnerWorkspace, TestRunner, WorkspaceMarkers};
    use super::CTestRunner;

    #[test]
    fn ctest_runner_builds_checked_command() {
        let root = PathBuf::from("/tmp/ctest-project");
        for (config, expected_build_dir) in [
            (BTreeMap::new(), "build"),
            (
                BTreeMap::from([(
                    "build_dir".to_string(),
                    ".specwright/ctest-build".to_string(),
                )]),
                ".specwright/ctest-build",
            ),
        ] {
            let workspace = workspace(Some(root.clone()), config);
            let command = CTestRunner
                .build_test_command(
                    &workspace,
                    &TestSelector {
                        package: Some("native".into()),
                        filter: "^native_rules$".into(),
                        level: Some("unit".into()),
                        test_double: None,
                        targets: None,
                    },
                )
                .unwrap();

            assert_eq!(command.program, "ctest");
            assert_eq!(command.cwd, Some(root.join(expected_build_dir)));
            assert_eq!(
                command.args,
                vec![
                    "--output-on-failure".to_string(),
                    "--no-tests=error".to_string(),
                    "-R".to_string(),
                    "^native_rules$".to_string(),
                ]
            );
            assert!(!command.args.iter().any(|arg| arg == "native"));
        }
    }

    #[test]
    fn ctest_runner_rejects_invalid_build_dir_matrix() {
        for build_dir in [
            "",
            "   ",
            "/tmp/ctest-build",
            "../build",
            "nested/../../build",
        ] {
            let workspace = workspace(
                Some(PathBuf::from("/tmp/ctest-project")),
                BTreeMap::from([("build_dir".to_string(), build_dir.to_string())]),
            );
            let error = CTestRunner
                .build_test_command(&workspace, &TestSelector::filter_only("native_rules"))
                .unwrap_err()
                .to_string();

            assert!(error.contains("build_dir"), "unexpected error: {error}");
        }
    }

    #[test]
    fn ctest_runner_rejects_missing_workspace_root() {
        let error = CTestRunner
            .build_test_command(
                &workspace(None, BTreeMap::new()),
                &TestSelector::filter_only("native_rules"),
            )
            .unwrap_err()
            .to_string();

        assert!(error.contains("workspace root"));
    }

    fn workspace(root: Option<PathBuf>, config: BTreeMap<String, String>) -> RunnerWorkspace {
        RunnerWorkspace::new_without_metadata(
            root,
            Vec::new(),
            config,
            WorkspaceMarkers::from_files(["CMakeLists.txt"]),
            Vec::new(),
        )
    }
}
