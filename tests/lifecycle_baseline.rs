use std::path::PathBuf;
use std::process::Command;

#[test]
fn cargo_lifecycle_default_matches_pre_refactor_baseline() -> Result<(), Box<dyn std::error::Error>>
{
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = Command::new(env!("CARGO_BIN_EXE_agent-spec"))
        .args([
            "lifecycle",
            "tests/fixtures/cargo-lifecycle-baseline.spec.md",
            "--code",
            ".",
            "--format",
            "json",
            "--change-scope",
            "none",
            "--layers",
            "test",
        ])
        .current_dir(&repo)
        .output()?;

    assert!(
        !output.status.success(),
        "zero-scenario baseline intentionally preserves the pre-refactor non-passing lifecycle status"
    );

    let actual = String::from_utf8(output.stdout)?;
    assert_eq!(
        actual.trim_end(),
        include_str!("fixtures/cargo-lifecycle-baseline.json").trim_end()
    );

    let json: serde_json::Value = serde_json::from_str(&actual)?;
    assert!(
        json.get("runner").is_none(),
        "default Cargo resolution must not add a runner trace field"
    );
    assert!(
        json.get("config_warnings").is_none(),
        "empty Cargo runner_config must not add config_warnings"
    );

    Ok(())
}

#[test]
fn bound_cargo_lifecycle_omits_command_program() -> Result<(), Box<dyn std::error::Error>> {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = Command::new(env!("CARGO_BIN_EXE_agent-spec"))
        .args([
            "lifecycle",
            "tests/fixtures/cargo-lifecycle-command-program.spec.md",
            "--code",
            ".",
            "--format",
            "json",
            "--change-scope",
            "none",
            "--layers",
            "test",
        ])
        .current_dir(&repo)
        .output()?;

    assert!(
        output.status.success(),
        "bound Cargo lifecycle baseline should pass"
    );

    let actual = String::from_utf8(output.stdout)?;
    let json: serde_json::Value = serde_json::from_str(&actual)?;
    assert_eq!(json["passed"], true);
    assert!(
        json.get("runner").is_none(),
        "default Cargo resolution must not add a runner trace field"
    );
    assert!(
        json.get("config_warnings").is_none(),
        "empty Cargo runner_config must not add config_warnings"
    );

    let results = json["verification"]["results"]
        .as_array()
        .ok_or("verification results should be an array")?;
    assert_eq!(results.len(), 1);
    let evidence = results[0]["evidence"]
        .as_array()
        .ok_or("scenario evidence should be an array")?;
    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0]["type"], "test_output");
    assert_eq!(evidence[0]["passed"], true);
    assert_eq!(
        evidence[0]["test_name"],
        "agent-spec::test_cargo_command_program_evidence_is_omitted_for_json_compatibility"
    );
    assert!(
        evidence[0].get("command_program").is_none(),
        "default Cargo TestOutput must omit command_program"
    );

    Ok(())
}
