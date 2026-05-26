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
}

/// Runner-facing workspace model. It contains data, not hidden IO handles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunnerWorkspace {
    pub root: Option<PathBuf>,
    pub code_paths: Vec<PathBuf>,
    pub config: BTreeMap<String, String>,
    pub markers: WorkspaceMarkers,
    pub source_files: Vec<RunnerSourceFile>,
}

impl RunnerWorkspace {
    pub fn new(
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
        }
    }

    pub fn for_test(root: impl Into<PathBuf>) -> Self {
        Self {
            root: Some(root.into()),
            code_paths: Vec::new(),
            config: BTreeMap::new(),
            markers: WorkspaceMarkers::from_files(["Cargo.toml"]),
            source_files: Vec::new(),
        }
    }
}

/// Language-neutral test runner contract.
pub trait TestRunner: Send + Sync {
    fn id(&self) -> &'static str;

    fn detect(&self, markers: &WorkspaceMarkers) -> bool;

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
