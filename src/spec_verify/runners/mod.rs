mod cargo;
mod model;
mod registry;

pub use cargo::CargoRunner;
#[cfg(test)]
pub use cargo::extract_bindings;
pub use model::{
    HostPlatform, PreflightOutcome, ResolutionSource, RunnerResolution, RunnerSelection,
    RunnerSourceFile, RunnerWarning, RunnerWorkspace, TestCommand, TestRunner, WorkspaceMarkers,
};
pub use registry::{RunnerRegistry, resolve_detected_runner, resolve_runner_choice};
