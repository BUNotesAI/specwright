#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Release smoke (task_792a6b6c): the built binary reports the crate version.
//! Uses CARGO_PKG_VERSION so it tracks future bumps without editing the test.

use std::process::Command;

#[test]
fn agent_spec_version_reports_bumped_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_agent-spec"))
        .arg("--version")
        .output()
        .expect("failed to run agent-spec --version");
    assert!(
        output.status.success(),
        "agent-spec --version should exit 0"
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "--version output {stdout:?} should contain crate version {}",
        env!("CARGO_PKG_VERSION")
    );
}
