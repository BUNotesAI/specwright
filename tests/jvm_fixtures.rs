use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_maven_runner_passes_java_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let json = run_lifecycle_fixture(
        "tests/fixtures/maven-mini/spec.md",
        "tests/fixtures/maven-mini",
    )?;
    let result = scenario_result(&json, "maven java scenario");

    assert_eq!(json["passed"], true);
    assert_eq!(result["verdict"], "pass");
    assert_eq!(result["evidence"][0]["command_program"], "./mvnw");
    assert!(
        result["evidence"][0]["stdout"]
            .as_str()
            .unwrap_or_default()
            .contains("BUILD SUCCESS")
    );

    Ok(())
}

#[test]
fn test_gradle_runner_passes_kotlin_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let json = run_lifecycle_fixture(
        "tests/fixtures/gradle-kotlin-mini/spec.md",
        "tests/fixtures/gradle-kotlin-mini",
    )?;
    let result = scenario_result(&json, "gradle kotlin scenario");

    assert_eq!(json["passed"], true);
    assert_eq!(result["verdict"], "pass");
    assert_eq!(result["evidence"][0]["command_program"], "./gradlew");
    assert!(
        result["evidence"][0]["stdout"]
            .as_str()
            .unwrap_or_default()
            .contains("BUILD SUCCESS")
    );

    Ok(())
}

fn run_lifecycle_fixture(
    spec_path: &str,
    code_path: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output = Command::new(env!("CARGO_BIN_EXE_specwright"))
        .args([
            "lifecycle",
            spec_path,
            "--code",
            code_path,
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
        "lifecycle fixture failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(serde_json::from_slice(&output.stdout)?)
}

fn scenario_result<'a>(json: &'a serde_json::Value, scenario_name: &str) -> &'a serde_json::Value {
    let Some(results) = json["verification"]["results"].as_array() else {
        panic!("missing verification results array")
    };
    results
        .iter()
        .find(|result| result["scenario_name"] == scenario_name)
        .unwrap_or_else(|| panic!("missing scenario result `{scenario_name}`"))
}
