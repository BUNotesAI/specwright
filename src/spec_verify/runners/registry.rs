use std::sync::Arc;

use crate::spec_core::{ResolvedSpec, SpecError, SpecResult};

use super::{
    CargoRunner, ResolutionSource, RunnerResolution, RunnerSelection, TestRunner, WorkspaceMarkers,
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

    pub fn detect(&self, markers: &WorkspaceMarkers) -> Option<Arc<dyn TestRunner>> {
        self.runners
            .iter()
            .find(|runner| runner.detect(markers))
            .cloned()
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
            let Some(runner) = registry.detect(markers) else {
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
        PreflightOutcome, ResolutionSource, RunnerSelection, RunnerWorkspace, TestCommand,
        TestRunner, WorkspaceMarkers,
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
