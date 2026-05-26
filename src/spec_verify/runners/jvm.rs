use std::collections::HashMap;
use std::path::Path;

use super::RunnerWorkspace;

/// Shared Java/Kotlin legacy `@Spec` binding scanner.
pub struct JvmBindingScanner;

impl JvmBindingScanner {
    pub fn scan(path: &Path, source: &str) -> Vec<(String, String)> {
        let mut bindings = Vec::new();
        let mut pending_specs = Vec::new();
        let mut saw_test_attr = false;
        let mut current_class = class_name_from_path(path);

        for line in source.lines() {
            let trimmed = line.trim();

            if let Some(spec_name) = extract_spec_name(trimmed) {
                pending_specs.push(spec_name);
                continue;
            }

            if is_test_attr(trimmed) {
                saw_test_attr = true;
                continue;
            }

            if let Some(class_name) = extract_class_name(trimmed) {
                current_class = Some(class_name.clone());
                if !saw_test_attr {
                    for spec_name in pending_specs.drain(..) {
                        bindings.push((spec_name, class_name.clone()));
                    }
                }
                saw_test_attr = false;
                continue;
            }

            if saw_test_attr
                && let Some(method_name) = extract_method_name(trimmed)
                && let Some(class_name) = current_class.as_ref()
            {
                for spec_name in pending_specs.drain(..) {
                    bindings.push((spec_name, format!("{class_name}#{method_name}")));
                }
                saw_test_attr = false;
                continue;
            }

            if clears_pending_annotation(trimmed) {
                pending_specs.clear();
                saw_test_attr = false;
            }
        }

        bindings
    }
}

pub fn scan_workspace_bindings(workspace: &RunnerWorkspace) -> HashMap<String, String> {
    let mut bindings = HashMap::new();
    for source in &workspace.source_files {
        if !is_jvm_source(&source.path) {
            continue;
        }
        for (scenario, selector) in JvmBindingScanner::scan(&source.path, &source.content) {
            bindings.entry(scenario).or_insert(selector);
        }
    }
    bindings
}

fn is_jvm_source(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension, "java" | "kt"))
}

fn extract_spec_name(line: &str) -> Option<String> {
    let rest = line.strip_prefix("@Spec(")?;
    let start = rest.find('"')?;
    let after_start = &rest[start + 1..];
    let end = after_start.find('"')?;
    Some(after_start[..end].to_string())
}

fn is_test_attr(line: &str) -> bool {
    line == "@Test" || line.starts_with("@Test(") || line.ends_with(".Test")
}

fn extract_class_name(line: &str) -> Option<String> {
    let mut tokens = line.split_whitespace();
    while let Some(token) = tokens.next() {
        if token == "class" {
            return tokens.next().and_then(clean_identifier);
        }
    }
    None
}

fn extract_method_name(line: &str) -> Option<String> {
    let before_paren = line.split('(').next()?.trim();
    if before_paren.is_empty() {
        return None;
    }

    let name = before_paren.split_whitespace().last()?;
    if matches!(name, "if" | "for" | "while" | "switch" | "class") {
        None
    } else {
        clean_identifier(name)
    }
}

fn class_name_from_path(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::to_string)
}

fn clean_identifier(raw: &str) -> Option<String> {
    let identifier = raw
        .trim_matches('{')
        .trim_matches('}')
        .trim_matches(':')
        .trim_matches('<')
        .trim_matches('>')
        .trim();
    if identifier.is_empty() {
        None
    } else {
        Some(identifier.to_string())
    }
}

fn clears_pending_annotation(line: &str) -> bool {
    !(line.is_empty()
        || line.starts_with('@')
        || line.starts_with("//")
        || line.starts_with("package ")
        || line.starts_with("import "))
}
