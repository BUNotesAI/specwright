#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

fn temp_dir(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("specwright-external-{label}-{unique}"));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_spec(dir: &Path, body: &str) -> PathBuf {
    let path = dir.join("external.spec.md");
    fs::write(&path, body).unwrap();
    path
}

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_specwright"))
        .args(args)
        .output()
        .unwrap()
}

fn external_spec(evidence: &str) -> String {
    format!(
        r#"spec: task
name: "External fixture"
---

## Intent

Wait for an external build result.

## Completion Criteria

Scenario: External build
  Verification: external
  Evidence: {evidence}
  Given source code is submitted
  When remote CI finishes
  Then the external build result is recorded
"#
    )
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn current_commit() -> String {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

fn manifest(spec_text: &str, evidence: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "schema_version": 1,
        "spec": {
            "identity": "External fixture",
            "sha256": sha256_hex(spec_text.as_bytes())
        },
        "subject_commit": current_commit(),
        "generated_at": "2026-08-14T02:00:00Z",
        "evidence": evidence
    })
}

fn evidence_item(id: &str, scenario: &str, verdict: &str) -> serde_json::Value {
    serde_json::json!({
        "scenario_name": scenario,
        "evidence_id": id,
        "artifact_url": "https://ci.example.invalid/runs/42/artifact",
        "digest": {
            "algorithm": "sha256",
            "value": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        },
        "verdict": verdict,
        "producer": {
            "name": "example-ci",
            "run_id": "42"
        }
    })
}

#[test]
fn external_fields_parse_into_scenario_model() {
    let dir = temp_dir("parse");
    let spec = write_spec(&dir, &external_spec("ci-build"));
    let output = run(&["parse", spec.to_str().unwrap(), "--format", "json"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let scenario = &json["sections"][1]["scenarios"][0];
    assert_eq!(scenario["verification"], "external");
    assert_eq!(scenario["evidence"], "ci-build");
}

#[test]
fn external_lint_requires_unique_evidence_ids() {
    let dir = temp_dir("lint");
    let valid = write_spec(&dir, &external_spec("ci-build"));
    let valid_output = run(&["lint", valid.to_str().unwrap(), "--format", "json"]);
    assert!(
        valid_output.status.success(),
        "{}",
        String::from_utf8_lossy(&valid_output.stderr)
    );

    let missing_text = external_spec("").replace("  Evidence: \n", "");
    let missing = dir.join("missing.spec.md");
    fs::write(&missing, missing_text).unwrap();
    let missing_output = run(&["lint", missing.to_str().unwrap(), "--format", "json"]);
    assert!(!missing_output.status.success());
    assert!(String::from_utf8_lossy(&missing_output.stdout).contains("external-evidence"));

    let duplicate_text = external_spec("ci-build")
        .replace("Scenario: External build", "Scenario: External build one")
        + &external_spec("ci-build")
            .split("## Completion Criteria\n\n")
            .nth(1)
            .unwrap()
            .replace("Scenario: External build", "Scenario: External build two");
    let duplicate = dir.join("duplicate.spec.md");
    fs::write(&duplicate, duplicate_text).unwrap();
    let duplicate_output = run(&["lint", duplicate.to_str().unwrap(), "--format", "json"]);
    assert!(!duplicate_output.status.success());
    assert!(String::from_utf8_lossy(&duplicate_output.stdout).contains("external-evidence"));
}

#[test]
fn ordinary_unbound_scenario_remains_blocking_skip() {
    let dir = temp_dir("ordinary-skip");
    let spec = write_spec(
        &dir,
        r#"spec: task
name: "Ordinary fixture"
---

## Intent

Keep ordinary unbound behavior.

## Completion Criteria

Scenario: Ordinary unbound
  Given source code exists
  When verification runs
  Then the scenario is not verified
"#,
    );
    let output = run(&[
        "verify",
        spec.to_str().unwrap(),
        "--code",
        env!("CARGO_MANIFEST_DIR"),
        "--format",
        "json",
    ]);
    assert!(!output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["summary"]["skipped"], 1);
    assert_eq!(json["results"][0]["verdict"], "skip");
}

#[test]
fn strict_external_pending_is_not_passing() {
    let dir = temp_dir("strict");
    let spec = write_spec(&dir, &external_spec("ci-build"));
    let output = run(&[
        "verify",
        spec.to_str().unwrap(),
        "--code",
        env!("CARGO_MANIFEST_DIR"),
        "--format",
        "json",
    ]);
    assert!(!output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["summary"]["external_pending"], 1);
    assert_eq!(json["results"][0]["verdict"], "external_pending");
}

#[test]
fn allow_pending_preserves_external_pending_json() {
    let dir = temp_dir("allow");
    let spec = write_spec(&dir, &external_spec("ci-build"));
    let output = run(&[
        "verify",
        spec.to_str().unwrap(),
        "--code",
        env!("CARGO_MANIFEST_DIR"),
        "--format",
        "json",
        "--external-mode",
        "allow-pending",
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["summary"]["external_pending"], 1);
    assert_eq!(json["results"][0]["verdict"], "external_pending");
}

#[test]
fn external_verification_json_contract_is_versioned() {
    let dir = temp_dir("json-contract");
    let spec = write_spec(&dir, &external_spec("ci-build"));
    for command in ["verify", "lifecycle"] {
        let output = run(&[
            command,
            spec.to_str().unwrap(),
            "--code",
            env!("CARGO_MANIFEST_DIR"),
            "--format",
            "json",
            "--external-mode",
            "allow-pending",
        ]);
        assert!(
            output.status.success(),
            "{command}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        let verification = if command == "lifecycle" {
            &json["verification"]
        } else {
            &json
        };
        assert_eq!(verification["schema_version"], 1);
        assert_eq!(verification["summary"]["external_pending"], 1);
        assert_eq!(verification["results"][0]["verdict"], "external_pending");
    }
}

#[test]
fn resolve_evidence_accepts_complete_manifest() {
    let dir = temp_dir("resolve-complete");
    let spec_text = external_spec("ci-build");
    let spec = write_spec(&dir, &spec_text);
    let manifest_path = dir.join("evidence.json");
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest(
            &spec_text,
            serde_json::json!([evidence_item("ci-build", "External build", "pass")]),
        ))
        .unwrap(),
    )
    .unwrap();

    let output = run(&[
        "resolve-evidence",
        spec.to_str().unwrap(),
        "--code",
        env!("CARGO_MANIFEST_DIR"),
        "--manifest",
        manifest_path.to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["stage"], "resolve-evidence");
    assert_eq!(json["passed"], true);
    assert_eq!(json["verification"]["schema_version"], 1);
    assert_eq!(json["verification"]["summary"]["external_pending"], 0);
    assert_eq!(json["verification"]["results"][0]["verdict"], "pass");
}

#[test]
fn resolve_evidence_rejects_unknown_duplicate_and_missing_ids() {
    let dir = temp_dir("resolve-invalid");
    let spec_text = external_spec("ci-build");
    let spec = write_spec(&dir, &spec_text);

    for (label, items, expected) in [
        ("missing", serde_json::json!([]), "missing Evidence ID"),
        (
            "unknown",
            serde_json::json!([evidence_item("other", "External build", "pass")]),
            "unknown Evidence ID",
        ),
        (
            "duplicate",
            serde_json::json!([
                evidence_item("ci-build", "External build", "pass"),
                evidence_item("ci-build", "External build", "pass")
            ]),
            "duplicate Evidence ID",
        ),
    ] {
        let path = dir.join(format!("{label}.json"));
        fs::write(
            &path,
            serde_json::to_vec_pretty(&manifest(&spec_text, items)).unwrap(),
        )
        .unwrap();
        let output = run(&[
            "resolve-evidence",
            spec.to_str().unwrap(),
            "--code",
            env!("CARGO_MANIFEST_DIR"),
            "--manifest",
            path.to_str().unwrap(),
        ]);
        assert!(!output.status.success(), "{label} unexpectedly passed");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected),
            "{label}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn resolve_evidence_never_overwrites_mechanical_failure() {
    let dir = temp_dir("mechanical-fail");
    let spec_text = r#"spec: task
name: "External fixture"
---

## Intent

Keep a mechanical failure authoritative.

## Completion Criteria

Scenario: Mechanical scenario
  Test: definitely_missing_test_selector
  Given source code exists
  When the mechanical test cannot run
  Then evidence cannot replace its result
"#;
    let spec = write_spec(&dir, spec_text);
    let manifest_path = dir.join("evidence.json");
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest(
            spec_text,
            serde_json::json!([evidence_item("ci-build", "Mechanical scenario", "pass")]),
        ))
        .unwrap(),
    )
    .unwrap();
    let output = run(&[
        "resolve-evidence",
        spec.to_str().unwrap(),
        "--code",
        env!("CARGO_MANIFEST_DIR"),
        "--manifest",
        manifest_path.to_str().unwrap(),
    ]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown Evidence ID"));
}
