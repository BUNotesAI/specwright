use std::path::{Path, PathBuf};
use std::process::{Command, Output};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn ctest_fixture_lifecycle_passes() -> TestResult {
    let Some(fixture) = prepare_native_fixture()? else {
        return Ok(());
    };
    let output = run_lifecycle(&fixture.root.join("pass.spec.md"), &fixture.root, None)?;
    assert_success(&output, "passing CTest lifecycle")?;

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let result = scenario_result(&json, "native CTest pass scenario");
    let evidence = &result["evidence"][0];
    let stdout = evidence["stdout"].as_str().unwrap_or_default();

    assert_eq!(json["passed"], true);
    assert_eq!(result["verdict"], "pass");
    assert_eq!(evidence["command_program"], "ctest");
    assert!(
        stdout.contains("ctest_pass"),
        "missing registered test name: {stdout}"
    );
    assert!(
        stdout.contains("100% tests passed"),
        "missing CTest pass summary: {stdout}"
    );
    assert!(fixture.build_dir.join("native-pass.sentinel").is_file());

    Ok(())
}

#[test]
fn ctest_fixture_scenario_failure_matrix() -> TestResult {
    let Some(fixture) = prepare_native_fixture()? else {
        return Ok(());
    };

    for (spec, scenario, expected_output) in [
        (
            "zero-match.spec.md",
            "CTest zero match scenario",
            "No tests were found",
        ),
        (
            "fail.spec.md",
            "native CTest failure scenario",
            "CTEST_FAIL_BODY",
        ),
        (
            "missing-executable.spec.md",
            "missing registered executable scenario",
            "Unable to find executable",
        ),
    ] {
        let output = run_lifecycle(&fixture.root.join(spec), &fixture.root, None)?;
        assert!(!output.status.success(), "{spec} unexpectedly passed");
        let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        let result = scenario_result(&json, scenario);
        let stdout = result["evidence"][0]["stdout"].as_str().unwrap_or_default();

        assert_eq!(json["passed"], false);
        assert_eq!(result["verdict"], "fail");
        assert_eq!(result["evidence"][0]["passed"], false);
        assert!(
            stdout.contains(expected_output),
            "{spec} missing `{expected_output}` in CTest evidence:\n{stdout}"
        );
    }

    Ok(())
}

#[test]
fn ctest_prerequisite_error_matrix() -> TestResult {
    let Some(fixture) = prepare_native_fixture()? else {
        return Ok(());
    };

    let empty_path = unique_temp_path("ctest-empty-path");
    std::fs::create_dir_all(&empty_path)?;
    let missing_ctest = run_lifecycle(
        &fixture.root.join("pass.spec.md"),
        &fixture.root,
        Some(&empty_path),
    )?;
    assert_run_wide_error(&missing_ctest, "failed to run ctest test command")?;

    let unprepared = copy_fixture("native-project", "ctest-missing-build-tree")?;
    let missing_build = run_lifecycle(&unprepared.join("pass.spec.md"), &unprepared, None)?;
    assert_run_wide_error(&missing_build, "failed to run ctest test command")?;

    Ok(())
}

#[test]
fn ctest_unconfigured_tree_is_scenario_failure() -> TestResult {
    if !fixture_tools_available() {
        return Ok(());
    }

    let root = copy_fixture("native-project", "ctest-unconfigured-tree")?;
    std::fs::create_dir_all(root.join("build"))?;
    let output = run_lifecycle(&root.join("pass.spec.md"), &root, None)?;
    assert!(!output.status.success());

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let result = scenario_result(&json, "native CTest pass scenario");
    assert_eq!(result["verdict"], "fail");
    assert_ne!(result["verdict"], "skip");

    Ok(())
}

#[test]
fn ctest_mixed_route_preserves_default_slot() -> TestResult {
    if !fixture_tools_available() || !has_program("cargo") {
        eprintln!("skipping mixed CTest fixture: cmake, ctest, and cargo are required");
        return Ok(());
    }

    let root = copy_fixture("mixed-project", "ctest-mixed-project")?;
    configure_and_build(&root.join("native"), &root.join("native/build"))?;
    let output = run_lifecycle(&root.join("spec.md"), &root, None)?;
    assert_success(&output, "mixed Cargo and CTest lifecycle")?;

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let cargo = scenario_result(&json, "mixed Cargo default scenario");
    let ctest = scenario_result(&json, "mixed routed CTest scenario");
    assert_eq!(json["passed"], true);
    assert_eq!(cargo["verdict"], "pass");
    assert!(cargo["evidence"][0].get("command_program").is_none());
    assert_eq!(ctest["verdict"], "pass");
    assert_eq!(ctest["evidence"][0]["command_program"], "ctest");
    assert_eq!(ctest["evidence"][0]["package"], "cppslot");

    Ok(())
}

#[test]
fn ctest_markerless_consumer_fixture_executes_registered_host_script() -> TestResult {
    if !fixture_tools_available() {
        return Ok(());
    }

    let root = copy_fixture("markerless-consumer", "ctest-markerless-consumer")?;
    let build_dir = root.join(".specwright/ctest-build");
    configure(&root.join("tests/specwright-ctest"), &build_dir)?;
    let output = run_lifecycle(&root.join("spec.md"), &root, None)?;
    assert_success(&output, "markerless host-script bridge")?;

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let result = scenario_result(&json, "markerless host script scenario");
    let stdout = result["evidence"][0]["stdout"].as_str().unwrap_or_default();
    assert_eq!(result["verdict"], "pass");
    assert_eq!(result["evidence"][0]["command_program"], "ctest");
    assert!(stdout.contains("markerless_host_test"));
    assert!(build_dir.join("markerless-host.sentinel").is_file());

    Ok(())
}

struct PreparedFixture {
    root: PathBuf,
    build_dir: PathBuf,
}

fn prepare_native_fixture() -> Result<Option<PreparedFixture>, Box<dyn std::error::Error>> {
    if !fixture_tools_available() {
        return Ok(None);
    }

    let root = copy_fixture("native-project", "ctest-native-project")?;
    let build_dir = root.join("build");
    configure_and_build(&root, &build_dir)?;
    Ok(Some(PreparedFixture { root, build_dir }))
}

fn configure_and_build(source: &Path, build_dir: &Path) -> TestResult {
    configure(source, build_dir)?;
    let output = Command::new("cmake")
        .args([
            "--build",
            build_dir.to_str().ok_or("non-utf8 build directory")?,
        ])
        .output()?;
    assert_success(&output, "CMake fixture build")
}

fn configure(source: &Path, build_dir: &Path) -> TestResult {
    let output = Command::new("cmake")
        .args([
            "-S",
            source.to_str().ok_or("non-utf8 source directory")?,
            "-B",
            build_dir.to_str().ok_or("non-utf8 build directory")?,
        ])
        .output()?;
    assert_success(&output, "CMake fixture configure")
}

fn run_lifecycle(
    spec: &Path,
    code: &Path,
    path: Option<&Path>,
) -> Result<Output, Box<dyn std::error::Error>> {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut command = Command::new(env!("CARGO_BIN_EXE_specwright"));
    command
        .args([
            "lifecycle",
            spec.to_str().ok_or("non-utf8 spec path")?,
            "--code",
            code.to_str().ok_or("non-utf8 code path")?,
            "--format",
            "json",
            "--change-scope",
            "none",
            "--layers",
            "test",
        ])
        .current_dir(repo);
    if let Some(path) = path {
        command.env("PATH", path);
    }
    Ok(command.output()?)
}

fn assert_success(output: &Output, label: &str) -> TestResult {
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "{label} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
    .into())
}

fn assert_run_wide_error(output: &Output, expected: &str) -> TestResult {
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(expected),
        "missing `{expected}` in stderr:\n{stderr}"
    );
    assert!(!stdout.contains("\"passed\": true"));
    assert!(!stdout.contains("\"verdict\": \"skip\""));
    Ok(())
}

fn copy_fixture(name: &str, label: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = repo.join("tests/fixtures/ctest-mini").join(name);
    let target = unique_temp_path(label);
    copy_dir(&source, &target)?;
    Ok(target)
}

fn copy_dir(source: &Path, target: &Path) -> TestResult {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir(&source_path, &target_path)?;
        } else {
            std::fs::copy(source_path, target_path)?;
        }
    }
    Ok(())
}

fn unique_temp_path(label: &str) -> PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    std::env::temp_dir().join(format!("specwright-{label}-{unique}"))
}

fn fixture_tools_available() -> bool {
    let available = has_program("cmake") && has_program("ctest");
    if !available {
        eprintln!("skipping CTest fixture: cmake and ctest 3.17 or newer are required");
    }
    available
}

fn has_program(program: &str) -> bool {
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&paths).any(|dir| dir.join(program).is_file())
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
