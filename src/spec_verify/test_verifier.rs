use std::collections::HashMap;
use std::process::Command;
use std::time::Instant;

use crate::spec_core::{
    Evidence, ReviewMode, Scenario, ScenarioResult, SpecError, SpecResult, StepVerdict,
    TestSelector, Verdict,
};

use super::{VerificationContext, Verifier};

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
}

impl Verifier for TestVerifier {
    fn name(&self) -> &str {
        "test"
    }

    #[allow(clippy::too_many_lines)] // Exception: legacy Cargo verifier path; runner refactor will split this under task_3455b7d6.
    fn verify(&self, ctx: &VerificationContext) -> SpecResult<Vec<ScenarioResult>> {
        let Some(workspace_root) = ctx.runner_workspace.root.as_ref() else {
            return Ok(Vec::new());
        };

        let legacy_bindings = ctx.runner.scan_legacy_bindings(&ctx.runner_workspace)?;
        let mut results = Vec::new();

        for scenario in &ctx.resolved_spec.all_scenarios {
            let Some(binding) = resolve_test_binding(scenario, &legacy_bindings) else {
                continue;
            };

            if let super::PreflightOutcome::MissingCapability { capability, reason } = ctx
                .runner
                .preflight(&ctx.runner_workspace, &binding.selector)?
            {
                results.push(skip_for_missing_capability(scenario, &capability, &reason));
                continue;
            }

            let started = Instant::now();
            let command = ctx
                .runner
                .build_test_command(&ctx.runner_workspace, &binding.selector)?;
            let output = Command::new(&command.program)
                .args(&command.args)
                .current_dir(workspace_root)
                .output()
                .map_err(|err| {
                    SpecError::Verification(format!(
                        "failed to run {} test command: {err}",
                        ctx.runner.id()
                    ))
                })?;
            let duration_ms = started.elapsed().as_millis() as u64;

            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            let combined = if stderr.trim().is_empty() {
                stdout.clone()
            } else if stdout.trim().is_empty() {
                stderr.clone()
            } else {
                format!("{stdout}\n{stderr}")
            };

            let verdict = if output.status.success() {
                if scenario.review == ReviewMode::Human {
                    Verdict::PendingReview
                } else {
                    Verdict::Pass
                }
            } else {
                Verdict::Fail
            };
            let selector_label = binding.selector.label();
            let reason = if output.status.success() {
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
            };

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
                    command_program: command_program_evidence(ctx.runner.id(), &command.program),
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

fn resolve_test_binding(
    scenario: &Scenario,
    legacy_bindings: &HashMap<String, String>,
) -> Option<TestBinding> {
    if let Some(selector) = scenario.test_selector.as_ref() {
        return Some(TestBinding {
            selector: selector.clone(),
            source: BindingSource::ExplicitScenarioSelector,
        });
    }

    legacy_bindings
        .get(&scenario.name)
        .map(|selector| TestBinding {
            selector: TestSelector::filter_only(selector.clone()),
            source: BindingSource::LegacyComment,
        })
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
    use std::collections::HashMap;
    use std::sync::Arc;

    use crate::spec_core::{
        ResolvedSpec, Scenario, Section, Span, SpecDocument, SpecLevel, SpecMeta, Step, StepKind,
        TestSelector, Verdict,
    };
    use crate::spec_verify::{
        AiMode, PreflightOutcome, ResolutionSource, RunnerResolution, RunnerWorkspace, TestCommand,
        TestRunner, VerificationContext, Verifier, WorkspaceMarkers,
    };

    use super::{BindingSource, TestVerifier, resolve_test_binding};

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
            tags: Vec::new(),
            review: Default::default(),
            mode: Default::default(),
            depends_on: vec![],
            span: Span::default(),
        };
        let legacy = HashMap::from([("场景一".to_string(), "legacy_test_name".to_string())]);

        let binding = resolve_test_binding(&scenario, &legacy).unwrap();
        assert_eq!(
            binding.selector,
            TestSelector::filter_only(
                "test_explicit_scenario_selector_takes_precedence_over_legacy_comment_binding"
            )
        );
        assert_eq!(binding.source, BindingSource::ExplicitScenarioSelector);
    }

    #[test]
    fn test_legacy_comment_binding_is_used_when_no_explicit_selector_exists() {
        let scenario = Scenario {
            name: "场景一".into(),
            steps: Vec::new(),
            test_selector: None,
            tags: Vec::new(),
            review: Default::default(),
            mode: Default::default(),
            depends_on: vec![],
            span: Span::default(),
        };
        let legacy = HashMap::from([(
            "场景一".to_string(),
            "test_legacy_comment_binding_is_used_when_no_explicit_selector_exists".to_string(),
        )]);

        let binding = resolve_test_binding(&scenario, &legacy).unwrap();
        assert_eq!(
            binding.selector,
            TestSelector::filter_only(
                "test_legacy_comment_binding_is_used_when_no_explicit_selector_exists"
            )
        );
        assert_eq!(binding.source, BindingSource::LegacyComment);
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

    struct MissingInstrumentedRunner;

    impl TestRunner for MissingInstrumentedRunner {
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
                    capability: "android-instrumented-on-windows".into(),
                    reason: "Android instrumented tests are not supported on Windows hosts".into(),
                });
            }
            Ok(PreflightOutcome::Ready)
        }
    }

    fn instrumented_preflight_context() -> VerificationContext {
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
            tags: Vec::new(),
            review: Default::default(),
            mode: Default::default(),
            depends_on: vec![],
            span: Span::line(1),
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
                        depends: vec![],
                        estimate: None,
                    },
                    sections: vec![Section::AcceptanceCriteria {
                        scenarios: vec![scenario.clone()],
                        span: Span::line(1),
                    }],
                    source_path: Default::default(),
                },
                inherited_constraints: vec![],
                inherited_decisions: vec![],
                all_scenarios: vec![scenario],
            },
            runner: Arc::new(MissingInstrumentedRunner),
            runner_workspace: RunnerWorkspace::for_test("."),
            runner_resolution: RunnerResolution {
                name: "android".into(),
                source: ResolutionSource::SpecFrontmatter,
                overridden_spec: None,
                config_warnings: Vec::new(),
            },
            config_warnings: Vec::new(),
        }
    }

    #[test]
    fn test_preflight_uses_bound_selector_and_skips_without_spawn() {
        let ctx = instrumented_preflight_context();

        let results = TestVerifier.verify(&ctx).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].verdict, Verdict::Skip);
        assert_eq!(
            results[0].step_results[0].reason,
            "android-instrumented-on-windows: Android instrumented tests are not supported on Windows hosts"
        );
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
