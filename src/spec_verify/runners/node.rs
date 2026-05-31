use std::collections::HashMap;
use std::path::Path;

use crate::spec_core::{SpecError, SpecResult, TestSelector};

use super::{
    NodeProjectMetadata, PreflightOutcome, RunnerWorkspace, TestCommand, TestRunner,
    WorkspaceMarkers,
};

/// Generic Node and TypeScript package-script runner.
pub struct NodeRunner;

impl TestRunner for NodeRunner {
    fn id(&self) -> &'static str {
        "node"
    }

    fn detect(&self, markers: &WorkspaceMarkers) -> bool {
        markers.contains("package.json")
    }

    fn source_extensions(&self) -> &'static [&'static str] {
        &["ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs"]
    }

    fn ignored_source_dirs(&self) -> &'static [&'static str] {
        &["node_modules", "dist", "coverage", "playwright-report"]
    }

    fn build_test_command(
        &self,
        workspace: &RunnerWorkspace,
        selector: &TestSelector,
    ) -> SpecResult<TestCommand> {
        let metadata = workspace.metadata.node.as_ref().ok_or_else(|| {
            SpecError::Verification(
                "node runner requires RunnerWorkspace.metadata.node from package.json probe".into(),
            )
        })?;
        let script = script_for_selector(workspace, metadata, selector)?;
        let package_manager = metadata.package_manager.manager;
        let mut args = package_manager_run_prefix(workspace, selector)?;
        args.push(script.to_string());
        args.extend(filter_args(workspace, selector)?);

        Ok(TestCommand {
            program: package_manager.as_str().to_string(),
            args,
        })
    }

    fn scan_legacy_bindings(
        &self,
        workspace: &RunnerWorkspace,
    ) -> SpecResult<HashMap<String, String>> {
        let mut bindings = HashMap::new();
        for source in &workspace.source_files {
            if !is_node_source(&source.path) {
                continue;
            }
            for (scenario, test_name) in extract_node_bindings(&source.content) {
                bindings.entry(scenario).or_insert(test_name);
            }
        }
        Ok(bindings)
    }

    fn preflight(
        &self,
        workspace: &RunnerWorkspace,
        selector: &TestSelector,
    ) -> SpecResult<PreflightOutcome> {
        let metadata = workspace.metadata.node.as_ref().ok_or_else(|| {
            SpecError::Verification(
                "node runner requires RunnerWorkspace.metadata.node from package.json probe".into(),
            )
        })?;
        let program = metadata.package_manager.manager.as_str();
        if !program_on_path(program) {
            return Ok(PreflightOutcome::MissingCapability {
                capability: program.to_string(),
                reason: format!("`{program}` executable was not found on PATH"),
            });
        }

        if selector.level.as_deref() == Some("e2e") {
            return Ok(PreflightOutcome::MissingCapability {
                capability: "playwright-browser".into(),
                reason:
                    "e2e browser capability is opt-in and is not part of the default close gate"
                        .into(),
            });
        }

        Ok(PreflightOutcome::Ready)
    }

    fn recognized_config_keys(&self) -> &'static [&'static str] {
        &[
            "package_manager",
            "unit_script",
            "typecheck_script",
            "lint_script",
            "build_script",
            "e2e_script",
            "unit_filter_style",
            "workspace_filter",
        ]
    }
}

fn script_for_selector<'a>(
    workspace: &'a RunnerWorkspace,
    metadata: &'a NodeProjectMetadata,
    selector: &TestSelector,
) -> SpecResult<&'a str> {
    let (config_key, default_script) = match selector.level.as_deref().unwrap_or("unit") {
        "unit" => ("unit_script", "test"),
        "typecheck" => ("typecheck_script", "typecheck"),
        "lint" => ("lint_script", "lint"),
        "build" => ("build_script", "build"),
        "e2e" => ("e2e_script", "e2e"),
        other => {
            return Err(SpecError::Verification(format!(
                "unsupported node test level `{other}`; expected unit, typecheck, lint, build, or e2e"
            )));
        }
    };
    let script = workspace
        .config
        .get(config_key)
        .map(String::as_str)
        .unwrap_or(default_script);
    if !metadata.scripts.contains(script) {
        return Err(SpecError::Verification(format!(
            "node runner requires package.json script `{script}` for level `{}`",
            selector.level.as_deref().unwrap_or("unit")
        )));
    }
    Ok(script)
}

fn package_manager_run_prefix(
    workspace: &RunnerWorkspace,
    selector: &TestSelector,
) -> SpecResult<Vec<String>> {
    let package = selector.package.as_deref();
    let configured_filter = workspace.config.get("workspace_filter").map(String::as_str);
    if package.is_some() || configured_filter.is_some() {
        let mut details = Vec::new();
        if let Some(package) = package {
            details.push(format!("Package `{package}`"));
        }
        if let Some(configured) = configured_filter {
            details.push(format!("runner_config.workspace_filter `{configured}`"));
        }
        return Err(SpecError::Verification(format!(
            "node runner v1 does not support workspace filters ({}); remove Package and runner_config.workspace_filter",
            details.join(", ")
        )));
    }

    Ok(vec!["run".to_string()])
}

fn filter_args(workspace: &RunnerWorkspace, selector: &TestSelector) -> SpecResult<Vec<String>> {
    let level = selector.level.as_deref().unwrap_or("unit");
    if level != "unit" {
        if selector.filter != "-" {
            return Err(SpecError::Verification(format!(
                "node level `{level}` requires the no-filter sentinel `Filter: -`"
            )));
        }
        return Ok(Vec::new());
    }

    let style = workspace
        .config
        .get("unit_filter_style")
        .map(String::as_str)
        .unwrap_or("none");
    if selector.filter == "-" {
        return Ok(Vec::new());
    }

    let pattern = escape_regex_literal(&selector.filter);
    let args = match style {
        "vitest" => vec!["--".to_string(), "-t".to_string(), pattern],
        "jest" => vec!["--".to_string(), "--testNamePattern".to_string(), pattern],
        "playwright" => vec!["--".to_string(), "--grep".to_string(), pattern],
        "none" => {
            return Err(SpecError::Verification(
                "node unit_filter_style `none` requires the no-filter sentinel `Filter: -`".into(),
            ));
        }
        other => {
            return Err(SpecError::Verification(format!(
                "unsupported node unit_filter_style `{other}`; expected vitest, jest, playwright, or none"
            )));
        }
    };
    Ok(args)
}

fn escape_regex_literal(input: &str) -> String {
    let mut escaped = String::new();
    for ch in input.chars() {
        if matches!(
            ch,
            '\\' | '.' | '*' | '+' | '?' | '^' | '$' | '{' | '}' | '(' | ')' | '|' | '[' | ']'
        ) {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}

fn extract_node_bindings(source: &str) -> Vec<(String, String)> {
    let mut bindings = Vec::new();
    let mut pending_specs = Vec::new();

    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(spec_name) = trimmed.strip_prefix("// @spec:") {
            pending_specs.push(spec_name.trim().to_string());
            continue;
        }

        if pending_specs.is_empty() {
            continue;
        }

        if let Some(test_name) = extract_node_test_title(trimmed) {
            for spec_name in pending_specs.drain(..) {
                bindings.push((spec_name, test_name.clone()));
            }
            continue;
        }

        if !trimmed.is_empty() && !trimmed.starts_with("//") {
            pending_specs.clear();
        }
    }

    bindings
}

fn extract_node_test_title(line: &str) -> Option<String> {
    for prefix in ["test", "it", "describe"] {
        let Some(rest) = line.strip_prefix(prefix) else {
            continue;
        };
        let rest = rest.trim_start();
        let Some(rest) = rest.strip_prefix('(') else {
            continue;
        };
        if let Some(title) = extract_quoted_title(rest) {
            return Some(title);
        }
    }
    None
}

fn extract_quoted_title(input: &str) -> Option<String> {
    let mut chars = input.chars();
    let quote = chars.next()?;
    if !matches!(quote, '"' | '\'' | '`') {
        return None;
    }

    let mut title = String::new();
    let mut escaped = false;
    for ch in chars {
        if escaped {
            title.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == quote {
            return Some(title);
        }
        title.push(ch);
    }
    None
}

fn is_node_source(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension,
                "ts" | "tsx" | "mts" | "cts" | "js" | "jsx" | "mjs" | "cjs"
            )
        })
}

fn program_on_path(program: &str) -> bool {
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&paths).any(|dir| {
        let candidate = dir.join(program);
        candidate.is_file()
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    use crate::spec_core::TestSelector;
    use crate::spec_verify::{
        RunnerSourceFile, RunnerWorkspace, RunnerWorkspaceMetadata, TestRunner, WorkspaceMarkers,
    };

    use super::super::{
        NodePackageManager, NodePackageManagerDecision, NodePackageManagerSource,
        NodeProjectMetadata,
    };
    use super::NodeRunner;

    #[test]
    fn test_node_unit_script_argv_matrix_for_package_managers() {
        for (manager, program) in [
            (NodePackageManager::Npm, "npm"),
            (NodePackageManager::Pnpm, "pnpm"),
            (NodePackageManager::Yarn, "yarn"),
            (NodePackageManager::Bun, "bun"),
        ] {
            let workspace = workspace(
                manager,
                BTreeMap::from([("unit_filter_style".to_string(), "vitest".to_string())]),
                ["test"],
            );
            let command = NodeRunner
                .build_test_command(&workspace, &unit_selector("renders dashboard"))
                .unwrap();

            assert_eq!(command.program, program);
            assert_eq!(
                command.args,
                vec![
                    "run".to_string(),
                    "test".to_string(),
                    "--".to_string(),
                    "-t".to_string(),
                    "renders dashboard".to_string()
                ]
            );
        }
    }

    #[test]
    fn test_node_unit_filter_style_argv_matrix() {
        for (style, expected) in [
            ("vitest", vec!["--", "-t", "renders dashboard"]),
            ("jest", vec!["--", "--testNamePattern", "renders dashboard"]),
            ("playwright", vec!["--", "--grep", "renders dashboard"]),
        ] {
            let workspace = workspace(
                NodePackageManager::Npm,
                BTreeMap::from([("unit_filter_style".to_string(), style.to_string())]),
                ["test"],
            );
            let command = NodeRunner
                .build_test_command(&workspace, &unit_selector("renders dashboard"))
                .unwrap();

            assert_eq!(
                command.args[2..],
                expected
                    .into_iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>()
            );
        }

        let workspace = workspace(NodePackageManager::Npm, BTreeMap::new(), ["test"]);
        let err = NodeRunner
            .build_test_command(&workspace, &unit_selector("renders dashboard"))
            .unwrap_err()
            .to_string();
        assert!(err.contains("unit_filter_style"));
        assert!(err.contains("Filter: -"));
    }

    #[test]
    fn test_node_workspace_filters_are_out_of_scope_for_v1() {
        let base_workspace = workspace(NodePackageManager::Npm, BTreeMap::new(), ["test"]);
        let err = NodeRunner
            .build_test_command(&base_workspace, &package_selector("web"))
            .unwrap_err()
            .to_string();

        assert!(err.contains("workspace filters"));
        assert!(err.contains("Package `web`"));

        let configured_workspace = workspace(
            NodePackageManager::Npm,
            BTreeMap::from([("workspace_filter".to_string(), "web".to_string())]),
            ["test"],
        );
        let err = NodeRunner
            .build_test_command(&configured_workspace, &unit_selector("-"))
            .unwrap_err()
            .to_string();

        assert!(err.contains("workspace filters"));
        assert!(err.contains("runner_config.workspace_filter `web`"));
    }

    #[test]
    fn test_node_filter_sentinel_for_typecheck_lint_build() {
        let workspace = workspace(
            NodePackageManager::Npm,
            BTreeMap::new(),
            ["typecheck", "lint", "build"],
        );
        for level in ["typecheck", "lint", "build"] {
            let command = NodeRunner
                .build_test_command(&workspace, &level_selector(level, "-"))
                .unwrap();
            assert_eq!(command.args, vec!["run".to_string(), level.to_string()]);

            let err = NodeRunner
                .build_test_command(&workspace, &level_selector(level, "renders dashboard"))
                .unwrap_err()
                .to_string();
            assert!(err.contains("Filter: -"));
        }
    }

    #[test]
    fn test_node_default_script_mapping_for_supported_levels() {
        let default_workspace = workspace(
            NodePackageManager::Npm,
            BTreeMap::new(),
            ["test", "typecheck", "lint", "build", "e2e"],
        );
        for (level, script) in [
            ("unit", "test"),
            ("typecheck", "typecheck"),
            ("lint", "lint"),
            ("build", "build"),
            ("e2e", "e2e"),
        ] {
            let command = NodeRunner
                .build_test_command(&default_workspace, &level_selector(level, "-"))
                .unwrap();
            assert_eq!(command.args, vec!["run".to_string(), script.to_string()]);
        }

        let override_workspace = workspace(
            NodePackageManager::Npm,
            BTreeMap::from([
                ("unit_script".to_string(), "unit-ci".to_string()),
                ("typecheck_script".to_string(), "check-types".to_string()),
                ("lint_script".to_string(), "lint-ci".to_string()),
                ("build_script".to_string(), "compile".to_string()),
                ("e2e_script".to_string(), "browser-ci".to_string()),
            ]),
            ["unit-ci", "check-types", "lint-ci", "compile", "browser-ci"],
        );
        for (level, script) in [
            ("unit", "unit-ci"),
            ("typecheck", "check-types"),
            ("lint", "lint-ci"),
            ("build", "compile"),
            ("e2e", "browser-ci"),
        ] {
            let command = NodeRunner
                .build_test_command(&override_workspace, &level_selector(level, "-"))
                .unwrap();
            assert_eq!(command.args, vec!["run".to_string(), script.to_string()]);
        }
    }

    #[test]
    fn test_node_missing_required_script_errors_loudly() {
        let workspace = workspace(NodePackageManager::Npm, BTreeMap::new(), ["test"]);
        let err = NodeRunner
            .build_test_command(&workspace, &level_selector("typecheck", "-"))
            .unwrap_err()
            .to_string();

        assert!(err.contains("typecheck"));
        assert!(err.contains("script"));
    }

    #[test]
    fn test_node_missing_package_manager_preflight_skips_without_spawn() {
        if super::program_on_path("bun") {
            return;
        }

        let workspace = workspace(NodePackageManager::Bun, BTreeMap::new(), ["test"]);
        let outcome = NodeRunner
            .preflight(&workspace, &level_selector("unit", "-"))
            .unwrap();

        assert_eq!(
            outcome,
            crate::spec_verify::PreflightOutcome::MissingCapability {
                capability: "bun".into(),
                reason: "`bun` executable was not found on PATH".into(),
            }
        );
    }

    #[test]
    fn test_node_e2e_level_is_opt_in_not_default_gate() {
        if !super::program_on_path("npm") {
            return;
        }

        let workspace = workspace(NodePackageManager::Npm, BTreeMap::new(), ["e2e"]);
        let outcome = NodeRunner
            .preflight(&workspace, &level_selector("e2e", "-"))
            .unwrap();

        assert_eq!(
            outcome,
            crate::spec_verify::PreflightOutcome::MissingCapability {
                capability: "playwright-browser".into(),
                reason:
                    "e2e browser capability is opt-in and is not part of the default close gate"
                        .into(),
            }
        );
    }

    #[test]
    fn test_node_filter_patterns_escape_regex_metacharacters() {
        let title = r"\.*+?^${}()|[]";
        let workspace = workspace(
            NodePackageManager::Npm,
            BTreeMap::from([("unit_filter_style".to_string(), "vitest".to_string())]),
            ["test"],
        );
        let command = NodeRunner
            .build_test_command(&workspace, &unit_selector(title))
            .unwrap();

        assert_eq!(command.args[4], r"\\\.\*\+\?\^\$\{\}\(\)\|\[\]");
    }

    #[test]
    fn test_node_binding_scanner_extracts_spec_annotations() {
        let workspace = RunnerWorkspace::new(
            Some(PathBuf::from(".")),
            Vec::new(),
            BTreeMap::new(),
            WorkspaceMarkers::from_files(["package.json"]),
            vec![RunnerSourceFile {
                path: PathBuf::from("tests/example.test.ts"),
                content: r#"
// @spec: renders dashboard
test("renders dashboard", () => {})

// @spec: shows summary
it('shows summary', () => {})

// @spec: dashboard suite
describe(`dashboard suite`, () => {})

// @spec: ignored non adjacent
const helper = true
"#
                .to_string(),
            }],
            RunnerWorkspaceMetadata::default(),
        );

        let bindings = NodeRunner.scan_legacy_bindings(&workspace).unwrap();

        assert_eq!(
            bindings.get("renders dashboard"),
            Some(&"renders dashboard".to_string())
        );
        assert_eq!(
            bindings.get("shows summary"),
            Some(&"shows summary".to_string())
        );
        assert_eq!(
            bindings.get("dashboard suite"),
            Some(&"dashboard suite".to_string())
        );
        assert!(!bindings.contains_key("ignored non adjacent"));
    }

    #[test]
    fn test_node_binding_scanner_first_binding_wins() {
        let workspace = RunnerWorkspace::new(
            Some(PathBuf::from(".")),
            Vec::new(),
            BTreeMap::new(),
            WorkspaceMarkers::from_files(["package.json"]),
            vec![
                RunnerSourceFile {
                    path: PathBuf::from("tests/a.test.ts"),
                    content: "// @spec: same scenario\ntest(\"first\", () => {})\n".to_string(),
                },
                RunnerSourceFile {
                    path: PathBuf::from("tests/b.test.ts"),
                    content: "// @spec: same scenario\ntest(\"second\", () => {})\n".to_string(),
                },
            ],
            RunnerWorkspaceMetadata::default(),
        );

        let bindings = NodeRunner.scan_legacy_bindings(&workspace).unwrap();

        assert_eq!(bindings.get("same scenario"), Some(&"first".to_string()));
    }

    fn workspace(
        manager: NodePackageManager,
        config: BTreeMap<String, String>,
        scripts: impl IntoIterator<Item = &'static str>,
    ) -> RunnerWorkspace {
        RunnerWorkspace::new(
            Some(PathBuf::from(".")),
            Vec::new(),
            config,
            WorkspaceMarkers::from_files(["package.json"]),
            Vec::new(),
            RunnerWorkspaceMetadata {
                node: Some(NodeProjectMetadata {
                    package_manager: NodePackageManagerDecision {
                        manager,
                        source: NodePackageManagerSource::RunnerConfig,
                    },
                    scripts: scripts.into_iter().map(str::to_string).collect(),
                    package_json_package_manager: None,
                    lockfiles: BTreeSet::new(),
                }),
            },
        )
    }

    fn unit_selector(filter: &str) -> TestSelector {
        TestSelector {
            package: None,
            filter: filter.to_string(),
            level: Some("unit".into()),
            test_double: None,
            targets: None,
        }
    }

    fn level_selector(level: &str, filter: &str) -> TestSelector {
        TestSelector {
            package: None,
            filter: filter.to_string(),
            level: Some(level.to_string()),
            test_double: None,
            targets: None,
        }
    }

    fn package_selector(package: &str) -> TestSelector {
        TestSelector {
            package: Some(package.to_string()),
            filter: "-".to_string(),
            level: Some("unit".to_string()),
            test_double: None,
            targets: None,
        }
    }
}
