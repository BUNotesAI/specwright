use std::sync::Arc;

use crate::spec_core::{ResolvedSpec, SpecError, SpecResult};

use super::{
    AndroidRunner, CTestRunner, CargoRunner, GradleRunner, IosRunner, MavenRunner, NodeRunner,
    ResolutionSource, RunnerResolution, RunnerRoutingPlan, RunnerSelection, RunnerWarning,
    TestRunner, ValidatedRoute, WorkspaceMarkers,
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
        registry.register(Arc::new(AndroidRunner::new()));
        registry.register(Arc::new(IosRunner::new()));
        registry.register(Arc::new(GradleRunner));
        registry.register(Arc::new(NodeRunner));
        registry.register(Arc::new(CTestRunner));
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

    fn runner_ids(&self) -> Vec<&'static str> {
        self.runners.iter().map(|runner| runner.id()).collect()
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
        return Err(unknown_runner_error(name));
    }

    Ok(selection)
}

/// Resolve the default runner plus declared package routes without probing the filesystem.
pub fn resolve_runner_routing(
    registry: &RunnerRegistry,
    resolved_spec: &ResolvedSpec,
    cli_runner: Option<&str>,
) -> SpecResult<RunnerRoutingPlan> {
    let default_runner = resolve_runner_choice(registry, resolved_spec, cli_runner)?;
    let declared_routes = &resolved_spec.task.meta.runner_routes;

    if cli_runner.is_some() {
        let config_warnings = if declared_routes.is_empty() {
            Vec::new()
        } else {
            vec![RunnerWarning {
                runner: selection_runner_label(&default_runner).to_string(),
                key: "runners".to_string(),
                reason: format!(
                    "--runner overrides {} declared {}; routes ignored",
                    declared_routes.len(),
                    pluralize("route", declared_routes.len())
                ),
            }]
        };
        return Ok(RunnerRoutingPlan {
            default_runner,
            routes: Vec::new(),
            config_warnings,
        });
    }

    let mut seen_packages: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();
    let mut routes = Vec::new();
    for route in declared_routes {
        if registry.get(&route.runner).is_none() {
            return Err(unknown_route_runner_error(&route.runner, registry));
        }

        for package in route.packages.keys() {
            if let Some(previous_runner) =
                seen_packages.insert(package.clone(), route.runner.clone())
            {
                return Err(SpecError::Verification(format!(
                    "duplicate package route token `{package}` declared for both `{previous_runner}` and `{}`",
                    route.runner
                )));
            }
        }

        routes.push(ValidatedRoute {
            runner: route.runner.clone(),
            root: route.root.clone(),
            packages: route.packages.clone(),
            config: route.config.clone(),
        });
    }

    Ok(RunnerRoutingPlan {
        default_runner,
        routes,
        config_warnings: Vec::new(),
    })
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
                return Err(unknown_runner_error(&name));
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

fn unknown_runner_error(name: &str) -> SpecError {
    if matches!(name, "vitest" | "jest" | "tsc" | "playwright") {
        SpecError::Verification(format!(
            "unknown test runner `{name}`; did you mean runner: node"
        ))
    } else {
        SpecError::Verification(format!("unknown test runner `{name}`"))
    }
}

fn unknown_route_runner_error(name: &str, registry: &RunnerRegistry) -> SpecError {
    let valid = registry.runner_ids().join(", ");
    if matches!(name, "vitest" | "jest" | "tsc" | "playwright") {
        SpecError::Verification(format!(
            "unknown test runner `{name}`; did you mean runner: node; valid runners: {valid}"
        ))
    } else {
        SpecError::Verification(format!(
            "unknown test runner `{name}`; valid runners: {valid}"
        ))
    }
}

fn selection_runner_label(selection: &RunnerSelection) -> &str {
    match selection {
        RunnerSelection::NeedsDetect => "detect",
        RunnerSelection::ByName { name, .. } => name,
    }
}

fn pluralize(noun: &str, count: usize) -> String {
    if count == 1 {
        noun.to_string()
    } else {
        format!("{noun}s")
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use std::sync::Arc;

    use crate::spec_core::{
        ResolvedSpec, RunnerRouteDecl, Section, Span, SpecDocument, SpecLevel, SpecMeta,
        TestSelector,
    };

    use super::super::{
        HostPlatform, PreflightOutcome, ResolutionSource, RunnerSelection, RunnerSourceFile,
        RunnerWorkspace, TestCommand, TestRunner, WorkspaceMarkers,
    };
    use super::{
        RunnerRegistry, resolve_detected_runner, resolve_runner_choice, resolve_runner_routing,
    };

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
                cwd: None,
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
        let workspace = RunnerWorkspace::new_without_metadata(
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
        let workspace = RunnerWorkspace::new_without_metadata(
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
        let workspace = RunnerWorkspace::new_without_metadata(
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
    fn test_android_build_test_command_unit_dispatch() {
        let registry = RunnerRegistry::with_defaults();
        let runner = registry.get("android").unwrap();
        let workspace = android_workspace();

        for level in [None, Some("unit".to_string())] {
            let command = runner
                .build_test_command(
                    &workspace,
                    &TestSelector {
                        package: Some(":app".to_string()),
                        filter: "com.example.PaymentRulesTest#approvesValidCard".to_string(),
                        level,
                        test_double: None,
                        targets: None,
                    },
                )
                .unwrap();

            assert_eq!(command.program, "./gradlew");
            assert_eq!(
                command.args,
                vec![
                    ":app:testDebugUnitTest".to_string(),
                    "--tests".to_string(),
                    "com.example.PaymentRulesTest.approvesValidCard".to_string()
                ]
            );
        }
    }

    #[test]
    fn test_android_build_test_command_instrumented_dispatch() {
        let registry = RunnerRegistry::with_defaults();
        let runner = registry.get("android").unwrap();
        let workspace = android_workspace();

        let command = runner
            .build_test_command(
                &workspace,
                &TestSelector {
                    package: Some(":app".to_string()),
                    filter: "com.example.PaymentRulesTest#approvesValidCard".to_string(),
                    level: Some("instrumented".to_string()),
                    test_double: None,
                    targets: None,
                },
            )
            .unwrap();

        assert_eq!(command.program, "./gradlew");
        assert_eq!(
            command.args,
            vec![
                ":app:connectedAndroidTest".to_string(),
                "-Pandroid.testInstrumentationRunnerArguments.class=com.example.PaymentRulesTest#approvesValidCard".to_string()
            ]
        );
    }

    #[test]
    fn test_android_build_test_command_unknown_level_errors() {
        let registry = RunnerRegistry::with_defaults();
        let runner = registry.get("android").unwrap();
        let workspace = android_workspace();

        let err = runner
            .build_test_command(
                &workspace,
                &TestSelector {
                    package: Some(":app".to_string()),
                    filter: "com.example.PaymentRulesTest#approvesValidCard".to_string(),
                    level: Some("integration".to_string()),
                    test_double: None,
                    targets: None,
                },
            )
            .unwrap_err();
        let message = err.to_string();

        assert!(message.contains("integration"));
        assert!(message.contains("unit"));
        assert!(message.contains("instrumented"));
    }

    #[test]
    fn test_android_requires_device_flips_with_level() {
        let registry = RunnerRegistry::with_defaults();
        let runner = registry.get("android").unwrap();

        let unit = TestSelector {
            package: Some(":app".to_string()),
            filter: "com.example.PaymentRulesTest#approvesValidCard".to_string(),
            level: None,
            test_double: None,
            targets: None,
        };
        let instrumented = TestSelector {
            level: Some("instrumented".to_string()),
            ..unit.clone()
        };

        assert!(!runner.requires_device(&unit));
        assert!(runner.requires_device(&instrumented));
    }

    #[test]
    fn test_android_recognized_config_keys_snapshot() {
        let registry = RunnerRegistry::with_defaults();
        let runner = registry.get("android").unwrap();

        assert_eq!(
            runner.recognized_config_keys(),
            &["gradle_args", "instrumentation_runner"]
        );
    }

    #[test]
    fn test_android_detect_precedence_over_gradle() {
        let registry = RunnerRegistry::with_defaults();
        let markers = WorkspaceMarkers::from_files([
            "AndroidManifest.xml",
            "build.gradle.kts",
            "settings.gradle.kts",
            "gradlew",
        ]);

        let (runner, resolution) =
            resolve_detected_runner(&registry, RunnerSelection::NeedsDetect, &markers).unwrap();

        assert_eq!(runner.id(), "android");
        assert_eq!(resolution.name, "android");
    }

    #[test]
    fn test_ios_build_test_command_with_destination_and_scheme() {
        let registry = RunnerRegistry::with_defaults();
        let Some(runner) = registry.get("ios") else {
            panic!("ios runner should be registered");
        };
        let workspace = ios_workspace(BTreeMap::from([
            (
                "destination".to_string(),
                "platform=iOS Simulator,name=iPhone 15".to_string(),
            ),
            ("scheme".to_string(), "IosMini".to_string()),
        ]));

        let command = runner
            .build_test_command(
                &workspace,
                &TestSelector {
                    package: Some("IosMiniTests".to_string()),
                    filter: "PaymentTests/testRejectsExpiredCard".to_string(),
                    level: None,
                    test_double: None,
                    targets: None,
                },
            )
            .unwrap();

        assert_eq!(command.program, "xcodebuild");
        assert_eq!(
            command.args,
            vec![
                "test".to_string(),
                "-scheme".to_string(),
                "IosMini".to_string(),
                "-destination".to_string(),
                "platform=iOS Simulator,name=iPhone 15".to_string(),
                "-only-testing:IosMiniTests/PaymentTests/testRejectsExpiredCard".to_string(),
            ]
        );
    }

    #[test]
    fn test_ios_build_test_command_without_destination_and_scheme() {
        let registry = RunnerRegistry::with_defaults();
        let Some(runner) = registry.get("ios") else {
            panic!("ios runner should be registered");
        };
        let workspace = ios_workspace(BTreeMap::new());

        let command = runner
            .build_test_command(
                &workspace,
                &TestSelector {
                    package: Some("IosMiniTests".to_string()),
                    filter: "PaymentTests/testRejectsExpiredCard".to_string(),
                    level: None,
                    test_double: None,
                    targets: None,
                },
            )
            .unwrap();

        assert_eq!(command.program, "xcodebuild");
        assert_eq!(
            command.args,
            vec![
                "test".to_string(),
                "-only-testing:IosMiniTests/PaymentTests/testRejectsExpiredCard".to_string(),
            ]
        );
    }

    #[test]
    fn test_ios_supported_host_platforms_snapshot() {
        let registry = RunnerRegistry::with_defaults();
        let Some(runner) = registry.get("ios") else {
            panic!("ios runner should be registered");
        };

        assert_eq!(runner.supported_host_platforms(), &[HostPlatform::MacOS]);
    }

    #[test]
    fn test_ios_detects_swift_package_marker() {
        let registry = RunnerRegistry::with_defaults();
        let markers = WorkspaceMarkers::from_files(["Package.swift"]);

        let (runner, resolution) =
            resolve_detected_runner(&registry, RunnerSelection::NeedsDetect, &markers).unwrap();

        assert_eq!(runner.id(), "ios");
        assert_eq!(resolution.name, "ios");
    }

    #[test]
    fn test_ios_detects_xcodeproj_marker() {
        let registry = RunnerRegistry::with_defaults();
        let markers = WorkspaceMarkers::from_files(["*.xcodeproj"]);

        let (runner, resolution) =
            resolve_detected_runner(&registry, RunnerSelection::NeedsDetect, &markers).unwrap();

        assert_eq!(runner.id(), "ios");
        assert_eq!(resolution.name, "ios");
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
    fn resolve_rejects_duplicate_package_token() {
        let registry = RunnerRegistry::with_defaults();
        let spec = resolved_spec_with_routes(
            Some("cargo"),
            vec![
                route("node", &[("admin", "apps/admin")]),
                route("cargo", &[("admin", "admin-copy")]),
            ],
        );

        let err = resolve_runner_routing(&registry, &spec, None)
            .unwrap_err()
            .to_string();

        assert!(err.contains("duplicate package route token `admin`"));
        assert!(err.contains("node"));
        assert!(err.contains("cargo"));
    }

    #[test]
    fn resolve_rejects_unknown_route_runner_id() {
        let registry = RunnerRegistry::with_defaults();
        let spec = resolved_spec_with_routes(
            Some("cargo"),
            vec![route("not_registered", &[("admin", "apps/admin")])],
        );

        let err = resolve_runner_routing(&registry, &spec, None)
            .unwrap_err()
            .to_string();

        assert!(err.contains("unknown test runner `not_registered`"));
        assert!(err.contains("valid runners"));
        assert!(err.contains("cargo"));
        assert!(err.contains("node"));
    }

    #[test]
    fn cli_override_ignores_routes_with_warning() {
        let registry = RunnerRegistry::with_defaults();
        let spec = resolved_spec_with_routes(
            Some("cargo"),
            vec![route("node", &[("admin", "apps/admin"), ("web", ".")])],
        );

        let plan = resolve_runner_routing(&registry, &spec, Some("cargo")).unwrap();

        assert_eq!(plan.routes, Vec::new());
        assert_eq!(
            plan.default_runner,
            RunnerSelection::ByName {
                name: "cargo".into(),
                source: ResolutionSource::CliFlag,
                overridden_spec: None,
            }
        );
        assert_eq!(plan.config_warnings.len(), 1);
        assert_eq!(plan.config_warnings[0].runner, "cargo");
        assert_eq!(plan.config_warnings[0].key, "runners");
        assert!(
            plan.config_warnings[0]
                .reason
                .contains("overrides 1 declared route")
        );
    }

    #[test]
    fn empty_routes_match_single_runner_plan() {
        let registry = RunnerRegistry::with_defaults();
        let spec = resolved_spec(Some("cargo"));

        let plan = resolve_runner_routing(&registry, &spec, None).unwrap();

        assert!(plan.routes.is_empty());
        assert!(plan.config_warnings.is_empty());
        assert_eq!(
            plan.default_runner,
            resolve_runner_choice(&registry, &spec, None).unwrap()
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

    #[test]
    fn test_node_runner_registered_and_framework_ids_hint_node() {
        let registry = RunnerRegistry::with_defaults();

        assert!(registry.get("node").is_some());
        for framework in ["vitest", "jest", "tsc", "playwright"] {
            let err = resolve_runner_choice(&registry, &resolved_spec(Some(framework)), None)
                .unwrap_err()
                .to_string();
            assert!(err.contains(framework));
            assert!(err.contains("did you mean runner: node"));
        }
    }

    fn android_workspace() -> RunnerWorkspace {
        RunnerWorkspace::new_without_metadata(
            Some(PathBuf::from(".")),
            Vec::new(),
            Default::default(),
            WorkspaceMarkers::from_files(["AndroidManifest.xml", "build.gradle.kts", "gradlew"]),
            Vec::new(),
        )
    }

    fn ios_workspace(config: BTreeMap<String, String>) -> RunnerWorkspace {
        RunnerWorkspace::new_without_metadata(
            Some(PathBuf::from(".")),
            Vec::new(),
            config,
            WorkspaceMarkers::from_files(["Package.swift"]),
            Vec::new(),
        )
    }

    fn route(runner: &str, packages: &[(&str, &str)]) -> RunnerRouteDecl {
        RunnerRouteDecl {
            runner: runner.to_string(),
            root: Some("web".to_string()),
            packages: packages
                .iter()
                .map(|(token, path)| ((*token).to_string(), (*path).to_string()))
                .collect(),
            config: BTreeMap::new(),
        }
    }

    fn resolved_spec_with_routes(
        runner: Option<&str>,
        routes: Vec<RunnerRouteDecl>,
    ) -> ResolvedSpec {
        let mut spec = resolved_spec(runner);
        spec.task.meta.runner_routes = routes;
        spec
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
                    runner_routes: Vec::new(),
                    depends: vec![],
                    estimate: None,
                },
                sections: vec![Section::AcceptanceCriteria {
                    scenarios: vec![],
                    span: Span::line(1),
                }],
                parser_warnings: Vec::new(),
                source_path: PathBuf::new(),
            },
            inherited_constraints: vec![],
            inherited_decisions: vec![],
            all_scenarios: vec![],
        }
    }
}
