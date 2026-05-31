mod android;
mod cargo;
mod gradle;
mod ios;
mod jvm;
mod maven;
mod model;
mod node;
mod registry;

pub use android::AndroidRunner;
pub use cargo::CargoRunner;
#[cfg(test)]
pub use cargo::extract_bindings;
pub use gradle::GradleRunner;
pub use ios::IosRunner;
pub use maven::MavenRunner;
pub use model::{
    HostPlatform, NodePackageManager, NodePackageManagerDecision, NodePackageManagerSource,
    NodeProjectMetadata, PreflightOutcome, ResolutionSource, RunnerResolution, RunnerSelection,
    RunnerSourceFile, RunnerWarning, RunnerWorkspace, RunnerWorkspaceMetadata, TestCommand,
    TestRunner, WorkspaceMarkers,
};
pub use node::NodeRunner;
pub use registry::{RunnerRegistry, resolve_detected_runner, resolve_runner_choice};
