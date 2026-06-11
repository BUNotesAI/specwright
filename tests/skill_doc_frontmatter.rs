#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Regression guard for the recurring agent-spec front-matter format bug.
//!
//! agent-spec front-matter has NO opening `---`: it starts on the first line of
//! the block and ends at a single `---`. A `---` line immediately followed by
//! `spec: project|task|org` makes `agent-spec parse` fail with
//! `invalid front-matter: missing 'spec:' field`. Skill authors keep
//! reintroducing it because the standard Markdown/YAML convention wraps
//! front-matter in `---...---`. These tests scan the bundled skill docs so the
//! source is guarded at CI time instead of by an external hook.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Recursively collect every Markdown file shipped under `skills/`.
fn skill_markdown_files() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("skills")];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "md") {
                out.push(path);
            }
        }
    }
    out
}

/// Contents of each fenced code block (text between a pair of ``` fences).
fn fenced_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            match current.take() {
                Some(buf) => blocks.push(buf),
                None => current = Some(String::new()),
            }
        } else if let Some(buf) = current.as_mut() {
            buf.push_str(line);
            buf.push('\n');
        }
    }
    blocks
}

fn opens_front_matter(line: &str) -> bool {
    line.starts_with("spec: project")
        || line.starts_with("spec: task")
        || line.starts_with("spec: org")
}

/// The exact recurring bug: a `---` line directly above the `spec:` key.
#[test]
fn skill_docs_have_no_leading_dash_frontmatter() {
    let files = skill_markdown_files();
    assert!(
        !files.is_empty(),
        "no skill markdown files found under skills/"
    );

    let mut violations = Vec::new();
    for file in &files {
        let text = fs::read_to_string(file).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        for i in 1..lines.len() {
            if lines[i - 1].trim_end() == "---" && opens_front_matter(lines[i]) {
                // i is 0-based; the offending `---` is line i (1-based).
                violations.push(format!("{}:{}", file.display(), i));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "agent-spec front-matter must start on the first line with no opening '---'.\n\
         Delete the leading '---' (the line number below points at it):\n  {}",
        violations.join("\n  ")
    );
}

/// Every front-matter-bearing spec example in the skill docs must actually parse.
#[test]
fn skill_doc_spec_examples_parse() {
    let bin = env!("CARGO_BIN_EXE_agent-spec");
    let dir = std::env::temp_dir();
    let mut failures = Vec::new();
    let mut checked = 0usize;

    for file in skill_markdown_files() {
        let text = fs::read_to_string(&file).unwrap();
        for block in fenced_blocks(&text) {
            // Only blocks that present full front-matter (i.e. start with `spec:`).
            if !block.trim_start().starts_with("spec:") {
                continue;
            }
            let path = dir.join(format!("agent_spec_skill_example_{checked}.spec.md"));
            checked += 1;
            fs::write(&path, &block).unwrap();
            let parsed = Command::new(bin)
                .args(["parse", path.to_str().unwrap()])
                .output()
                .expect("run agent-spec parse");
            if !parsed.status.success() {
                failures.push(format!(
                    "{}: {}",
                    file.display(),
                    String::from_utf8_lossy(&parsed.stderr).trim()
                ));
            }
            let _ = fs::remove_file(&path);
        }
    }

    assert!(checked > 0, "found no spec examples to parse");
    assert!(
        failures.is_empty(),
        "spec examples in skill docs failed to parse:\n  {}",
        failures.join("\n  ")
    );
}
