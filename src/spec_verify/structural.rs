use std::fs;
use std::path::Path;

use crate::spec_core::{
    BoundaryCategory, Constraint, ConstraintCategory, Evidence, ScenarioResult, SpecResult,
    StepVerdict, Verdict,
};

use super::{VerificationContext, Verifier};

/// Structural verifier: checks code against constraints using pattern matching.
///
/// This is the cheapest verification tier — no tests run, no AI calls.
/// It matches constraint text patterns against source code.
pub struct StructuralVerifier;

impl Verifier for StructuralVerifier {
    fn name(&self) -> &str {
        "structural"
    }

    fn verify(&self, ctx: &VerificationContext) -> SpecResult<Vec<ScenarioResult>> {
        let mut results = Vec::new();

        // Collect all constraints (inherited + task's own)
        let mut all_constraints = ctx.resolved_spec.inherited_constraints.clone();
        for section in &ctx.resolved_spec.task.sections {
            match section {
                crate::spec_core::Section::Constraints { items, .. } => {
                    all_constraints.extend(items.clone());
                }
                crate::spec_core::Section::Boundaries { items, .. } => {
                    for item in items {
                        if matches!(
                            item.category,
                            BoundaryCategory::Deny | BoundaryCategory::General
                        ) {
                            all_constraints.push(Constraint {
                                text: item.text.clone(),
                                category: ConstraintCategory::MustNot,
                                span: item.span,
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        // Load all source files
        let source_contents = load_source_files(&ctx.code_paths);

        // Check MustNot constraints
        for constraint in &all_constraints {
            if constraint.category == ConstraintCategory::MustNot
                && let Some(result) = check_must_not(constraint, &source_contents)
            {
                results.push(result);
            }
        }

        Ok(results)
    }
}

fn check_must_not(constraint: &Constraint, sources: &[(String, String)]) -> Option<ScenarioResult> {
    if !is_structural_must_not(&constraint.text) {
        return None;
    }

    let patterns = extract_forbidden_patterns(&constraint.text);
    if patterns.is_empty() {
        return None;
    }

    let mut evidence = Vec::new();
    let mut found_violation = false;

    for (file_path, content) in sources {
        for (line_num, line) in content.lines().enumerate() {
            for pattern in &patterns {
                if line.contains(pattern.as_str()) {
                    found_violation = true;
                    evidence.push(Evidence::CodeSnippet {
                        file: file_path.clone(),
                        line: line_num + 1,
                        content: line.trim().to_string(),
                    });
                }
            }
        }
    }

    let verdict = if found_violation {
        Verdict::Fail
    } else {
        Verdict::Pass
    };

    Some(ScenarioResult {
        scenario_name: format!("[structural] {}", truncate(&constraint.text, 50)),
        verdict,
        step_results: vec![StepVerdict {
            step_text: constraint.text.clone(),
            verdict,
            reason: if found_violation {
                format!("found {} violation(s)", evidence.len())
            } else {
                "no violations found".into()
            },
        }],
        evidence,
        duration_ms: 0,
    })
}

fn is_structural_must_not(text: &str) -> bool {
    let lower = text.to_lowercase();
    let triggers = [
        "禁止使用",
        "不要使用",
        "不应存在",
        "不得出现",
        "must not use",
        "do not use",
        "should not contain",
        "must not contain",
    ];

    triggers.iter().any(|trigger| lower.contains(trigger))
}

/// Extract code patterns that should NOT appear from a MustNot constraint.
fn extract_forbidden_patterns(text: &str) -> Vec<String> {
    let mut patterns = Vec::new();

    // Common patterns: "不要使用 X" / "禁止 X" / "No X in production"
    // Look for quoted code identifiers
    let mut in_backtick = false;
    let mut current = String::new();
    for ch in text.chars() {
        if ch == '`' {
            if in_backtick && !current.is_empty() && is_likely_code_pattern(&current) {
                patterns.push(current.clone());
                current.clear();
            }
            in_backtick = !in_backtick;
        } else if in_backtick {
            current.push(ch);
        }
    }

    // Common Rust-specific patterns
    let known_patterns: &[(&str, &str)] = &[
        (".unwrap()", ".unwrap()"),
        (".expect(", ".expect("),
        ("unwrap()", ".unwrap()"),
        ("panic!", "panic!"),
        ("todo!", "todo!"),
        ("f32", "f32"),
        ("f64", "f64"),
        ("浮点", "f32"),
    ];

    let lower = text.to_lowercase();
    for &(trigger, pattern) in known_patterns {
        if lower.contains(trigger) {
            patterns.push(pattern.to_string());
        }
    }

    patterns
}

fn is_likely_code_pattern(pattern: &str) -> bool {
    pattern.chars().any(|ch| {
        matches!(
            ch,
            '.' | '!' | '(' | ')' | '_' | ':' | '/' | '\\' | '-' | '[' | ']'
        )
    })
}

fn load_source_files(paths: &[std::path::PathBuf]) -> Vec<(String, String)> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_file() {
            if let Ok(content) = fs::read_to_string(path) {
                files.push((path.display().to_string(), content));
            }
        } else if path.is_dir() {
            collect_rust_files(path, &mut files);
        }
    }
    files
}

fn collect_rust_files(dir: &Path, files: &mut Vec<(String, String)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        // Never follow symlinks: a linked dir (e.g. a vault symlink) can
        // point outside the workspace or into huge trees.
        if path
            .symlink_metadata()
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false)
        {
            continue;
        }
        if path.is_dir() {
            // Skip dependency/build/hidden dirs: walking node_modules-scale
            // trees is the documented lifecycle-hang root cause.
            if let Some(name) = path.file_name().and_then(|n| n.to_str())
                && (name.starts_with('.')
                    || matches!(
                        name,
                        "target" | "node_modules" | "vendor" | "dist" | "build"
                    ))
            {
                continue;
            }
            collect_rust_files(&path, files);
        } else if let Some(ext) = path.extension()
            && (ext == "rs" || ext == "ts" || ext == "py" || ext == "js")
            && let Ok(content) = fs::read_to_string(&path)
        {
            files.push((path.display().to_string(), content));
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let t: String = s.chars().take(max - 3).collect();
        format!("{t}...")
    }
}

#[cfg(test)]
mod tests {
    use super::{extract_forbidden_patterns, is_structural_must_not};

    #[test]
    fn keeps_code_like_backtick_patterns() {
        let patterns = extract_forbidden_patterns("禁止使用 `panic!` 和 `search_dirs`");
        assert!(patterns.contains(&"panic!".to_string()));
        assert!(patterns.contains(&"search_dirs".to_string()));
    }

    #[test]
    fn ignores_plain_language_backtick_words() {
        let patterns = extract_forbidden_patterns("不要把 `skip` 记为 `pass`");
        assert!(patterns.is_empty());
    }

    #[test]
    fn only_checks_explicit_structural_must_not_rules() {
        assert!(is_structural_must_not("禁止使用 `.unwrap()`"));
        assert!(is_structural_must_not("Do not use `panic!`"));
        assert!(!is_structural_must_not(
            "不要让普通磁盘用例手工传入 `search_dirs`"
        ));
    }
}

#[cfg(test)]
mod walk_tests {
    use super::collect_rust_files;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn structural_walk_skips_dependency_dirs_and_symlinks() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("agent-spec-walk-{nanos}"));
        let external = std::env::temp_dir().join(format!("agent-spec-walk-ext-{nanos}"));
        fs::create_dir_all(root.join("node_modules/dep")).unwrap();
        fs::create_dir_all(root.join("vendor")).unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(&external).unwrap();
        fs::write(root.join("node_modules/dep/a.ts"), "let a = 1;").unwrap();
        fs::write(root.join("vendor/b.rs"), "fn b() {}").unwrap();
        fs::write(root.join("src/d.rs"), "fn d() {}").unwrap();
        fs::write(external.join("c.rs"), "fn c() {}").unwrap();
        std::os::unix::fs::symlink(&external, root.join("linked")).unwrap();

        let mut files = Vec::new();
        collect_rust_files(&root, &mut files);
        let names: Vec<&str> = files.iter().map(|(p, _)| p.as_str()).collect();

        assert_eq!(files.len(), 1, "only src/d.rs collected, got {names:?}");
        assert!(names[0].ends_with("src/d.rs"));

        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&external);
    }
}
