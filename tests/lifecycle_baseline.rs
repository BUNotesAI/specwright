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
