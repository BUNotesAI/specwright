use std::fs;

#[test]
fn test_ios_fixture_compile_gated_to_macos() -> Result<(), Box<dyn std::error::Error>> {
    let source = fs::read_to_string("tests/ios_fixtures.rs")?;
    assert!(
        source.contains("#[cfg(target_os = \"macos\")]"),
        "iOS fixture execution must stay compile-gated to macOS"
    );
    Ok(())
}

#[cfg(target_os = "macos")]
mod macos {
    use std::process::Command;

    #[test]
    fn test_ios_runner_passes_xctest_fixture() -> Result<(), Box<dyn std::error::Error>> {
        let json =
            run_lifecycle_fixture("tests/fixtures/ios-mini/spec.md", "tests/fixtures/ios-mini")?;
        let result = scenario_result(&json, "ios xctest scenario");
        let stdout = result["evidence"][0]["stdout"].as_str().unwrap_or_default();

        assert_eq!(json["passed"], true);
        assert_eq!(result["verdict"], "pass");
        assert_eq!(result["evidence"][0]["command_program"], "xcodebuild");
        assert_eq!(result["evidence"][0]["package"], "IosMiniTests");
        assert!(
            stdout.contains("TEST SUCCEEDED") || stdout.contains("Test Suite"),
            "missing XCTest success marker in stdout:\n{stdout}"
        );

        Ok(())
    }

    fn run_lifecycle_fixture(
        spec_path: &str,
        code_path: &str,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let repo = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let output = Command::new(env!("CARGO_BIN_EXE_agent-spec"))
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

    fn scenario_result<'a>(
        json: &'a serde_json::Value,
        scenario_name: &str,
    ) -> &'a serde_json::Value {
        let Some(results) = json["verification"]["results"].as_array() else {
            panic!("missing verification results array")
        };
        results
            .iter()
            .find(|result| result["scenario_name"] == scenario_name)
            .unwrap_or_else(|| panic!("missing scenario result `{scenario_name}`"))
    }
}
