use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn test_android_runner_passes_kotlin_unit_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let sdk = android_sdk_home()?;
    let json = run_lifecycle_fixture(
        "tests/fixtures/android-mini/spec.md",
        "tests/fixtures/android-mini",
        &sdk,
    )?;
    let result = scenario_result(&json, "android kotlin unit scenario");
    let stdout = result["evidence"][0]["stdout"].as_str().unwrap_or_default();

    assert_eq!(json["passed"], true);
    assert_eq!(result["verdict"], "pass");
    assert_eq!(result["evidence"][0]["command_program"], "./gradlew");
    assert_eq!(result["evidence"][0]["package"], ":app");
    assert_eq!(result["evidence"][0]["level"], "unit");
    assert!(
        stdout.contains("BUILD SUCCESSFUL") || stdout.contains("BUILD SUCCESS"),
        "missing Gradle success marker in stdout:\n{stdout}"
    );

    Ok(())
}

fn run_lifecycle_fixture(
    spec_path: &str,
    code_path: &str,
    sdk: &Path,
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
        .env("ANDROID_HOME", sdk)
        .env("ANDROID_SDK_ROOT", sdk)
        .output()?;

    assert!(
        output.status.success(),
        "lifecycle fixture failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(serde_json::from_slice(&output.stdout)?)
}

fn android_sdk_home() -> Result<PathBuf, Box<dyn std::error::Error>> {
    for key in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Some(path) = std::env::var_os(key).map(PathBuf::from)
            && has_required_android_sdk_files(&path)
        {
            return Ok(path);
        }
    }

    if let Some(path) = std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join("Library/Android/sdk"))
        && has_required_android_sdk_files(&path)
    {
        return Ok(path);
    }

    Err(
        "Android SDK with platform-tools/adb and platforms/android-36/android.jar was not found"
            .into(),
    )
}

fn has_required_android_sdk_files(path: &Path) -> bool {
    path.join("platform-tools/adb").is_file()
        && path.join("platforms/android-36/android.jar").is_file()
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
