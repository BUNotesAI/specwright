mod ai_verifier;
mod boundaries;
mod complexity;
mod runners;
mod structural;
mod test_verifier;

use std::collections::{BTreeMap, BTreeSet, HashSet};
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
pub use runners::{
    HostPlatform, NodePackageManager, NodePackageManagerDecision, NodePackageManagerSource,
    NodeProjectMetadata, RunnerSelection, RunnerWarning, RunnerWorkspaceMetadata, TestCommand,
};
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
    let source_files = collect_source_files(
        &code_paths,
        runner.source_extensions(),
        runner.ignored_source_dirs(),
    )?;
    let metadata = build_workspace_metadata(
        runner.as_ref(),
        root.as_deref(),
        &markers,
        &resolved_spec.task.meta.runner_config,
    )?;
    let runner_workspace = RunnerWorkspace::new(
        root,
        code_paths.clone(),
        resolved_spec.task.meta.runner_config.clone(),
        markers,
        source_files,
        metadata,
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

fn build_workspace_metadata(
    runner: &dyn TestRunner,
    root: Option<&Path>,
    markers: &WorkspaceMarkers,
    config: &BTreeMap<String, String>,
) -> SpecResult<RunnerWorkspaceMetadata> {
    if runner.id() != "node" {
        return Ok(RunnerWorkspaceMetadata::default());
    }

    Ok(RunnerWorkspaceMetadata {
        node: Some(build_node_project_metadata(root, markers, config)?),
    })
}

fn build_node_project_metadata(
    root: Option<&Path>,
    markers: &WorkspaceMarkers,
    config: &BTreeMap<String, String>,
) -> SpecResult<NodeProjectMetadata> {
    let root = root.ok_or_else(|| {
        SpecError::Verification("node runner requires a workspace root with package.json".into())
    })?;
    let package_json_path = root.join("package.json");
    let package_json = std::fs::read_to_string(&package_json_path).map_err(|err| {
        SpecError::Verification(format!(
            "node runner requires readable package.json at {}: {err}",
            package_json_path.display()
        ))
    })?;
    let parsed: serde_json::Value = serde_json::from_str(&package_json).map_err(|err| {
        SpecError::Verification(format!(
            "node runner could not parse package.json at {}: {err}",
            package_json_path.display()
        ))
    })?;
    let scripts = parsed
        .get("scripts")
        .and_then(serde_json::Value::as_object)
        .map(|scripts| scripts.keys().cloned().collect::<BTreeSet<_>>())
        .unwrap_or_default();
    let package_json_package_manager = parsed
        .get("packageManager")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let lockfiles = node_lockfiles_from_markers(markers);
    let package_manager =
        select_node_package_manager(config, package_json_package_manager.as_deref(), &lockfiles)?;

    Ok(NodeProjectMetadata {
        package_manager,
        scripts,
        package_json_package_manager,
        lockfiles,
    })
}

fn node_lockfiles_from_markers(markers: &WorkspaceMarkers) -> BTreeSet<String> {
    NODE_LOCKFILES
        .iter()
        .filter(|lockfile| markers.contains(lockfile))
        .map(|lockfile| (*lockfile).to_string())
        .collect()
}

fn select_node_package_manager(
    config: &BTreeMap<String, String>,
    package_json_package_manager: Option<&str>,
    lockfiles: &BTreeSet<String>,
) -> SpecResult<NodePackageManagerDecision> {
    if let Some(configured) = config.get("package_manager") {
        let manager = parse_node_package_manager(configured, "package_manager")?;
        return Ok(NodePackageManagerDecision {
            manager,
            source: NodePackageManagerSource::RunnerConfig,
        });
    }

    if let Some(raw) = package_json_package_manager {
        let name = raw.split('@').next().unwrap_or(raw);
        let manager = parse_node_package_manager(name, "packageManager")?;
        return Ok(NodePackageManagerDecision {
            manager,
            source: NodePackageManagerSource::PackageJson,
        });
    }

    if lockfiles.len() > 1 {
        return Err(SpecError::Verification(format!(
            "multiple node lockfiles found without package-manager selection: {}",
            lockfiles.iter().cloned().collect::<Vec<_>>().join(", ")
        )));
    }

    if let Some(lockfile) = lockfiles.iter().next() {
        return Ok(NodePackageManagerDecision {
            manager: node_manager_for_lockfile(lockfile)?,
            source: NodePackageManagerSource::Lockfile(lockfile.clone()),
        });
    }

    Ok(NodePackageManagerDecision {
        manager: NodePackageManager::Npm,
        source: NodePackageManagerSource::DefaultNpm,
    })
}

fn parse_node_package_manager(value: &str, field_name: &str) -> SpecResult<NodePackageManager> {
    match value {
        "npm" => Ok(NodePackageManager::Npm),
        "pnpm" => Ok(NodePackageManager::Pnpm),
        "yarn" => Ok(NodePackageManager::Yarn),
        "bun" => Ok(NodePackageManager::Bun),
        other => Err(SpecError::Verification(format!(
            "invalid node {field_name} value `{other}`; expected npm, pnpm, yarn, or bun"
        ))),
    }
}

fn node_manager_for_lockfile(lockfile: &str) -> SpecResult<NodePackageManager> {
    match lockfile {
        "pnpm-lock.yaml" => Ok(NodePackageManager::Pnpm),
        "bun.lock" | "bun.lockb" => Ok(NodePackageManager::Bun),
        "yarn.lock" => Ok(NodePackageManager::Yarn),
        "package-lock.json" => Ok(NodePackageManager::Npm),
        other => Err(SpecError::Verification(format!(
            "unsupported node lockfile marker `{other}`"
        ))),
    }
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
        "`{}` test runner requires {}; current host is `{}`",
        runner.id(),
        supported_host_label(supported),
        host_platform.as_str()
    )))
}

fn supported_host_label(platforms: &[HostPlatform]) -> String {
    platforms
        .iter()
        .map(|platform| match platform {
            HostPlatform::All => "all hosts",
            HostPlatform::Linux => "Linux",
            HostPlatform::MacOS => "macOS",
            HostPlatform::Windows => "Windows",
            HostPlatform::Other => "other",
        })
        .collect::<Vec<_>>()
        .join(", ")
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
        let mut node_candidate = None;

        loop {
            if has_non_node_workspace_marker(&current) {
                return Some(current);
            }
            if node_candidate.is_none() && has_node_workspace_marker(&current) {
                node_candidate = Some(current.clone());
            }
            if !current.pop() {
                break;
            }
        }

        if node_candidate.is_some() {
            return node_candidate;
        }
    }

    None
}

fn probe_workspace_markers(root: Option<&Path>) -> WorkspaceMarkers {
    let mut markers = Vec::new();
    if let Some(root) = root {
        for marker in WORKSPACE_MARKERS {
            if workspace_marker_exists(root, marker) {
                markers.push(*marker);
            }
        }
    }
    WorkspaceMarkers::from_files(markers)
}

fn collect_source_files(
    code_paths: &[PathBuf],
    source_extensions: &[&str],
    ignored_source_dirs: &[&str],
) -> SpecResult<Vec<RunnerSourceFile>> {
    let mut files = Vec::new();
    let mut paths = Vec::new();

    for path in code_paths {
        if path.is_file() {
            if has_source_extension(path, source_extensions) {
                paths.push(path.clone());
            }
        } else if path.is_dir() {
            collect_source_paths(path, source_extensions, ignored_source_dirs, &mut paths);
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

fn collect_source_paths(
    dir: &Path,
    source_extensions: &[&str],
    ignored_source_dirs: &[&str],
    files: &mut Vec<PathBuf>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|name| name.to_str())
                && (name.starts_with('.')
                    || name == "target"
                    || name == "build"
                    || ignored_source_dirs.contains(&name))
            {
                continue;
            }
            collect_source_paths(&path, source_extensions, ignored_source_dirs, files);
        } else if has_source_extension(&path, source_extensions) {
            files.push(path);
        }
    }
}

const NON_NODE_WORKSPACE_ROOT_MARKERS: &[&str] = &[
    "Cargo.toml",
    "AndroidManifest.xml",
    "Package.swift",
    "*.xcodeproj",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    "settings.gradle",
    "settings.gradle.kts",
];

const NODE_WORKSPACE_ROOT_MARKERS: &[&str] = &["package.json"];

const WORKSPACE_MARKERS: &[&str] = &[
    "Cargo.toml",
    "AndroidManifest.xml",
    "Package.swift",
    "*.xcodeproj",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    "settings.gradle",
    "settings.gradle.kts",
    "mvnw",
    "mvnw.cmd",
    "gradlew",
    "gradlew.bat",
    "package.json",
    "pnpm-lock.yaml",
    "bun.lock",
    "bun.lockb",
    "yarn.lock",
    "package-lock.json",
];

const NODE_LOCKFILES: &[&str] = &[
    "pnpm-lock.yaml",
    "bun.lock",
    "bun.lockb",
    "yarn.lock",
    "package-lock.json",
];

fn has_non_node_workspace_marker(dir: &Path) -> bool {
    NON_NODE_WORKSPACE_ROOT_MARKERS
        .iter()
        .any(|marker| workspace_marker_exists(dir, marker))
}

fn has_node_workspace_marker(dir: &Path) -> bool {
    NODE_WORKSPACE_ROOT_MARKERS
        .iter()
        .any(|marker| workspace_marker_exists(dir, marker))
}

fn workspace_marker_exists(dir: &Path, marker: &str) -> bool {
    if marker == "*.xcodeproj" {
        return contains_xcodeproj_dir(dir);
    }

    let path = dir.join(marker);
    path.is_file() || path.is_dir()
}

fn contains_xcodeproj_dir(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };

    entries.flatten().any(|entry| {
        let path = entry.path();
        path.is_dir()
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".xcodeproj"))
    })
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
    use std::collections::{BTreeMap, BTreeSet, HashMap};
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use crate::spec_core::{
        ResolvedSpec, Scenario, ScenarioResult, Section, Span, SpecDocument, SpecLevel, SpecMeta,
        Step, StepKind, TestSelector, Verdict,
    };

    use super::{
        AiMode, CargoRunner, HostPlatform, NodePackageManager, NodePackageManagerDecision,
        NodePackageManagerSource, ResolutionSource, RunnerRegistry, RunnerResolution,
        RunnerWarning, RunnerWorkspace, TestCommand, TestRunner, VerificationContext, Verifier,
        WorkspaceMarkers, probe_and_build_context_with_registry_and_host, run_verification,
        select_node_package_manager,
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

    #[test]
    fn test_ios_host_gate_by_name_no_io() {
        let result = probe_and_build_context_with_registry_and_host(
            RunnerRegistry::with_defaults(),
            vec![PathBuf::from("this/path/must/not/be/probed")],
            vec![],
            AiMode::Off,
            resolved_spec_with_runner(None),
            Some("ios"),
            HostPlatform::Linux,
        );
        let Err(err) = result else {
            panic!("iOS runner should be rejected on a non-macOS host before workspace probing");
        };
        let message = err.to_string();

        assert!(message.contains("ios"));
        assert!(message.contains("requires macOS"));
    }

    #[test]
    fn test_ios_host_gate_needs_detect_marker_only_io() {
        let root = temp_workspace_path("ios-needs-detect-host-gate");
        write_file(
            &root.join("MyApp.xcodeproj/project.pbxproj"),
            "// xcode project marker\n",
        );

        let result = probe_and_build_context_with_registry_and_host(
            RunnerRegistry::with_defaults(),
            vec![root],
            vec![],
            AiMode::Off,
            resolved_spec_with_runner(None),
            None,
            HostPlatform::Linux,
        );
        let Err(err) = result else {
            panic!("detected iOS runner should be rejected on a non-macOS host");
        };
        let message = err.to_string();

        assert!(message.contains("ios"));
        assert!(message.contains("requires macOS"));
    }

    #[test]
    fn test_ios_unknown_config_key_surfaces_through_warnings() {
        let ctx = probe_and_build_context_with_registry_and_host(
            RunnerRegistry::with_defaults(),
            vec![PathBuf::from("src/spec_verify/mod.rs")],
            vec![],
            AiMode::Off,
            resolved_spec_with_runner_and_config(
                None,
                BTreeMap::from([("destinaiton".to_string(), "typo".to_string())]),
            ),
            Some("ios"),
            HostPlatform::MacOS,
        )
        .unwrap();

        let expected = vec![RunnerWarning {
            runner: "ios".into(),
            key: "destinaiton".into(),
            reason: "unknown `destinaiton` runner_config key for `ios` runner".into(),
        }];
        assert_eq!(ctx.config_warnings, expected);
        assert_eq!(ctx.runner_resolution.config_warnings, expected);
    }

    #[test]
    fn test_node_markers_do_not_steal_nested_cargo_workspace_root() {
        let root = temp_workspace_path("node-marker-cargo-root-regression");
        write_file(
            &root.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        );
        write_file(
            &root.join("frontend/package.json"),
            "{\"scripts\":{\"test\":\"node test.js\"}}\n",
        );
        write_file(
            &root.join("frontend/src/app.ts"),
            "export const value = 1;\n",
        );

        let ctx = probe_and_build_context_with_registry_and_host(
            RunnerRegistry::with_defaults(),
            vec![root.join("frontend/src/app.ts")],
            vec![],
            AiMode::Off,
            resolved_spec_with_runner(None),
            None,
            HostPlatform::MacOS,
        )
        .unwrap();

        assert_eq!(ctx.runner_workspace.root, Some(root));
        assert_eq!(ctx.runner_resolution.name, "cargo");
    }

    #[test]
    fn test_node_mixed_workspace_detection_precedence_and_explicit_override() {
        let root = temp_workspace_path("node-cargo-mixed-root");
        write_file(
            &root.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        );
        write_file(
            &root.join("package.json"),
            "{\"scripts\":{\"test\":\"node test.js\"}}\n",
        );

        let auto = probe_and_build_context_with_registry_and_host(
            RunnerRegistry::with_defaults(),
            vec![root.clone()],
            vec![],
            AiMode::Off,
            resolved_spec_with_runner(None),
            None,
            HostPlatform::MacOS,
        )
        .unwrap();
        assert_eq!(auto.runner_resolution.name, "cargo");
        assert!(auto.runner_workspace.metadata.node.is_none());

        let explicit = probe_and_build_context_with_registry_and_host(
            RunnerRegistry::with_defaults(),
            vec![root],
            vec![],
            AiMode::Off,
            resolved_spec_with_runner(Some("node")),
            None,
            HostPlatform::MacOS,
        )
        .unwrap();
        assert_eq!(explicit.runner_resolution.name, "node");
        assert!(explicit.runner_workspace.metadata.node.is_some());
    }

    #[test]
    fn test_node_workspace_markers_feed_probe_metadata() {
        let root = temp_workspace_path("node-markers-metadata");
        write_file(
            &root.join("package.json"),
            r#"{"packageManager":"pnpm@9.0.0","scripts":{"test":"node test.js","build":"node build.js"}}"#,
        );
        for lockfile in [
            "pnpm-lock.yaml",
            "bun.lock",
            "bun.lockb",
            "yarn.lock",
            "package-lock.json",
        ] {
            write_file(&root.join(lockfile), "");
        }

        let ctx = probe_and_build_context_with_registry_and_host(
            RunnerRegistry::with_defaults(),
            vec![root],
            vec![],
            AiMode::Off,
            resolved_spec_with_runner(None),
            None,
            HostPlatform::MacOS,
        )
        .unwrap();
        let metadata = ctx.runner_workspace.metadata.node.as_ref().unwrap();

        assert_eq!(ctx.runner_resolution.name, "node");
        assert_eq!(
            metadata.package_json_package_manager.as_deref(),
            Some("pnpm@9.0.0")
        );
        assert_eq!(
            metadata.package_manager,
            NodePackageManagerDecision {
                manager: NodePackageManager::Pnpm,
                source: NodePackageManagerSource::PackageJson,
            }
        );
        assert!(metadata.scripts.contains("test"));
        assert!(metadata.scripts.contains("build"));
        assert_eq!(
            metadata.lockfiles,
            BTreeSet::from([
                "bun.lock".to_string(),
                "bun.lockb".to_string(),
                "package-lock.json".to_string(),
                "pnpm-lock.yaml".to_string(),
                "yarn.lock".to_string(),
            ])
        );
    }

    #[test]
    fn test_node_project_metadata_is_probed_once_and_carried_to_workspace() {
        let root = temp_workspace_path("node-probed-metadata");
        write_file(
            &root.join("package.json"),
            r#"{"packageManager":"yarn@4.0.0","scripts":{"test":"node test.js","typecheck":"node typecheck.js"}}"#,
        );
        write_file(
            &root.join("src/example.test.ts"),
            "// @spec: node example\ntest(\"node example\", () => {})\n",
        );

        let ctx = probe_and_build_context_with_registry_and_host(
            RunnerRegistry::with_defaults(),
            vec![root],
            vec![],
            AiMode::Off,
            resolved_spec_with_runner(Some("node")),
            None,
            HostPlatform::MacOS,
        )
        .unwrap();
        let metadata = ctx.runner_workspace.metadata.node.as_ref().unwrap();

        assert_eq!(
            metadata.package_json_package_manager.as_deref(),
            Some("yarn@4.0.0")
        );
        assert_eq!(metadata.package_manager.manager, NodePackageManager::Yarn);
        assert!(metadata.scripts.contains("test"));
        assert!(metadata.scripts.contains("typecheck"));
        assert_eq!(ctx.runner_workspace.source_files.len(), 1);
        assert!(
            ctx.runner
                .build_test_command(
                    &ctx.runner_workspace,
                    &TestSelector {
                        package: None,
                        filter: "-".into(),
                        level: Some("unit".into()),
                        test_double: None,
                        targets: None,
                    }
                )
                .is_ok()
        );
    }

    #[test]
    fn test_node_package_manager_precedence_matrix() {
        assert_eq!(
            select_node_package_manager(
                &BTreeMap::from([("package_manager".to_string(), "bun".to_string())]),
                Some("pnpm@9.0.0"),
                &BTreeSet::from(["yarn.lock".to_string()])
            )
            .unwrap(),
            NodePackageManagerDecision {
                manager: NodePackageManager::Bun,
                source: NodePackageManagerSource::RunnerConfig,
            }
        );
        assert_eq!(
            select_node_package_manager(
                &BTreeMap::new(),
                Some("pnpm@9.0.0"),
                &BTreeSet::from(["yarn.lock".to_string()])
            )
            .unwrap(),
            NodePackageManagerDecision {
                manager: NodePackageManager::Pnpm,
                source: NodePackageManagerSource::PackageJson,
            }
        );
        assert_eq!(
            select_node_package_manager(
                &BTreeMap::new(),
                None,
                &BTreeSet::from(["bun.lock".to_string()])
            )
            .unwrap(),
            NodePackageManagerDecision {
                manager: NodePackageManager::Bun,
                source: NodePackageManagerSource::Lockfile("bun.lock".into()),
            }
        );
        assert_eq!(
            select_node_package_manager(&BTreeMap::new(), None, &BTreeSet::new()).unwrap(),
            NodePackageManagerDecision {
                manager: NodePackageManager::Npm,
                source: NodePackageManagerSource::DefaultNpm,
            }
        );

        let multiple_lockfiles = select_node_package_manager(
            &BTreeMap::new(),
            None,
            &BTreeSet::from([
                "package-lock.json".to_string(),
                "pnpm-lock.yaml".to_string(),
            ]),
        )
        .unwrap_err()
        .to_string();
        assert!(multiple_lockfiles.contains("multiple node lockfiles"));
    }

    #[test]
    fn test_node_invalid_package_manager_value_errors() {
        let err = select_node_package_manager(
            &BTreeMap::from([("package_manager".to_string(), "deno".to_string())]),
            Some("pnpm@9.0.0"),
            &BTreeSet::new(),
        )
        .unwrap_err()
        .to_string();

        assert!(err.contains("package_manager"));
        assert!(err.contains("deno"));
    }

    #[test]
    fn test_node_invalid_package_json_package_manager_value_errors() {
        let root = temp_workspace_path("node-invalid-package-json-package-manager");
        write_file(
            &root.join("package.json"),
            r#"{"packageManager":"deno@2.0.0","scripts":{"test":"node test.js"}}"#,
        );

        let err = match probe_and_build_context_with_registry_and_host(
            RunnerRegistry::with_defaults(),
            vec![root],
            vec![],
            AiMode::Off,
            resolved_spec_with_runner(Some("node")),
            None,
            HostPlatform::MacOS,
        ) {
            Ok(_) => panic!("invalid packageManager should fail verification"),
            Err(err) => err,
        };
        let message = err.to_string();

        assert!(message.contains("packageManager"));
        assert!(message.contains("deno"));
        assert!(!message.contains("RunnerWarning"));
    }

    #[test]
    fn test_node_unknown_runner_config_keys_warn() {
        let root = temp_workspace_path("node-unknown-config-warning");
        write_file(
            &root.join("package.json"),
            r#"{"scripts":{"test":"node test.js"}}"#,
        );

        let ctx = probe_and_build_context_with_registry_and_host(
            RunnerRegistry::with_defaults(),
            vec![root],
            vec![],
            AiMode::Off,
            resolved_spec_with_runner_and_config(
                Some("node"),
                BTreeMap::from([
                    ("unit_filter_style".to_string(), "none".to_string()),
                    ("unit_fitler_style".to_string(), "typo".to_string()),
                ]),
            ),
            None,
            HostPlatform::MacOS,
        )
        .unwrap();

        assert_eq!(
            ctx.config_warnings,
            vec![RunnerWarning {
                runner: "node".into(),
                key: "unit_fitler_style".into(),
                reason: "unknown `unit_fitler_style` runner_config key for `node` runner".into(),
            }]
        );
    }

    #[test]
    fn test_node_explicit_runner_requires_parseable_package_json() {
        let missing_root = temp_workspace_path("node-missing-package-json");
        std::fs::create_dir_all(&missing_root).unwrap();
        let missing_err = match probe_and_build_context_with_registry_and_host(
            RunnerRegistry::with_defaults(),
            vec![missing_root],
            vec![],
            AiMode::Off,
            resolved_spec_with_runner(Some("node")),
            None,
            HostPlatform::MacOS,
        ) {
            Ok(_) => panic!("missing package.json should fail verification"),
            Err(err) => err.to_string(),
        };
        assert!(missing_err.contains("package.json"));

        let malformed_root = temp_workspace_path("node-malformed-package-json");
        write_file(&malformed_root.join("package.json"), "{ invalid json");
        let malformed_err = match probe_and_build_context_with_registry_and_host(
            RunnerRegistry::with_defaults(),
            vec![malformed_root],
            vec![],
            AiMode::Off,
            resolved_spec_with_runner(Some("node")),
            None,
            HostPlatform::MacOS,
        ) {
            Ok(_) => panic!("malformed package.json should fail verification"),
            Err(err) => err.to_string(),
        };
        assert!(malformed_err.contains("package.json"));
    }

    #[test]
    fn test_node_source_collection_prunes_generated_and_vendor_dirs() {
        let node_root = temp_workspace_path("node-source-prune");
        write_file(
            &node_root.join("package.json"),
            "{\"scripts\":{\"test\":\"node test.js\"}}\n",
        );
        write_file(
            &node_root.join("tests/owned.test.ts"),
            "// @spec: owned\ntest(\"owned\", () => {})\n",
        );
        for dir in ["node_modules", "dist", "coverage", "playwright-report"] {
            write_file(
                &node_root.join(dir).join("ignored.test.ts"),
                "// @spec: ignored\ntest(\"ignored\", () => {})\n",
            );
        }

        let node_ctx = probe_and_build_context_with_registry_and_host(
            RunnerRegistry::with_defaults(),
            vec![node_root],
            vec![],
            AiMode::Off,
            resolved_spec_with_runner(Some("node")),
            None,
            HostPlatform::MacOS,
        )
        .unwrap();
        let source_names: Vec<String> = node_ctx
            .runner_workspace
            .source_files
            .iter()
            .map(|source| source.path.to_string_lossy().into_owned())
            .collect();
        assert_eq!(source_names.len(), 1);
        assert!(source_names[0].contains("owned.test.ts"));

        let cargo_root = temp_workspace_path("cargo-source-prune-regression");
        write_file(
            &cargo_root.join("Cargo.toml"),
            "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        );
        write_file(&cargo_root.join("dist/lib.rs"), "pub fn from_dist() {}\n");
        write_file(
            &cargo_root.join("coverage/lib.rs"),
            "pub fn from_coverage() {}\n",
        );
        write_file(
            &cargo_root.join("playwright-report/lib.rs"),
            "pub fn from_report() {}\n",
        );

        let cargo_ctx = probe_and_build_context_with_registry_and_host(
            RunnerRegistry::with_defaults(),
            vec![cargo_root],
            vec![],
            AiMode::Off,
            resolved_spec_with_runner(None),
            None,
            HostPlatform::MacOS,
        )
        .unwrap();
        let cargo_sources: Vec<String> = cargo_ctx
            .runner_workspace
            .source_files
            .iter()
            .map(|source| source.path.to_string_lossy().into_owned())
            .collect();
        assert!(
            cargo_sources
                .iter()
                .any(|path| path.contains("dist/lib.rs"))
        );
        assert!(
            cargo_sources
                .iter()
                .any(|path| path.contains("coverage/lib.rs"))
        );
        assert!(
            cargo_sources
                .iter()
                .any(|path| path.contains("playwright-report/lib.rs"))
        );
    }

    fn resolved_spec_with_runner(runner: Option<&str>) -> ResolvedSpec {
        resolved_spec_with_runner_and_config(runner, BTreeMap::new())
    }

    fn resolved_spec_with_runner_and_config(
        runner: Option<&str>,
        config: BTreeMap<String, String>,
    ) -> ResolvedSpec {
        ResolvedSpec {
            task: SpecDocument {
                meta: SpecMeta {
                    level: SpecLevel::Task,
                    name: "runner spec".into(),
                    inherits: None,
                    lang: vec![],
                    tags: vec![],
                    runner: runner.map(str::to_string),
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
                io_offenders_outside_preflight(path, &source)
            })
            .collect();

        assert!(
            offenders.is_empty(),
            "runner methods outside preflight should stay IO-free: {offenders:?}"
        );
    }

    fn io_offenders_outside_preflight(path: &std::path::Path, source: &str) -> Vec<String> {
        let mut offenders = Vec::new();
        let mut in_preflight = false;
        let mut seen_preflight_body = false;
        let mut brace_depth = 0isize;

        for (index, line) in source.lines().enumerate() {
            if !in_preflight && line.contains("fn preflight(") {
                in_preflight = true;
                seen_preflight_body = false;
                brace_depth = 0;
            }

            if (line.contains("fs::") || line.contains("Command::new")) && !in_preflight {
                offenders.push(format!("{}:{}", path.display(), index + 1));
            }

            if in_preflight {
                let opens = line.matches('{').count() as isize;
                let closes = line.matches('}').count() as isize;
                if opens > 0 {
                    seen_preflight_body = true;
                }
                brace_depth += opens - closes;
                if seen_preflight_body && brace_depth == 0 {
                    in_preflight = false;
                }
            }
        }

        offenders
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
