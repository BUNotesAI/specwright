mod ai_verifier;
mod boundaries;
mod complexity;
mod runners;
mod structural;
mod test_verifier;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::spec_core::{
    ResolvedSpec, ScenarioResult, SpecError, SpecResult, StepVerdict, Verdict, VerificationReport,
};

pub use ai_verifier::{AiBackend, AiVerifier, build_ai_request};
pub use boundaries::BoundariesVerifier;
pub use complexity::ComplexityVerifier;
#[cfg(test)]
pub use runners::{CargoRunner, ResolutionSource, extract_bindings};
#[allow(unused_imports)]
pub use runners::{HostPlatform, RunnerSelection, RunnerWarning, TestCommand};
pub use runners::{
    PreflightOutcome, RunnerRegistry, RunnerResolution, RunnerSourceFile, RunnerWorkspace,
    TestRunner, WorkspaceMarkers, resolve_detected_runner, resolve_runner_choice,
};
pub use structural::StructuralVerifier;
pub use test_verifier::TestVerifier;

/// AI verifier mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiMode {
    Off,
    Stub,
    External,
    /// Caller mode: emit AiRequests for the calling agent to resolve externally.
    Caller,
}

/// Context for verification.
pub struct VerificationContext {
    pub code_paths: Vec<PathBuf>,
    pub change_paths: Vec<PathBuf>,
    pub ai_mode: AiMode,
    pub resolved_spec: ResolvedSpec,
    pub runner: Arc<dyn TestRunner>,
    pub runner_workspace: RunnerWorkspace,
    pub runner_resolution: RunnerResolution,
    pub config_warnings: Vec<RunnerWarning>,
}

pub fn probe_and_build_context(
    code_paths: Vec<PathBuf>,
    change_paths: Vec<PathBuf>,
    ai_mode: AiMode,
    resolved_spec: ResolvedSpec,
    cli_runner: Option<&str>,
) -> SpecResult<VerificationContext> {
    probe_and_build_context_with_registry(
        RunnerRegistry::default(),
        code_paths,
        change_paths,
        ai_mode,
        resolved_spec,
        cli_runner,
    )
}

pub fn probe_and_build_context_with_registry(
    registry: RunnerRegistry,
    code_paths: Vec<PathBuf>,
    change_paths: Vec<PathBuf>,
    ai_mode: AiMode,
    resolved_spec: ResolvedSpec,
    cli_runner: Option<&str>,
) -> SpecResult<VerificationContext> {
    probe_and_build_context_with_registry_and_host(
        registry,
        code_paths,
        change_paths,
        ai_mode,
        resolved_spec,
        cli_runner,
        current_host_platform(),
    )
}

fn probe_and_build_context_with_registry_and_host(
    registry: RunnerRegistry,
    code_paths: Vec<PathBuf>,
    change_paths: Vec<PathBuf>,
    ai_mode: AiMode,
    resolved_spec: ResolvedSpec,
    cli_runner: Option<&str>,
    host_platform: HostPlatform,
) -> SpecResult<VerificationContext> {
    let selection = resolve_runner_choice(&registry, &resolved_spec, cli_runner)?;

    if let RunnerSelection::ByName { name, .. } = &selection
        && let Some(runner) = registry.get(name)
    {
        ensure_runner_supported_on_host(runner.as_ref(), host_platform)?;
    }

    let root = find_workspace_root(&code_paths);
    let markers = probe_workspace_markers(root.as_deref());
    let (runner, mut runner_resolution) = resolve_detected_runner(&registry, selection, &markers)?;
    ensure_runner_supported_on_host(runner.as_ref(), host_platform)?;
    let source_files = collect_source_files(&code_paths, runner.source_extensions())?;
    let runner_workspace = RunnerWorkspace::new(
        root,
        code_paths.clone(),
        resolved_spec.task.meta.runner_config.clone(),
        markers,
        source_files,
    );
    let config_warnings = build_config_warnings(runner.as_ref(), &runner_workspace);
    runner_resolution.config_warnings = config_warnings.clone();

    Ok(VerificationContext {
        code_paths,
        change_paths,
        ai_mode,
        resolved_spec,
        runner,
        runner_workspace,
        runner_resolution,
        config_warnings,
    })
}

fn build_config_warnings(
    runner: &dyn TestRunner,
    workspace: &RunnerWorkspace,
) -> Vec<RunnerWarning> {
    let recognized = runner.recognized_config_keys();
    workspace
        .config
        .keys()
        .filter(|key| !recognized.contains(&key.as_str()))
        .map(|key| RunnerWarning {
            runner: runner.id().to_string(),
            key: key.clone(),
            reason: format!(
                "unknown `{key}` runner_config key for `{}` runner",
                runner.id()
            ),
        })
        .collect()
}

fn ensure_runner_supported_on_host(
    runner: &dyn TestRunner,
    host_platform: HostPlatform,
) -> SpecResult<()> {
    let supported = runner.supported_host_platforms();
    if supported
        .iter()
        .any(|platform| *platform == HostPlatform::All || *platform == host_platform)
    {
        return Ok(());
    }

    Err(SpecError::Verification(format!(
        "`{}` test runner is not supported on `{}` host",
        runner.id(),
        host_platform.as_str()
    )))
}

fn current_host_platform() -> HostPlatform {
    match std::env::consts::OS {
        "linux" => HostPlatform::Linux,
        "macos" => HostPlatform::MacOS,
        "windows" => HostPlatform::Windows,
        _ => HostPlatform::Other,
    }
}

/// Trait for scenario verifiers.
pub trait Verifier: Send + Sync {
    fn name(&self) -> &str;
    fn verify(&self, ctx: &VerificationContext) -> SpecResult<Vec<ScenarioResult>>;
}

/// Run verification with a set of verifiers.
pub fn run_verification(
    ctx: &VerificationContext,
    verifiers: &[&dyn Verifier],
) -> SpecResult<VerificationReport> {
    let mut all_results = Vec::new();
    let mut covered_scenarios = HashSet::new();

    for verifier in verifiers {
        let results = verifier.verify(ctx)?;
        for result in results {
            if !covered_scenarios.insert(result.scenario_name.clone()) {
                continue;
            }
            all_results.push(result);
        }
    }

    for scenario in &ctx.resolved_spec.all_scenarios {
        if covered_scenarios.contains(&scenario.name) {
            continue;
        }

        let step_results: Vec<StepVerdict> = scenario
            .steps
            .iter()
            .map(|step| StepVerdict {
                step_text: step.text.clone(),
                verdict: Verdict::Skip,
                reason: "no verifier covered this step".into(),
            })
            .collect();

        all_results.push(ScenarioResult {
            scenario_name: scenario.name.clone(),
            verdict: Verdict::Skip,
            step_results,
            evidence: Vec::new(),
            duration_ms: 0,
        });
    }

    Ok(VerificationReport::from_results(
        ctx.resolved_spec.task.meta.name.clone(),
        all_results,
    ))
}

fn find_workspace_root(code_paths: &[PathBuf]) -> Option<PathBuf> {
    for path in code_paths {
        let mut current = if path.is_file() {
            path.parent()?.to_path_buf()
        } else {
            path.clone()
        };

        loop {
            if has_workspace_marker(&current) {
                return Some(current);
            }
            if !current.pop() {
                break;
            }
        }
    }

    None
}

fn probe_workspace_markers(root: Option<&Path>) -> WorkspaceMarkers {
    let mut markers = Vec::new();
    if let Some(root) = root {
        for marker in WORKSPACE_MARKERS {
            if root.join(marker).is_file() {
                markers.push(*marker);
            }
        }
    }
    WorkspaceMarkers::from_files(markers)
}

fn collect_source_files(
    code_paths: &[PathBuf],
    source_extensions: &[&str],
) -> SpecResult<Vec<RunnerSourceFile>> {
    let mut files = Vec::new();
    let mut paths = Vec::new();

    for path in code_paths {
        if path.is_file() {
            if has_source_extension(path, source_extensions) {
                paths.push(path.clone());
            }
        } else if path.is_dir() {
            collect_source_paths(path, source_extensions, &mut paths);
        }
    }

    for path in paths {
        files.push(RunnerSourceFile {
            content: std::fs::read_to_string(&path)?,
            path,
        });
    }

    Ok(files)
}

fn collect_source_paths(dir: &Path, source_extensions: &[&str], files: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|name| name.to_str())
                && (name.starts_with('.') || name == "target" || name == "build")
            {
                continue;
            }
            collect_source_paths(&path, source_extensions, files);
        } else if has_source_extension(&path, source_extensions) {
            files.push(path);
        }
    }
}

const WORKSPACE_ROOT_MARKERS: &[&str] = &[
    "Cargo.toml",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    "settings.gradle",
    "settings.gradle.kts",
];

const WORKSPACE_MARKERS: &[&str] = &[
    "Cargo.toml",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    "settings.gradle",
    "settings.gradle.kts",
    "mvnw",
    "mvnw.cmd",
    "gradlew",
    "gradlew.bat",
];

fn has_workspace_marker(dir: &Path) -> bool {
    WORKSPACE_ROOT_MARKERS
        .iter()
        .any(|marker| dir.join(marker).is_file())
}

fn has_source_extension(path: &Path, source_extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| source_extensions.contains(&extension))
}

#[cfg(test)]
impl VerificationContext {
    pub(crate) fn for_test(
        code_paths: Vec<PathBuf>,
        change_paths: Vec<PathBuf>,
        ai_mode: AiMode,
        resolved_spec: ResolvedSpec,
    ) -> Self {
        Self {
            code_paths,
            change_paths,
            ai_mode,
            resolved_spec,
            runner: Arc::new(CargoRunner),
            runner_workspace: RunnerWorkspace::for_test("."),
            runner_resolution: RunnerResolution {
                name: "cargo".into(),
                source: ResolutionSource::Detected,
                overridden_spec: None,
                config_warnings: Vec::new(),
            },
            config_warnings: Vec::new(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::collections::{BTreeMap, HashMap};
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use crate::spec_core::{
        ResolvedSpec, Scenario, ScenarioResult, Section, Span, SpecDocument, SpecLevel, SpecMeta,
        Step, StepKind, TestSelector, Verdict,
    };

    use super::{
        AiMode, CargoRunner, HostPlatform, ResolutionSource, RunnerRegistry, RunnerResolution,
        RunnerWarning, RunnerWorkspace, TestCommand, TestRunner, VerificationContext, Verifier,
        WorkspaceMarkers, probe_and_build_context_with_registry_and_host, run_verification,
    };

    struct FirstVerifier;
    struct SecondVerifier;
    struct ConfigRunner;

    impl Verifier for FirstVerifier {
        fn name(&self) -> &str {
            "first"
        }

        fn verify(
            &self,
            _ctx: &VerificationContext,
        ) -> crate::spec_core::SpecResult<Vec<ScenarioResult>> {
            Ok(vec![ScenarioResult {
                scenario_name: "同一场景".into(),
                verdict: Verdict::Pass,
                step_results: vec![],
                evidence: vec![],
                duration_ms: 0,
            }])
        }
    }

    impl Verifier for SecondVerifier {
        fn name(&self) -> &str {
            "second"
        }

        fn verify(
            &self,
            _ctx: &VerificationContext,
        ) -> crate::spec_core::SpecResult<Vec<ScenarioResult>> {
            Ok(vec![ScenarioResult {
                scenario_name: "同一场景".into(),
                verdict: Verdict::Uncertain,
                step_results: vec![],
                evidence: vec![],
                duration_ms: 0,
            }])
        }
    }

    impl TestRunner for ConfigRunner {
        fn id(&self) -> &'static str {
            "config_runner"
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
                program: "cargo".into(),
                args: vec!["test".into()],
            })
        }

        fn scan_legacy_bindings(
            &self,
            _workspace: &RunnerWorkspace,
        ) -> crate::spec_core::SpecResult<HashMap<String, String>> {
            Ok(HashMap::new())
        }

        fn recognized_config_keys(&self) -> &'static [&'static str] {
            &["known"]
        }
    }

    #[test]
    fn run_verification_keeps_first_result_for_same_scenario() {
        let scenario = Scenario {
            name: "同一场景".into(),
            steps: vec![Step {
                kind: StepKind::Given,
                text: "前置条件".into(),
                params: vec![],
                table: vec![],
                span: Span::line(1),
            }],
            test_selector: None,
            tags: vec![],
            review: Default::default(),
            mode: Default::default(),
            depends_on: vec![],
            span: Span::line(1),
        };
        let ctx = VerificationContext {
            code_paths: vec![PathBuf::from(".")],
            change_paths: vec![],
            ai_mode: AiMode::Off,
            resolved_spec: ResolvedSpec {
                task: SpecDocument {
                    meta: SpecMeta {
                        level: SpecLevel::Task,
                        name: "test".into(),
                        inherits: None,
                        lang: vec![],
                        tags: vec![],
                        runner: None,
                        runner_config: Default::default(),
                        depends: vec![],
                        estimate: None,
                    },
                    sections: vec![Section::AcceptanceCriteria {
                        scenarios: vec![scenario.clone()],
                        span: Span::line(1),
                    }],
                    source_path: PathBuf::new(),
                },
                inherited_constraints: vec![],
                inherited_decisions: vec![],
                all_scenarios: vec![scenario],
            },
            runner: Arc::new(CargoRunner),
            runner_workspace: RunnerWorkspace::for_test("."),
            runner_resolution: RunnerResolution {
                name: "cargo".into(),
                source: ResolutionSource::Detected,
                overridden_spec: None,
                config_warnings: Vec::new(),
            },
            config_warnings: Vec::new(),
        };

        let first = FirstVerifier;
        let second = SecondVerifier;
        let report = run_verification(&ctx, &[&first, &second]).unwrap();

        assert_eq!(report.results.len(), 1);
        assert_eq!(report.results[0].verdict, Verdict::Pass);
    }

    #[test]
    fn test_gradle_runner_walks_java_and_kotlin_sources() {
        let root = temp_workspace_path("gradle-java-kotlin-sources");
        write_file(&root.join("build.gradle.kts"), "plugins {}\n");
        write_file(&root.join("gradlew"), "#!/bin/sh\n");
        write_file(
            &root.join("src/test/java/com/example/JavaRulesTest.java"),
            "class JavaRulesTest {}\n",
        );
        write_file(
            &root.join("src/test/kotlin/com/example/KotlinRulesTest.kt"),
            "class KotlinRulesTest\n",
        );

        let ctx = probe_and_build_context_with_registry_and_host(
            RunnerRegistry::with_defaults(),
            vec![root.clone()],
            vec![],
            AiMode::Off,
            resolved_spec_with_runner(None),
            None,
            HostPlatform::MacOS,
        )
        .unwrap();

        let source_names: Vec<String> = ctx
            .runner_workspace
            .source_files
            .iter()
            .map(|source| {
                source
                    .path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap()
                    .to_string()
            })
            .collect();

        assert_eq!(ctx.runner_resolution.name, "gradle");
        assert!(source_names.contains(&"JavaRulesTest.java".to_string()));
        assert!(source_names.contains(&"KotlinRulesTest.kt".to_string()));
    }

    #[test]
    fn test_probe_context_collects_unknown_runner_config_warnings() {
        let mut registry = RunnerRegistry::new();
        registry.register(Arc::new(ConfigRunner));
        let ctx = probe_and_build_context_with_registry_and_host(
            registry,
            vec![PathBuf::from("src/spec_verify/mod.rs")],
            vec![],
            AiMode::Off,
            resolved_spec_with_config(BTreeMap::from([
                ("known".to_string(), "ok".to_string()),
                ("typo".to_string(), "bad".to_string()),
            ])),
            Some("config_runner"),
            HostPlatform::Linux,
        )
        .unwrap();

        let expected = vec![RunnerWarning {
            runner: "config_runner".into(),
            key: "typo".into(),
            reason: "unknown `typo` runner_config key for `config_runner` runner".into(),
        }];
        assert_eq!(ctx.config_warnings, expected);
        assert_eq!(ctx.runner_resolution.config_warnings, expected);
    }

    fn resolved_spec_with_runner(runner: Option<&str>) -> ResolvedSpec {
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

    fn resolved_spec_with_config(config: BTreeMap<String, String>) -> ResolvedSpec {
        ResolvedSpec {
            task: SpecDocument {
                meta: SpecMeta {
                    level: SpecLevel::Task,
                    name: "config spec".into(),
                    inherits: None,
                    lang: vec![],
                    tags: vec![],
                    runner: None,
                    runner_config: config,
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

    fn temp_workspace_path(name: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("agent-spec-{name}-{unique}"))
    }

    fn write_file(path: &Path, content: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn test_single_verification_context_literal_in_production() {
        let paths = [
            "src/main.rs",
            "src/spec_gateway/lifecycle.rs",
            "src/spec_verify/mod.rs",
        ];
        let constructors: Vec<String> = paths
            .iter()
            .flat_map(|path| {
                let source = std::fs::read_to_string(path).unwrap();
                let production = production_source_before_tests(&source);
                production
                    .lines()
                    .enumerate()
                    .filter_map(move |(index, line)| {
                        if line.contains("VerificationContext {")
                            && !line.contains("struct")
                            && !line.contains("impl ")
                        {
                            Some(format!("{path}:{}", index + 1))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        assert_eq!(
            constructors.len(),
            1,
            "VerificationContext construction should be centralized in probe_and_build_context: {constructors:?}"
        );
        assert!(
            constructors[0].starts_with("src/spec_verify/mod.rs:"),
            "single constructor should live in src/spec_verify/mod.rs: {constructors:?}"
        );
    }

    fn production_source_before_tests(source: &str) -> &str {
        source
            .split("\n#[cfg(test)]\n#[allow(clippy::unwrap_used)]\nmod tests")
            .next()
            .and_then(|prefix| prefix.split("\n#[cfg(test)]\nmod tests").next())
            .unwrap_or(source)
    }

    #[test]
    fn test_runners_module_io_grep_guard() {
        let runners_dir = std::path::Path::new("src/spec_verify/runners");
        assert!(
            runners_dir.is_dir(),
            "runner-specific code should live under src/spec_verify/runners"
        );

        let mut files = Vec::new();
        collect_rs_files(runners_dir, &mut files);
        assert!(
            !files.is_empty(),
            "runner module should contain Rust source files"
        );

        let offenders: Vec<String> = files
            .iter()
            .flat_map(|path| {
                let source = std::fs::read_to_string(path).unwrap();
                source
                    .lines()
                    .enumerate()
                    .filter_map(move |(index, line)| {
                        if line.contains("fs::") || line.contains("Command::new") {
                            Some(format!("{}:{}", path.display(), index + 1))
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        assert!(
            offenders.is_empty(),
            "runner methods outside preflight should stay IO-free: {offenders:?}"
        );
    }

    fn collect_rs_files(dir: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                collect_rs_files(&path, files);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path);
            }
        }
    }
}
