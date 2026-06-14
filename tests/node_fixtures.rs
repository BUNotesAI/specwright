use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn test_node_package_json_workspace_detects_node_and_executes_bound_test()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(json) = run_node_lifecycle_fixture()? else {
        return Ok(());
    };
    let result = scenario_result(&json, "node npm unit scenario");

    assert_eq!(json["passed"], true);
    assert_eq!(result["verdict"], "pass");
    assert_eq!(result["evidence"][0]["command_program"], "npm");
    assert!(
        result["evidence"][0]["stdout"]
            .as_str()
            .unwrap_or_default()
            .contains("NODE_UNIT_OK:-t|renders dashboard")
    );

    Ok(())
}

#[test]
fn test_node_fixture_lifecycle_runs_install_free_npm_scripts()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(json) = run_node_lifecycle_fixture()? else {
        return Ok(());
    };

    assert_eq!(json["passed"], true);
    for scenario in [
        ("node npm unit scenario", "NODE_UNIT_OK"),
        ("node npm typecheck scenario", "NODE_TYPECHECK_OK"),
        ("node npm build scenario", "NODE_BUILD_OK"),
    ] {
        let result = scenario_result(&json, scenario.0);
        assert_eq!(result["verdict"], "pass");
        assert_eq!(result["evidence"][0]["command_program"], "npm");
        assert!(
            result["evidence"][0]["stdout"]
                .as_str()
                .unwrap_or_default()
                .contains(scenario.1)
        );
    }

    Ok(())
}

#[test]
fn test_node_build_level_runs_build_script() -> Result<(), Box<dyn std::error::Error>> {
    let Some(json) = run_node_lifecycle_fixture()? else {
        return Ok(());
    };
    let result = scenario_result(&json, "node npm build scenario");

    assert_eq!(result["verdict"], "pass");
    assert_eq!(result["evidence"][0]["level"], "build");
    assert!(
        result["evidence"][0]["stdout"]
            .as_str()
            .unwrap_or_default()
            .contains("NODE_BUILD_OK")
    );

    Ok(())
}

fn run_node_lifecycle_fixture() -> Result<Option<serde_json::Value>, Box<dyn std::error::Error>> {
    if !has_program("node") || !has_program("npm") {
        eprintln!("skipping Node fixture lifecycle: host node and npm are required");
        return Ok(None);
    }

    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture = copy_fixture_to_temp(&repo.join("tests/fixtures/node-npm-mini"))?;
    let output = Command::new(env!("CARGO_BIN_EXE_specwright"))
        .args([
            "lifecycle",
            fixture
                .join("spec.md")
                .to_str()
                .ok_or("non-utf8 spec path")?,
            "--code",
            fixture.to_str().ok_or("non-utf8 fixture path")?,
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

    Ok(Some(serde_json::from_slice(&output.stdout)?))
}

fn copy_fixture_to_temp(source: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let target = std::env::temp_dir().join(format!("agent-spec-node-npm-mini-{unique}"));
    copy_dir(source, &target)?;
    Ok(target)
}

fn copy_dir(source: &Path, target: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir(&source_path, &target_path)?;
        } else {
            std::fs::copy(&source_path, &target_path)?;
        }
    }
    Ok(())
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
