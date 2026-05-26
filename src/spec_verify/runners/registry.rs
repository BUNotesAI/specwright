use std::sync::Arc;

use crate::spec_core::{ResolvedSpec, SpecError, SpecResult};

use super::{
    CargoRunner, GradleRunner, MavenRunner, ResolutionSource, RunnerResolution, RunnerSelection,
    TestRunner, WorkspaceMarkers,
};

/// Registry of available test runners.
#[derive(Clone)]
pub struct RunnerRegistry {
    runners: Vec<Arc<dyn TestRunner>>,
}

impl RunnerRegistry {
    pub fn new() -> Self {
        Self {
            runners: Vec::new(),
        }
    }

    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register(Arc::new(CargoRunner));
        registry.register(Arc::new(MavenRunner));
        registry.register(Arc::new(GradleRunner));
        registry
    }

    pub fn register(&mut self, runner: Arc<dyn TestRunner>) {
        self.runners.push(runner);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn TestRunner>> {
        self.runners
            .iter()
            .find(|runner| runner.id() == name)
            .cloned()
    }

    pub fn detect(&self, markers: &WorkspaceMarkers) -> SpecResult<Option<Arc<dyn TestRunner>>> {
        if let Some(runner) = self.detect_maven_gradle_tie_break(markers)? {
            return Ok(Some(runner));
        }

        Ok(self
            .runners
            .iter()
            .find(|runner| runner.detect(markers))
            .cloned())
    }

    fn detect_maven_gradle_tie_break(
        &self,
        markers: &WorkspaceMarkers,
    ) -> SpecResult<Option<Arc<dyn TestRunner>>> {
        let has_maven_manifest = markers.contains("pom.xml");
        let has_gradle_manifest =
            markers.contains("build.gradle") || markers.contains("build.gradle.kts");
        if !(has_maven_manifest && has_gradle_manifest) {
            return Ok(None);
        }

        let has_maven_wrapper = markers.contains("mvnw") || markers.contains("mvnw.cmd");
        let has_gradle_wrapper = markers.contains("gradlew") || markers.contains("gradlew.bat");
        match (has_maven_wrapper, has_gradle_wrapper) {
            (true, false) => self.runner_or_registration_error("maven").map(Some),
            (false, true) => self.runner_or_registration_error("gradle").map(Some),
            _ => Err(SpecError::Verification(
                "ambiguous Maven/Gradle workspace: found both `pom.xml` and `build.gradle*` without exactly one wrapper family; set `runner: maven` or `runner: gradle`, or pass `--runner`"
                    .into(),
            )),
        }
    }

    fn runner_or_registration_error(&self, name: &str) -> SpecResult<Arc<dyn TestRunner>> {
        self.get(name).ok_or_else(|| {
            SpecError::Verification(format!(
                "detected `{name}` runner marker but `{name}` runner is not registered"
            ))
        })
    }
}

impl Default for RunnerRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

pub fn resolve_runner_choice(
    registry: &RunnerRegistry,
    resolved_spec: &ResolvedSpec,
    cli_runner: Option<&str>,
) -> SpecResult<RunnerSelection> {
    let spec_runner = resolved_spec.task.meta.runner.as_deref();
    let selection = match (cli_runner, spec_runner) {
        (None, None) => RunnerSelection::NeedsDetect,
        (Some(cli), None) => RunnerSelection::ByName {
            name: cli.to_string(),
            source: ResolutionSource::CliFlag,
            overridden_spec: None,
        },
        (None, Some(spec)) => RunnerSelection::ByName {
            name: spec.to_string(),
            source: ResolutionSource::SpecFrontmatter,
            overridden_spec: None,
        },
        (Some(cli), Some(spec)) => RunnerSelection::ByName {
            name: cli.to_string(),
            source: ResolutionSource::CliFlag,
            overridden_spec: if cli == spec {
                None
            } else {
                Some(spec.to_string())
            },
        },
    };

    if let RunnerSelection::ByName { name, .. } = &selection
        && registry.get(name).is_none()
    {
        return Err(SpecError::Verification(format!(
            "unknown test runner `{name}`"
        )));
    }

    Ok(selection)
}

pub fn resolve_detected_runner(
    registry: &RunnerRegistry,
    selection: RunnerSelection,
    markers: &WorkspaceMarkers,
) -> SpecResult<(Arc<dyn TestRunner>, RunnerResolution)> {
    match selection {
        RunnerSelection::NeedsDetect => {
            let Some(runner) = registry.detect(markers)? else {
                return Err(SpecError::Verification(
                    "no test runner detected for workspace".into(),
                ));
            };
            let resolution = RunnerResolution {
                name: runner.id().to_string(),
                source: ResolutionSource::Detected,
                overridden_spec: None,
                config_warnings: Vec::new(),
            };
            Ok((runner, resolution))
        }
        RunnerSelection::ByName {
            name,
            source,
            overridden_spec,
        } => {
            let Some(runner) = registry.get(&name) else {
                return Err(SpecError::Verification(format!(
                    "unknown test runner `{name}`"
                )));
            };
            let resolution = RunnerResolution {
                name,
                source,
                overridden_spec,
                config_warnings: Vec::new(),
            };
            Ok((runner, resolution))
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use crate::spec_core::{
        ResolvedSpec, Section, Span, SpecDocument, SpecLevel, SpecMeta, TestSelector,
    };

    use super::super::{
        PreflightOutcome, ResolutionSource, RunnerSelection, RunnerSourceFile, RunnerWorkspace,
        TestCommand, TestRunner, WorkspaceMarkers,
    };
    use super::{RunnerRegistry, resolve_detected_runner, resolve_runner_choice};

    struct FakeRunner;

    impl TestRunner for FakeRunner {
        fn id(&self) -> &'static str {
            "fake_runner"
        }

        fn detect(&self, markers: &WorkspaceMarkers) -> bool {
            markers.contains("fake.marker")
        }

        fn build_test_command(
            &self,
            _workspace: &RunnerWorkspace,
            _selector: &TestSelector,
        ) -> crate::spec_core::SpecResult<TestCommand> {
            Ok(TestCommand {
                program: "cargo".into(),
                args: vec!["test".into(), "-q".into(), "test_parse_basic_meta".into()],
            })
        }

        fn scan_legacy_bindings(
            &self,
            _workspace: &RunnerWorkspace,
        ) -> crate::spec_core::SpecResult<std::collections::HashMap<String, String>> {
            Ok(Default::default())
        }

        fn preflight(
            &self,
            _workspace: &RunnerWorkspace,
            _selector: &TestSelector,
        ) -> crate::spec_core::SpecResult<PreflightOutcome> {
            Ok(PreflightOutcome::Ready)
        }
    }

    #[test]
    fn test_maven_runner_uses_mvnw_via_wrapper_family() {
        let registry = RunnerRegistry::with_defaults();
        let runner = registry.get("maven").unwrap();
        let workspace = RunnerWorkspace::new(
            Some(PathBuf::from(".")),
            Vec::new(),
            Default::default(),
            WorkspaceMarkers::from_files(["pom.xml", "mvnw"]),
            Vec::new(),
        );

        let command = runner
            .build_test_command(
                &workspace,
                &TestSelector::filter_only("PaymentRulesTest#approvesValidCard"),
            )
            .unwrap();

        assert_eq!(command.program, "./mvnw");
        assert_eq!(
            command.args,
            vec![
                "test".to_string(),
                "-Dtest=PaymentRulesTest#approvesValidCard".to_string()
            ]
        );
    }

    #[test]
    fn test_jvm_scanner_method_level_emits_method_selector() {
        let registry = RunnerRegistry::with_defaults();
        let runner = registry.get("maven").unwrap();
        let workspace = RunnerWorkspace::new(
            Some(PathBuf::from(".")),
            Vec::new(),
            Default::default(),
            WorkspaceMarkers::from_files(["pom.xml"]),
            vec![
                RunnerSourceFile {
                    path: PathBuf::from("src/test/java/com/example/PaymentRulesTest.java"),
                    content: r#"
package com.example;

class PaymentRulesTest {
    @Spec("rejects expired card")
    @Test
    void rejectsExpiredCard() {}
}
"#
                    .to_string(),
                },
                RunnerSourceFile {
                    path: PathBuf::from("src/test/kotlin/com/example/RiskRulesTest.kt"),
                    content: r#"
package com.example

class RiskRulesTest {
    @Spec("rejects risky payment")
    @Test
    fun rejectsRiskyPayment() {}
}
"#
                    .to_string(),
                },
            ],
        );

        let bindings = runner.scan_legacy_bindings(&workspace).unwrap();

        assert_eq!(
            bindings.get("rejects expired card"),
            Some(&"PaymentRulesTest#rejectsExpiredCard".to_string())
        );
        assert_eq!(
            bindings.get("rejects risky payment"),
            Some(&"RiskRulesTest#rejectsRiskyPayment".to_string())
        );
    }

    #[test]
    fn test_jvm_scanner_class_level_emits_single_class_selector() {
        let registry = RunnerRegistry::with_defaults();
        let runner = registry.get("maven").unwrap();
        let workspace = RunnerWorkspace::new(
            Some(PathBuf::from(".")),
            Vec::new(),
            Default::default(),
            WorkspaceMarkers::from_files(["pom.xml"]),
            vec![RunnerSourceFile {
                path: PathBuf::from("src/test/java/com/example/PaymentRiskRulesTest.java"),
                content: r#"
package com.example;

@Spec("payment risk rules")
class PaymentRiskRulesTest {
    @Test
    void acceptsLowRisk() {}

    @Test
    void rejectsHighRisk() {}

    @Test
    void logsReviewQueue() {}
}
"#
                .to_string(),
            }],
        );

        let bindings = runner.scan_legacy_bindings(&workspace).unwrap();

        assert_eq!(bindings.len(), 1);
        assert_eq!(
            bindings.get("payment risk rules"),
            Some(&"PaymentRiskRulesTest".to_string())
        );
    }

    #[test]
    fn test_wrapper_family_both_present_fails_loudly() {
        let registry = RunnerRegistry::with_defaults();
        let markers = WorkspaceMarkers::from_files(["pom.xml", "build.gradle", "mvnw", "gradlew"]);
        let result = resolve_detected_runner(&registry, RunnerSelection::NeedsDetect, &markers)
            .map(|(runner, _)| runner.id().to_string());

        let Err(err) = result else {
            panic!("dual Maven/Gradle wrapper families should fail loud");
        };
        let message = err.to_string();

        assert!(message.contains("Maven"));
        assert!(message.contains("Gradle"));
        assert!(message.contains("runner:"));
        assert!(message.contains("--runner"));
    }

    #[test]
    fn test_resolve_runner_choice_precedence_matrix() {
        let mut registry = RunnerRegistry::with_defaults();
        registry.register(Arc::new(FakeRunner));

        let no_runner = resolved_spec(None);
        assert_eq!(
            resolve_runner_choice(&registry, &no_runner, None).unwrap(),
            RunnerSelection::NeedsDetect
        );

        assert_eq!(
            resolve_runner_choice(&registry, &no_runner, Some("cargo")).unwrap(),
            RunnerSelection::ByName {
                name: "cargo".into(),
                source: ResolutionSource::CliFlag,
                overridden_spec: None,
            }
        );

        let spec_cargo = resolved_spec(Some("cargo"));
        assert_eq!(
            resolve_runner_choice(&registry, &spec_cargo, None).unwrap(),
            RunnerSelection::ByName {
                name: "cargo".into(),
                source: ResolutionSource::SpecFrontmatter,
                overridden_spec: None,
            }
        );
        assert_eq!(
            resolve_runner_choice(&registry, &spec_cargo, Some("cargo")).unwrap(),
            RunnerSelection::ByName {
                name: "cargo".into(),
                source: ResolutionSource::CliFlag,
                overridden_spec: None,
            }
        );
        assert_eq!(
            resolve_runner_choice(&registry, &spec_cargo, Some("fake_runner")).unwrap(),
            RunnerSelection::ByName {
                name: "fake_runner".into(),
                source: ResolutionSource::CliFlag,
                overridden_spec: Some("cargo".into()),
            }
        );
    }

    #[test]
    fn test_runner_registry_register_custom_runner_end_to_end() {
        let mut registry = RunnerRegistry::new();
        registry.register(Arc::new(FakeRunner));

        let selection =
            resolve_runner_choice(&registry, &resolved_spec(Some("fake_runner")), None).unwrap();
        let (runner, resolution) = resolve_detected_runner(
            &registry,
            selection,
            &WorkspaceMarkers::from_files(["fake.marker"]),
        )
        .unwrap();

        assert_eq!(runner.id(), "fake_runner");
        assert_eq!(resolution.name, "fake_runner");
        assert_eq!(resolution.source, ResolutionSource::SpecFrontmatter);
    }

    fn resolved_spec(runner: Option<&str>) -> ResolvedSpec {
        ResolvedSpec {
            task: SpecDocument {
                meta: SpecMeta {
                    level: SpecLevel::Task,
                    name: "runner spec".into(),
                    inherits: None,
                    lang: vec![],
                    tags: vec![],
                    runner: runner.map(str::to_string),
                    runner_config: Default::default(),
                    depends: vec![],
                    estimate: None,
                },
                sections: vec![Section::AcceptanceCriteria {
                    scenarios: vec![],
                    span: Span::line(1),
                }],
                source_path: PathBuf::new(),
            },
            inherited_constraints: vec![],
            inherited_decisions: vec![],
            all_scenarios: vec![],
        }
    }
}
