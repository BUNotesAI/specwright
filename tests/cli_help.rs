//! Runtime contracts for agent-facing CLI help.

use std::error::Error;
use std::process::Command;

type TestResult = Result<(), Box<dyn Error>>;

fn help(args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_specwright"))
        .args(args)
        .arg("--help")
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "specwright {} --help failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn assert_contains_all(help: &str, required: &[&str]) {
    for text in required {
        assert!(help.contains(text), "help should contain `{text}`:\n{help}");
    }
}

#[test]
fn top_level_help_lists_current_agent_workflows() -> TestResult {
    let output = help(&[])?;
    assert_contains_all(
        &output,
        &["resolve-ai", "resolve-evidence", "plan", "graph"],
    );
    Ok(())
}

#[test]
fn verify_help_explains_external_policy_ai_modes_and_ctest_prerequisites() -> TestResult {
    let output = help(&["verify"])?;
    assert_contains_all(
        &output,
        &[
            "off, stub, caller",
            "Verification: external",
            "Evidence: <id>",
            "external_pending",
            "allow-pending",
            "intermediate",
            "runner: ctest",
            "runner_config: { build_dir: \"build\" }",
            "never configures",
            "builds the project",
        ],
    );
    Ok(())
}

#[test]
fn lifecycle_help_explains_external_policy_ai_modes_and_ctest_prerequisites() -> TestResult {
    let output = help(&["lifecycle"])?;
    assert_contains_all(
        &output,
        &[
            "off, stub, caller",
            "Verification: external",
            "Evidence: <id>",
            "strict",
            "allow-pending",
            "runner: ctest",
            "preconfigured and built",
            "never configures",
            "builds the project",
        ],
    );
    Ok(())
}

#[test]
fn resolve_evidence_help_explains_manifest_and_atomic_resolution_contract() -> TestResult {
    let output = help(&["resolve-evidence"])?;
    assert_contains_all(
        &output,
        &[
            "Verification: external",
            "Evidence: <id>",
            "external_pending",
            "complete versioned manifest",
            "spec identity and SHA-256",
            "subject commit",
            "artifact URL",
            "producer or attestation",
            "Unknown, duplicate, or missing Evidence IDs",
            "never overwrites",
            "mechanical result",
        ],
    );
    Ok(())
}
