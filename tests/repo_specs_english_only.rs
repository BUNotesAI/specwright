#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Real-path guard for the CJK keyword migration (task_792a6b6c).
//!
//! Every `.spec` / `.spec.md` under `specs/` and `examples/` must parse under
//! the (English-only) parser. After the migration this proves the migration is
//! complete: any leftover Chinese structural keyword would fail to parse (the
//! parser hard-rejects CJK keywords), so a green run means no structural
//! keyword was missed. Descriptions may stay Chinese.

use std::path::{Path, PathBuf};
use std::process::Command;

fn collect_specs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_specs(&path, out);
        } else if let Some(name) = path.file_name().and_then(|n| n.to_str())
            && (name.ends_with(".spec") || name.ends_with(".spec.md"))
        {
            out.push(path);
        }
    }
}

#[test]
fn test_repo_specs_parse_under_english_keywords() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut specs = Vec::new();
    collect_specs(&repo.join("specs"), &mut specs);
    collect_specs(&repo.join("examples"), &mut specs);
    specs.sort();

    assert!(
        specs.len() >= 49,
        "expected the repository spec corpus (>= 49 files), found {}",
        specs.len()
    );

    let mut failures = Vec::new();
    for spec in &specs {
        let rel = spec.strip_prefix(&repo).unwrap_or(spec);
        let rel_str = rel.to_str().unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_specwright"))
            .args(["contract", rel_str, "--format", "json"])
            .current_dir(&repo)
            .output()
            .expect("failed to run specwright contract");
        if !output.status.success() {
            failures.push(format!(
                "{}: {}",
                rel.display(),
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{} spec(s) failed to parse under English-only keywords:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
