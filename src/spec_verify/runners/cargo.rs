use std::collections::HashMap;

use crate::spec_core::{SpecResult, TestSelector};

use super::{RunnerWorkspace, TestCommand, TestRunner, WorkspaceMarkers};

/// Built-in Cargo test runner.
pub struct CargoRunner;

impl TestRunner for CargoRunner {
    fn id(&self) -> &'static str {
        "cargo"
    }

    fn detect(&self, markers: &WorkspaceMarkers) -> bool {
        markers.contains("Cargo.toml")
    }

    fn source_extensions(&self) -> &'static [&'static str] {
        &["rs"]
    }

    fn build_test_command(
        &self,
        _workspace: &RunnerWorkspace,
        selector: &TestSelector,
    ) -> SpecResult<TestCommand> {
        let mut args = vec!["test".to_string(), "-q".to_string()];
        if let Some(package) = &selector.package {
            args.push("-p".to_string());
            args.push(package.clone());
        }
        args.push(selector.filter.clone());
        Ok(TestCommand {
            program: "cargo".to_string(),
            args,
        })
    }

    fn scan_legacy_bindings(
        &self,
        workspace: &RunnerWorkspace,
    ) -> SpecResult<HashMap<String, String>> {
        let mut bindings = HashMap::new();
        for source in &workspace.source_files {
            if source.path.extension().is_none_or(|ext| ext != "rs") {
                continue;
            }
            for (scenario, test_name) in extract_bindings(&source.content) {
                bindings.entry(scenario).or_insert(test_name);
            }
        }
        Ok(bindings)
    }
}

pub fn extract_bindings(source: &str) -> Vec<(String, String)> {
    let mut bindings = Vec::new();
    let mut pending_specs = Vec::new();
    let mut saw_test_attr = false;

    for line in source.lines() {
        let trimmed = line.trim();

        if let Some(spec_name) = trimmed
            .strip_prefix("// @spec:")
            .or_else(|| trimmed.strip_prefix("/// @spec:"))
        {
            pending_specs.push(spec_name.trim().to_string());
            continue;
        }

        if trimmed.starts_with("#[test]") || trimmed.starts_with("#[tokio::test") {
            saw_test_attr = true;
            continue;
        }

        if saw_test_attr && trimmed.starts_with("fn ") {
            if let Some(test_name) = extract_fn_name(trimmed) {
                for spec_name in pending_specs.drain(..) {
                    bindings.push((spec_name, test_name.clone()));
                }
            }
            saw_test_attr = false;
            continue;
        }

        if !trimmed.starts_with("#[") && !trimmed.is_empty() {
            pending_specs.clear();
            saw_test_attr = false;
        }
    }

    bindings
}

fn extract_fn_name(line: &str) -> Option<String> {
    let rest = line.strip_prefix("fn ")?;
    let name = rest.split('(').next()?.trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::path::PathBuf;

    use crate::spec_core::TestSelector;

    use super::super::{RunnerSourceFile, RunnerWorkspace, TestRunner, WorkspaceMarkers};
    use super::CargoRunner;

    #[test]
    fn test_build_test_command_cargo_argv_baseline() {
        let runner = CargoRunner;
        let workspace = RunnerWorkspace::for_test(".");
        let selectors = [
            (
                TestSelector::filter_only("filter_only"),
                vec!["test", "-q", "filter_only"],
            ),
            (
                TestSelector {
                    package: Some("agent-spec".into()),
                    filter: "with_package".into(),
                    level: None,
                    test_double: None,
                    targets: None,
                },
                vec!["test", "-q", "-p", "agent-spec", "with_package"],
            ),
            (
                TestSelector {
                    package: None,
                    filter: "with_level".into(),
                    level: Some("unit".into()),
                    test_double: None,
                    targets: None,
                },
                vec!["test", "-q", "with_level"],
            ),
        ];

        for (selector, expected) in selectors {
            let command = runner.build_test_command(&workspace, &selector).unwrap();
            assert_eq!(command.program, "cargo");
            assert_eq!(
                command.args,
                expected
                    .into_iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn test_duplicate_legacy_binding_first_wins() {
        let runner = CargoRunner;
        let workspace = RunnerWorkspace::new_without_metadata(
            Some(PathBuf::from(".")),
            Vec::new(),
            Default::default(),
            WorkspaceMarkers::from_files(["Cargo.toml"]),
            vec![RunnerSourceFile {
                path: PathBuf::from("src/lib.rs"),
                content: r#"
// @spec: same scenario
#[test]
fn first_test() {}

// @spec: same scenario
#[test]
fn second_test() {}
"#
                .to_string(),
            }],
        );

        let bindings = runner.scan_legacy_bindings(&workspace).unwrap();
        assert_eq!(
            bindings.get("same scenario"),
            Some(&"first_test".to_string())
        );
    }
}
