mod android;
mod cargo;
mod gradle;
mod jvm;
mod maven;
mod model;
mod registry;

pub use android::AndroidRunner;
pub use cargo::CargoRunner;
#[cfg(test)]
pub use cargo::extract_bindings;
pub use gradle::GradleRunner;
pub use maven::MavenRunner;
pub use model::{
    HostPlatform, PreflightOutcome, ResolutionSource, RunnerResolution, RunnerSelection,
    RunnerSourceFile, RunnerWarning, RunnerWorkspace, TestCommand, TestRunner, WorkspaceMarkers,
};
pub use registry::{RunnerRegistry, resolve_detected_runner, resolve_runner_choice};
