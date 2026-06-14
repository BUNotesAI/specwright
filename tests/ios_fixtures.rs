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

    /// The XCTest fixture needs a booted simulator. Hosts without one must still
    /// pin the runner's graceful-skip semantics instead of going red, so the test
    /// branches on the same signal the iOS runner itself probes (`xcrun simctl`).
    #[test]
    fn test_ios_runner_passes_xctest_fixture() -> Result<(), Box<dyn std::error::Error>> {
        let (lifecycle_passed, json) =
            run_lifecycle_fixture("tests/fixtures/ios-mini/spec.md", "tests/fixtures/ios-mini")?;
        let result = scenario_result(&json, "ios xctest scenario");

        if booted_ios_simulator_available() {
            let stdout = result["evidence"][0]["stdout"].as_str().unwrap_or_default();
            assert!(
                lifecycle_passed,
                "lifecycle must pass with a booted simulator"
            );
            assert_eq!(json["passed"], true);
            assert_eq!(result["verdict"], "pass");
            assert_eq!(result["evidence"][0]["command_program"], "xcodebuild");
            assert_eq!(result["evidence"][0]["package"], "IosMiniTests");
            assert!(
                stdout.contains("TEST SUCCEEDED") || stdout.contains("Test Suite"),
                "missing XCTest success marker in stdout:\n{stdout}"
            );
        } else {
            eprintln!(
                "no booted iOS simulator; pinning the runner's graceful-skip semantics \
                 instead of the pass path"
            );
            assert!(
                !lifecycle_passed,
                "lifecycle must not pass when its only scenario is skipped"
            );
            assert_eq!(json["passed"], false);
            assert_eq!(result["verdict"], "skip");
            assert_eq!(json["verification"]["summary"]["failed"], 0);
            assert_eq!(json["verification"]["summary"]["skipped"], 1);
            let reason = result["step_results"][0]["reason"]
                .as_str()
                .unwrap_or_default();
            assert!(
                reason.contains("ios-simulator") && reason.contains("booted"),
                "skip must carry the ios-simulator preflight reason, got: {reason}"
            );
        }

        Ok(())
    }

    /// Same availability signal the iOS runner probes before executing the
    /// fixture; a missing `xcrun` counts as no simulator, not as an error.
    fn booted_ios_simulator_available() -> bool {
        Command::new("xcrun")
            .args(["simctl", "list", "devices", "booted"])
            .output()
            .map(|output| {
                output.status.success()
                    && String::from_utf8_lossy(&output.stdout).contains("(Booted)")
            })
            .unwrap_or(false)
    }

    fn run_lifecycle_fixture(
        spec_path: &str,
        code_path: &str,
    ) -> Result<(bool, serde_json::Value), Box<dyn std::error::Error>> {
        let repo = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
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

        let json = serde_json::from_slice(&output.stdout).map_err(|err| {
            format!(
                "lifecycle emitted unparseable stdout ({err})\nstdout:\n{}\nstderr:\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )
        })?;
        Ok((output.status.success(), json))
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
