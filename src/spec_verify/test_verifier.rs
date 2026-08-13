use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::spec_core::{
    Evidence, ReviewMode, Scenario, ScenarioResult, SpecError, SpecResult, StepVerdict,
    TestSelector, Verdict,
};

use super::{RunnerOutput, VerificationContext, Verifier};

pub struct TestVerifier;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BindingSource {
    ExplicitScenarioSelector,
    LegacyComment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TestBinding {
    selector: TestSelector,
    source: BindingSource,
    slot_index: Option<usize>,
}

impl Verifier for TestVerifier {
    fn name(&self) -> &str {
        "test"
    }

    #[allow(clippy::too_many_lines)] // Exception: legacy Cargo verifier path; runner refactor will split this under task_3455b7d6.
    fn verify(&self, ctx: &VerificationContext) -> SpecResult<Vec<ScenarioResult>> {
        let legacy_bindings = scan_legacy_bindings(ctx)?;
        let mut results = Vec::new();

        for scenario in &ctx.resolved_spec.all_scenarios {
            let Some(binding) = resolve_test_binding(scenario, &legacy_bindings) else {
                continue;
            };
            let slot = slot_for_binding(ctx, &binding);

            if let super::PreflightOutcome::MissingCapability { capability, reason } =
                slot.runner
                    .preflight(&slot.runner_workspace, &binding.selector)?
            {
                results.push(skip_for_missing_capability(scenario, &capability, &reason));
                continue;
            }

            let started = Instant::now();
            let command = slot
                .runner
                .build_test_command(&slot.runner_workspace, &binding.selector)?;
            let Some(current_dir) = command.cwd.as_ref().or(slot.runner_workspace.root.as_ref())
            else {
                continue;
            };
            let output = Command::new(&command.program)
                .args(&command.args)
                .current_dir(current_dir)
                .output()
                .map_err(|err| {
                    SpecError::Verification(format!(
                        "failed to run {} test command: {err}",
                        slot.runner.id()
                    ))
                })?;
            let duration_ms = started.elapsed().as_millis() as u64;

            let runner_output = RunnerOutput {
                status_success: output.status.success(),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            };
            let combined = runner_output.combined();
            let interpretation = slot
                .runner
                .interpret_output(&binding.selector, &runner_output);
            let verdict = if interpretation.verdict == Verdict::Pass
                && scenario.review == ReviewMode::Human
            {
                Verdict::PendingReview
            } else {
                interpretation.verdict
            };
            let selector_label = binding.selector.label();
            let reason = append_cargo_route_hint(
                append_runner_warnings(
                    interpretation.reason.unwrap_or_else(|| {
                        default_test_reason(&binding, &selector_label, runner_output.status_success)
                    }),
                    &interpretation.warnings,
                ),
                slot.runner.id(),
                &slot.runner_workspace,
                &binding.selector,
            );

            let step_results = scenario
                .steps
                .iter()
                .map(|step| StepVerdict {
                    step_text: step.text.clone(),
                    verdict,
                    reason: reason.clone(),
                })
                .collect();

            results.push(ScenarioResult {
                scenario_name: scenario.name.clone(),
                verdict,
                step_results,
                evidence: vec![Evidence::TestOutput {
                    test_name: selector_label,
                    stdout: combined,
                    passed: output.status.success(),
                    command_program: command_program_evidence(slot.runner.id(), &command.program),
                    package: binding.selector.package.clone(),
                    level: binding.selector.level.clone(),
                    test_double: binding.selector.test_double.clone(),
                    targets: binding.selector.targets.clone(),
                }],
                duration_ms,
            });
        }

        Ok(results)
    }
}

fn default_test_reason(
    binding: &TestBinding,
    selector_label: &str,
    status_success: bool,
) -> String {
    if status_success {
        match binding.source {
            BindingSource::ExplicitScenarioSelector => {
                format!("covered by explicit test `{selector_label}`")
            }
            BindingSource::LegacyComment => {
                format!("covered by legacy @spec test `{selector_label}`")
            }
        }
    } else {
        match binding.source {
            BindingSource::ExplicitScenarioSelector => {
                format!("explicit test `{selector_label}` failed")
            }
            BindingSource::LegacyComment => {
                format!("legacy @spec test `{selector_label}` failed")
            }
        }
    }
}

fn append_runner_warnings(reason: String, warnings: &[String]) -> String {
    if warnings.is_empty() {
        reason
    } else {
        format!(
            "{reason}; runner warning: {}",
            warnings.join("; runner warning: ")
        )
    }
}

fn append_cargo_route_hint(
    reason: String,
    runner_id: &str,
    workspace: &super::RunnerWorkspace,
    selector: &TestSelector,
) -> String {
    let Some(package) = selector.package.as_deref() else {
        return reason;
    };
    if runner_id != "cargo" || !reason.contains("matched zero tests") {
        return reason;
    }
    let Some(root) = workspace.root.as_deref() else {
        return reason;
    };
    let Some(package_root) = find_node_package_token_root_io(root, package) else {
        return reason;
    };
    let display_path = package_root
        .strip_prefix(root)
        .unwrap_or(package_root.as_path())
        .display();
    format!(
        "{reason}; Package `{package}` looks like a Node package at `{display_path}`; declare it under `runners:`"
    )
}

fn find_node_package_token_root_io(root: &Path, package: &str) -> Option<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if !file_type.is_dir() {
                continue;
            }
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if matches!(
                name.as_ref(),
                ".git" | "target" | "node_modules" | "dist" | "coverage"
            ) {
                continue;
            }
            if name == package && path.join("package.json").is_file() {
                return Some(path);
            }
            stack.push(path);
        }
    }
    None
}

fn scan_legacy_bindings(ctx: &VerificationContext) -> SpecResult<HashMap<String, TestBinding>> {
    let mut legacy_bindings = HashMap::new();
    for (slot_index, slot) in ctx.routed_contexts.slots().iter().enumerate() {
        for (scenario, selector) in slot.runner.scan_legacy_bindings(&slot.runner_workspace)? {
            legacy_bindings.entry(scenario).or_insert(TestBinding {
                selector: TestSelector::filter_only(selector),
                source: BindingSource::LegacyComment,
                slot_index: Some(slot_index),
            });
        }
    }
    Ok(legacy_bindings)
}

fn slot_for_binding<'a>(
    ctx: &'a VerificationContext,
    binding: &TestBinding,
) -> &'a super::RunnerSlot {
    binding
        .slot_index
        .and_then(|index| ctx.routed_contexts.slot_at(index))
        .unwrap_or_else(|| {
            ctx.routed_contexts
                .slot_for(binding.selector.package.as_deref())
        })
}

fn resolve_test_binding(
    scenario: &Scenario,
    legacy_bindings: &HashMap<String, TestBinding>,
) -> Option<TestBinding> {
    if let Some(selector) = scenario.test_selector.as_ref() {
        return Some(TestBinding {
            selector: selector.clone(),
            source: BindingSource::ExplicitScenarioSelector,
            slot_index: None,
        });
    }

    legacy_bindings.get(&scenario.name).cloned()
}

fn command_program_evidence(runner_id: &str, program: &str) -> Option<String> {
    if runner_id == "cargo" && program == "cargo" {
        None
    } else {
        Some(program.to_string())
    }
}

fn skip_for_missing_capability(
    scenario: &Scenario,
    capability: &str,
    reason: &str,
) -> ScenarioResult {
    let skip_reason = format!("{capability}: {reason}");
    let step_results = scenario
        .steps
        .iter()
        .map(|step| StepVerdict {
            step_text: step.text.clone(),
            verdict: Verdict::Skip,
            reason: skip_reason.clone(),
        })
        .collect();

    ScenarioResult {
        scenario_name: scenario.name.clone(),
        verdict: Verdict::Skip,
        step_results,
        evidence: Vec::new(),
        duration_ms: 0,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::collections::{BTreeMap, HashMap};
    use std::sync::Arc;

    use crate::spec_core::{
        Evidence, ResolvedSpec, Scenario, Section, Span, SpecDocument, SpecLevel, SpecMeta, Step,
        StepKind, TestSelector, Verdict,
    };
    use crate::spec_verify::{
        AiMode, PreflightOutcome, ResolutionSource, RoutedContexts, RunnerOutput,
        RunnerOutputInterpretation, RunnerResolution, RunnerSlot, RunnerWorkspace, TestCommand,
        TestRunner, VerificationContext, Verifier, WorkspaceMarkers,
    };

    use super::{BindingSource, TestBinding, TestVerifier, resolve_test_binding};

    #[test]
    fn extracts_spec_bindings_from_test_comments() {
        let source = r#"
// @spec: 场景一
// @spec: 场景二
#[test]
fn test_example() {}
"#;

        let bindings = crate::spec_verify::extract_bindings(source);
        assert_eq!(bindings.len(), 2);
        assert_eq!(
            bindings[0],
            ("场景一".to_string(), "test_example".to_string())
        );
        assert_eq!(
            bindings[1],
            ("场景二".to_string(), "test_example".to_string())
        );
    }

    #[test]
    fn ignores_comments_not_followed_by_a_test() {
        let source = r#"
// @spec: 场景一
fn helper() {}
"#;

        assert!(crate::spec_verify::extract_bindings(source).is_empty());
    }

    #[test]
    fn test_explicit_scenario_selector_takes_precedence_over_legacy_comment_binding() {
        let scenario = Scenario {
            name: "场景一".into(),
            steps: Vec::new(),
            test_selector: Some(TestSelector::filter_only(
                "test_explicit_scenario_selector_takes_precedence_over_legacy_comment_binding",
            )),
            verification: Default::default(),
            evidence: None,
            tags: Vec::new(),
            review: Default::default(),
            mode: Default::default(),
            depends_on: vec![],
            span: Span::default(),
        };
        let legacy = HashMap::from([("场景一".to_string(), legacy_binding("legacy_test_name", 0))]);

        let binding = resolve_test_binding(&scenario, &legacy).unwrap();
        assert_eq!(
            binding.selector,
            TestSelector::filter_only(
                "test_explicit_scenario_selector_takes_precedence_over_legacy_comment_binding"
            )
        );
        assert_eq!(binding.source, BindingSource::ExplicitScenarioSelector);
        assert_eq!(binding.slot_index, None);
    }

    #[test]
    fn test_legacy_comment_binding_is_used_when_no_explicit_selector_exists() {
        let scenario = Scenario {
            name: "场景一".into(),
            steps: Vec::new(),
            test_selector: None,
            verification: Default::default(),
            evidence: None,
            tags: Vec::new(),
            review: Default::default(),
            mode: Default::default(),
            depends_on: vec![],
            span: Span::default(),
        };
        let legacy = HashMap::from([(
            "场景一".to_string(),
            legacy_binding(
                "test_legacy_comment_binding_is_used_when_no_explicit_selector_exists",
                1,
            ),
        )]);

        let binding = resolve_test_binding(&scenario, &legacy).unwrap();
        assert_eq!(
            binding.selector,
            TestSelector::filter_only(
                "test_legacy_comment_binding_is_used_when_no_explicit_selector_exists"
            )
        );
        assert_eq!(binding.source, BindingSource::LegacyComment);
        assert_eq!(binding.slot_index, Some(1));
    }

    fn legacy_binding(selector: &str, slot_index: usize) -> TestBinding {
        TestBinding {
            selector: TestSelector::filter_only(selector),
            source: BindingSource::LegacyComment,
            slot_index: Some(slot_index),
        }
    }

    #[test]
    fn test_cargo_command_program_evidence_is_omitted_for_json_compatibility() {
        assert_eq!(super::command_program_evidence("cargo", "cargo"), None);
        assert_eq!(
            super::command_program_evidence("maven", "./mvnw"),
            Some("./mvnw".to_string())
        );
        assert_eq!(
            super::command_program_evidence("gradle", "./gradlew"),
            Some("./gradlew".to_string())
        );
    }

    #[test]
    fn test_node_command_program_evidence_records_package_manager() {
        assert_eq!(
            super::command_program_evidence("node", "pnpm"),
            Some("pnpm".to_string())
        );
    }

    struct MissingAdbRunner;

    impl TestRunner for MissingAdbRunner {
        fn id(&self) -> &'static str {
            "android"
        }

        fn detect(&self, _markers: &WorkspaceMarkers) -> bool {
            true
        }

        fn build_test_command(
            &self,
            _workspace: &RunnerWorkspace,
            _selector: &TestSelector,
        ) -> crate::spec_core::SpecResult<TestCommand> {
            panic!("preflight skip should avoid spawning a command")
        }

        fn scan_legacy_bindings(
            &self,
            _workspace: &RunnerWorkspace,
        ) -> crate::spec_core::SpecResult<HashMap<String, String>> {
            Ok(HashMap::new())
        }

        fn preflight(
            &self,
            _workspace: &RunnerWorkspace,
            selector: &TestSelector,
        ) -> crate::spec_core::SpecResult<PreflightOutcome> {
            if selector.level.as_deref() == Some("instrumented") {
                return Ok(PreflightOutcome::MissingCapability {
                    capability: "adb-device".into(),
                    reason: "adb devices did not report an active device".into(),
                });
            }
            Ok(PreflightOutcome::Ready)
        }
    }

    fn android_missing_adb_context() -> VerificationContext {
        let scenario = Scenario {
            name: "Android instrumented".into(),
            steps: vec![Step {
                kind: StepKind::Then,
                text: "instrumented test is skipped".into(),
                params: vec![],
                table: vec![],
                span: Span::line(1),
            }],
            test_selector: Some(TestSelector {
                package: Some(":app".into()),
                filter: "com.example.ExampleTest#runs".into(),
                level: Some("instrumented".into()),
                test_double: None,
                targets: None,
            }),
            verification: Default::default(),
            evidence: None,
            tags: Vec::new(),
            review: Default::default(),
            mode: Default::default(),
            depends_on: vec![],
            span: Span::line(1),
        };
        let runner = Arc::new(MissingAdbRunner);
        let workspace = RunnerWorkspace::for_test(".");
        let resolution = RunnerResolution {
            name: "android".into(),
            source: ResolutionSource::SpecFrontmatter,
            overridden_spec: None,
            config_warnings: Vec::new(),
        };

        VerificationContext {
            code_paths: vec![".".into()],
            change_paths: vec![],
            ai_mode: AiMode::Off,
            resolved_spec: ResolvedSpec {
                task: SpecDocument {
                    meta: SpecMeta {
                        level: SpecLevel::Task,
                        name: "selector preflight".into(),
                        inherits: None,
                        lang: vec![],
                        tags: vec![],
                        runner: Some("android".into()),
                        runner_config: Default::default(),
                        runner_routes: Vec::new(),
                        depends: vec![],
                        estimate: None,
                    },
                    sections: vec![Section::AcceptanceCriteria {
                        scenarios: vec![scenario.clone()],
                        span: Span::line(1),
                    }],
                    parser_warnings: Vec::new(),
                    source_path: Default::default(),
                },
                inherited_constraints: vec![],
                inherited_decisions: vec![],
                all_scenarios: vec![scenario],
            },
            routed_contexts: RoutedContexts::default_only(RunnerSlot {
                runner: runner.clone(),
                runner_workspace: workspace.clone(),
                runner_resolution: resolution.clone(),
            }),
            runner,
            runner_workspace: workspace,
            runner_resolution: resolution,
            config_warnings: Vec::new(),
        }
    }

    #[test]
    fn test_android_preflight_missing_adb_skips_verdict() {
        let ctx = android_missing_adb_context();

        let results = TestVerifier.verify(&ctx).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].verdict, Verdict::Skip);
        assert_eq!(
            results[0].step_results[0].reason,
            "adb-device: adb devices did not report an active device"
        );
    }

    struct DefaultSkipRunner;
    struct RoutedPassRunner;

    impl TestRunner for DefaultSkipRunner {
        fn id(&self) -> &'static str {
            "cargo"
        }

        fn detect(&self, _markers: &WorkspaceMarkers) -> bool {
            true
        }

        fn build_test_command(
            &self,
            _workspace: &RunnerWorkspace,
            _selector: &TestSelector,
        ) -> crate::spec_core::SpecResult<TestCommand> {
            panic!("routed package selector should not use the default slot")
        }

        fn scan_legacy_bindings(
            &self,
            _workspace: &RunnerWorkspace,
        ) -> crate::spec_core::SpecResult<HashMap<String, String>> {
            Ok(HashMap::new())
        }

        fn preflight(
            &self,
            _workspace: &RunnerWorkspace,
            _selector: &TestSelector,
        ) -> crate::spec_core::SpecResult<PreflightOutcome> {
            Ok(PreflightOutcome::MissingCapability {
                capability: "wrong-slot".into(),
                reason: "default slot was selected".into(),
            })
        }
    }

    impl TestRunner for RoutedPassRunner {
        fn id(&self) -> &'static str {
            "node"
        }

        fn detect(&self, _markers: &WorkspaceMarkers) -> bool {
            false
        }

        fn build_test_command(
            &self,
            _workspace: &RunnerWorkspace,
            _selector: &TestSelector,
        ) -> crate::spec_core::SpecResult<TestCommand> {
            Ok(TestCommand {
                program: "true".into(),
                args: Vec::new(),
                cwd: None,
            })
        }

        fn scan_legacy_bindings(
            &self,
            _workspace: &RunnerWorkspace,
        ) -> crate::spec_core::SpecResult<HashMap<String, String>> {
            Ok(HashMap::from([(
                "Legacy admin scenario".to_string(),
                "admin_test".to_string(),
            )]))
        }
    }

    #[test]
    fn test_package_selector_uses_routed_slot() {
        let scenario = Scenario {
            name: "Admin scenario".into(),
            steps: vec![Step {
                kind: StepKind::Then,
                text: "admin package test runs".into(),
                params: vec![],
                table: vec![],
                span: Span::line(1),
            }],
            test_selector: Some(TestSelector {
                package: Some("admin".into()),
                filter: "admin_test".into(),
                level: Some("unit".into()),
                test_double: None,
                targets: None,
            }),
            verification: Default::default(),
            evidence: None,
            tags: Vec::new(),
            review: Default::default(),
            mode: Default::default(),
            depends_on: vec![],
            span: Span::line(1),
        };
        let ctx = routed_test_context(scenario);

        let results = TestVerifier.verify(&ctx).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].verdict, Verdict::Pass);
        let Evidence::TestOutput {
            command_program, ..
        } = &results[0].evidence[0]
        else {
            panic!("routed selector should produce test output evidence");
        };
        assert_eq!(command_program.as_deref(), Some("true"));
    }

    #[test]
    fn test_legacy_binding_uses_source_slot() {
        let scenario = Scenario {
            name: "Legacy admin scenario".into(),
            steps: vec![Step {
                kind: StepKind::Then,
                text: "legacy admin test runs".into(),
                params: vec![],
                table: vec![],
                span: Span::line(1),
            }],
            test_selector: None,
            verification: Default::default(),
            evidence: None,
            tags: Vec::new(),
            review: Default::default(),
            mode: Default::default(),
            depends_on: vec![],
            span: Span::line(1),
        };
        let ctx = routed_test_context(scenario);

        let results = TestVerifier.verify(&ctx).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].verdict, Verdict::Pass);
        assert_eq!(
            results[0].step_results[0].reason,
            "covered by legacy @spec test `admin_test`"
        );
    }

    struct InterpretFailRunner;
    struct CargoZeroMatchRouteHintRunner;

    impl TestRunner for InterpretFailRunner {
        fn id(&self) -> &'static str {
            "node"
        }

        fn detect(&self, _markers: &WorkspaceMarkers) -> bool {
            true
        }

        fn build_test_command(
            &self,
            _workspace: &RunnerWorkspace,
            _selector: &TestSelector,
        ) -> crate::spec_core::SpecResult<TestCommand> {
            Ok(TestCommand {
                program: "sh".into(),
                args: vec!["-c".into(), "printf 'running 0 tests\\n'".into()],
                cwd: None,
            })
        }

        fn scan_legacy_bindings(
            &self,
            _workspace: &RunnerWorkspace,
        ) -> crate::spec_core::SpecResult<HashMap<String, String>> {
            Ok(HashMap::new())
        }

        fn interpret_output(
            &self,
            selector: &TestSelector,
            output: &RunnerOutput,
        ) -> RunnerOutputInterpretation {
            assert_eq!(output.combined(), "running 0 tests\n");
            RunnerOutputInterpretation::zero_match(selector)
        }
    }

    impl TestRunner for CargoZeroMatchRouteHintRunner {
        fn id(&self) -> &'static str {
            "cargo"
        }

        fn detect(&self, _markers: &WorkspaceMarkers) -> bool {
            true
        }

        fn build_test_command(
            &self,
            _workspace: &RunnerWorkspace,
            _selector: &TestSelector,
        ) -> crate::spec_core::SpecResult<TestCommand> {
            Ok(TestCommand {
                program: "sh".into(),
                args: vec!["-c".into(), "printf 'running 0 tests\\n'".into()],
                cwd: None,
            })
        }

        fn scan_legacy_bindings(
            &self,
            _workspace: &RunnerWorkspace,
        ) -> crate::spec_core::SpecResult<HashMap<String, String>> {
            Ok(HashMap::new())
        }

        fn interpret_output(
            &self,
            selector: &TestSelector,
            _output: &RunnerOutput,
        ) -> RunnerOutputInterpretation {
            RunnerOutputInterpretation::zero_match(selector)
        }
    }

    #[test]
    fn zero_match_selector_fails_verdict() {
        let scenario = Scenario {
            name: "Node zero match".into(),
            steps: vec![Step {
                kind: StepKind::Then,
                text: "zero matched tests fail".into(),
                params: vec![],
                table: vec![],
                span: Span::line(1),
            }],
            test_selector: Some(TestSelector::filter_only("missing node test")),
            verification: Default::default(),
            evidence: None,
            tags: Vec::new(),
            review: Default::default(),
            mode: Default::default(),
            depends_on: vec![],
            span: Span::line(1),
        };
        let runner: Arc<dyn TestRunner> = Arc::new(InterpretFailRunner);
        let resolution = RunnerResolution {
            name: "node".into(),
            source: ResolutionSource::SpecFrontmatter,
            overridden_spec: None,
            config_warnings: Vec::new(),
        };
        let workspace = RunnerWorkspace::for_test(".");
        let ctx = VerificationContext {
            code_paths: vec![".".into()],
            change_paths: vec![],
            ai_mode: AiMode::Off,
            resolved_spec: resolved_spec_for_scenario(scenario),
            routed_contexts: RoutedContexts::default_only(RunnerSlot {
                runner: runner.clone(),
                runner_workspace: workspace.clone(),
                runner_resolution: resolution.clone(),
            }),
            runner,
            runner_workspace: workspace,
            runner_resolution: resolution,
            config_warnings: Vec::new(),
        };

        let results = TestVerifier.verify(&ctx).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].verdict, Verdict::Fail);
        assert_eq!(
            results[0].step_results[0].reason,
            "test selector `missing node test` matched zero tests; a filter that resolves to nothing is not coverage"
        );
    }

    #[test]
    fn cargo_zero_match_hint_suggests_route() {
        let root = temp_workspace_path("cargo-route-hint");
        write_file(&root.join("Cargo.toml"), "[workspace]\nmembers = []\n");
        write_file(
            &root.join("web/apps/admin/package.json"),
            "{\"scripts\":{}}\n",
        );
        let scenario = Scenario {
            name: "Cargo zero match with node-looking package".into(),
            steps: vec![Step {
                kind: StepKind::Then,
                text: "route hint is included".into(),
                params: vec![],
                table: vec![],
                span: Span::line(1),
            }],
            test_selector: Some(TestSelector {
                package: Some("admin".into()),
                filter: "missing_admin_test".into(),
                level: Some("unit".into()),
                test_double: None,
                targets: None,
            }),
            verification: Default::default(),
            evidence: None,
            tags: Vec::new(),
            review: Default::default(),
            mode: Default::default(),
            depends_on: vec![],
            span: Span::line(1),
        };
        let runner: Arc<dyn TestRunner> = Arc::new(CargoZeroMatchRouteHintRunner);
        let resolution = RunnerResolution {
            name: "cargo".into(),
            source: ResolutionSource::Detected,
            overridden_spec: None,
            config_warnings: Vec::new(),
        };
        let workspace = RunnerWorkspace::for_test(&root);
        let ctx = VerificationContext {
            code_paths: vec![root.clone()],
            change_paths: vec![],
            ai_mode: AiMode::Off,
            resolved_spec: resolved_spec_for_scenario(scenario),
            routed_contexts: RoutedContexts::default_only(RunnerSlot {
                runner: runner.clone(),
                runner_workspace: workspace.clone(),
                runner_resolution: resolution.clone(),
            }),
            runner,
            runner_workspace: workspace,
            runner_resolution: resolution,
            config_warnings: Vec::new(),
        };

        let results = TestVerifier.verify(&ctx).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].verdict, Verdict::Fail);
        let reason = &results[0].step_results[0].reason;
        assert!(
            reason.contains(
                "Package `admin` looks like a Node package at `web/apps/admin`; declare it under `runners:`"
            ),
            "expected route hint in reason, got: {reason}"
        );
    }

    fn routed_test_context(scenario: Scenario) -> VerificationContext {
        let default_runner: Arc<dyn TestRunner> = Arc::new(DefaultSkipRunner);
        let routed_runner: Arc<dyn TestRunner> = Arc::new(RoutedPassRunner);
        let default_workspace = RunnerWorkspace::for_test(".");
        let default_resolution = RunnerResolution {
            name: "cargo".into(),
            source: ResolutionSource::Detected,
            overridden_spec: None,
            config_warnings: Vec::new(),
        };
        let routed_resolution = RunnerResolution {
            name: "node".into(),
            source: ResolutionSource::SpecFrontmatter,
            overridden_spec: None,
            config_warnings: Vec::new(),
        };

        VerificationContext {
            code_paths: vec![".".into()],
            change_paths: vec![],
            ai_mode: AiMode::Off,
            resolved_spec: resolved_spec_for_scenario(scenario),
            routed_contexts: RoutedContexts::new(
                vec![
                    RunnerSlot {
                        runner: default_runner.clone(),
                        runner_workspace: default_workspace.clone(),
                        runner_resolution: default_resolution.clone(),
                    },
                    RunnerSlot {
                        runner: routed_runner,
                        runner_workspace: RunnerWorkspace::for_test("."),
                        runner_resolution: routed_resolution,
                    },
                ],
                BTreeMap::from([("admin".to_string(), 1)]),
            )
            .unwrap(),
            runner: default_runner,
            runner_workspace: default_workspace,
            runner_resolution: default_resolution,
            config_warnings: Vec::new(),
        }
    }

    fn resolved_spec_for_scenario(scenario: Scenario) -> ResolvedSpec {
        ResolvedSpec {
            task: SpecDocument {
                meta: SpecMeta {
                    level: SpecLevel::Task,
                    name: "routed selector".into(),
                    inherits: None,
                    lang: vec![],
                    tags: vec![],
                    runner: Some("cargo".into()),
                    runner_config: Default::default(),
                    runner_routes: Vec::new(),
                    depends: vec![],
                    estimate: None,
                },
                sections: vec![Section::AcceptanceCriteria {
                    scenarios: vec![scenario.clone()],
                    span: Span::line(1),
                }],
                parser_warnings: Vec::new(),
                source_path: Default::default(),
            },
            inherited_constraints: vec![],
            inherited_decisions: vec![],
            all_scenarios: vec![scenario],
        }
    }

    fn temp_workspace_path(label: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("specwright-{label}-{nanos}"));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    fn write_file(path: &std::path::Path, contents: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, contents).unwrap();
    }

    #[test]
    fn test_build_cargo_test_command_with_package_selector() {
        let runner = crate::spec_verify::CargoRunner;
        let selector = TestSelector {
            package: Some("spec-parser".into()),
            filter: "test_parse_structured_test_selector_block".into(),
            level: None,
            test_double: None,
            targets: None,
        };

        let command = crate::spec_verify::TestRunner::build_test_command(
            &runner,
            &crate::spec_verify::RunnerWorkspace::for_test("."),
            &selector,
        )
        .unwrap();
        assert_eq!(command.program, "cargo");
        assert_eq!(
            command.args,
            vec![
                "test".to_string(),
                "-q".to_string(),
                "-p".to_string(),
                "spec-parser".to_string(),
                "test_parse_structured_test_selector_block".to_string(),
            ]
        );
    }
}
