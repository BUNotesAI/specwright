use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::spec_core::{SpecResult, TestSelector};

/// Host platform supported by a test runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostPlatform {
    All,
    Linux,
    MacOS,
    Windows,
    Other,
}

impl HostPlatform {
    /// Returns the stable label used in host-gate diagnostics.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Linux => "linux",
            Self::MacOS => "macos",
            Self::Windows => "windows",
            Self::Other => "other",
        }
    }
}

/// Result of checking host or device capabilities before running tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreflightOutcome {
    Ready,
    MissingCapability { capability: String, reason: String },
}

/// Non-fatal runner configuration warning.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RunnerWarning {
    pub runner: String,
    pub key: String,
    pub reason: String,
}

/// Why a runner was selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionSource {
    CliFlag,
    SpecFrontmatter,
    Detected,
}

/// Pure runner choice before effectful workspace probing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunnerSelection {
    NeedsDetect,
    ByName {
        name: String,
        source: ResolutionSource,
        overridden_spec: Option<String>,
    },
}

/// Final runner resolution attached to a verification context.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RunnerResolution {
    pub name: String,
    pub source: ResolutionSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overridden_spec: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub config_warnings: Vec<RunnerWarning>,
}

/// Command data produced by a runner. Execution happens outside runner modules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestCommand {
    pub program: String,
    pub args: Vec<String>,
}

/// Source file contents available to pure runner scanners.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunnerSourceFile {
    pub path: PathBuf,
    pub content: String,
}

/// Marker files discovered by effectful workspace probing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkspaceMarkers {
    files: BTreeSet<String>,
}

impl WorkspaceMarkers {
    pub fn from_files(files: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            files: files.into_iter().map(Into::into).collect(),
        }
    }

    pub fn contains(&self, marker: &str) -> bool {
        self.files.contains(marker)
    }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.files.iter().map(String::as_str)
    }
}

/// Runner-specific typed metadata produced by effectful workspace probing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RunnerWorkspaceMetadata {
    pub node: Option<NodeProjectMetadata>,
}

/// Typed Node project metadata consumed by the pure Node runner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeProjectMetadata {
    pub package_manager: NodePackageManagerDecision,
    pub scripts: BTreeSet<String>,
    pub package_json_package_manager: Option<String>,
    pub lockfiles: BTreeSet<String>,
}

/// Selected Node package manager and the source of that decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodePackageManagerDecision {
    pub manager: NodePackageManager,
    pub source: NodePackageManagerSource,
}

/// Supported Node package managers for the v1 runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodePackageManager {
    Npm,
    Pnpm,
    Yarn,
    Bun,
}

impl NodePackageManager {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Npm => "npm",
            Self::Pnpm => "pnpm",
            Self::Yarn => "yarn",
            Self::Bun => "bun",
        }
    }
}

/// Source used to select a Node package manager.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodePackageManagerSource {
    RunnerConfig,
    PackageJson,
    Lockfile(String),
    DefaultNpm,
}

/// Runner-facing workspace model. It contains data, not hidden IO handles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunnerWorkspace {
    pub root: Option<PathBuf>,
    pub code_paths: Vec<PathBuf>,
    pub config: BTreeMap<String, String>,
    pub markers: WorkspaceMarkers,
    pub source_files: Vec<RunnerSourceFile>,
    pub metadata: RunnerWorkspaceMetadata,
}

impl RunnerWorkspace {
    pub fn new(
        root: Option<PathBuf>,
        code_paths: Vec<PathBuf>,
        config: BTreeMap<String, String>,
        markers: WorkspaceMarkers,
        source_files: Vec<RunnerSourceFile>,
        metadata: RunnerWorkspaceMetadata,
    ) -> Self {
        Self {
            root,
            code_paths,
            config,
            markers,
            source_files,
            metadata,
        }
    }

    pub fn new_without_metadata(
        root: Option<PathBuf>,
        code_paths: Vec<PathBuf>,
        config: BTreeMap<String, String>,
        markers: WorkspaceMarkers,
        source_files: Vec<RunnerSourceFile>,
    ) -> Self {
        Self {
            root,
            code_paths,
            config,
            markers,
            source_files,
            metadata: RunnerWorkspaceMetadata::default(),
        }
    }

    pub fn for_test(root: impl Into<PathBuf>) -> Self {
        Self {
            root: Some(root.into()),
            code_paths: Vec::new(),
            config: BTreeMap::new(),
            markers: WorkspaceMarkers::from_files(["Cargo.toml"]),
            source_files: Vec::new(),
            metadata: RunnerWorkspaceMetadata::default(),
        }
    }
}

/// Language-neutral test runner contract.
pub trait TestRunner: Send + Sync {
    fn id(&self) -> &'static str;

    fn detect(&self, markers: &WorkspaceMarkers) -> bool;

    fn source_extensions(&self) -> &'static [&'static str] {
        &[]
    }

    fn ignored_source_dirs(&self) -> &'static [&'static str] {
        &[]
    }

    fn build_test_command(
        &self,
        workspace: &RunnerWorkspace,
        selector: &TestSelector,
    ) -> SpecResult<TestCommand>;

    fn scan_legacy_bindings(
        &self,
        workspace: &RunnerWorkspace,
    ) -> SpecResult<std::collections::HashMap<String, String>>;

    fn preflight(
        &self,
        _workspace: &RunnerWorkspace,
        _selector: &TestSelector,
    ) -> SpecResult<PreflightOutcome> {
        Ok(PreflightOutcome::Ready)
    }

    fn recognized_config_keys(&self) -> &'static [&'static str] {
        &[]
    }

    fn requires_device(&self, _selector: &TestSelector) -> bool {
        false
    }

    fn supported_host_platforms(&self) -> &'static [HostPlatform] {
        &[HostPlatform::All]
    }
}
