use crate::spec_core::{Lang, SpecLevel, SpecMeta};
use std::collections::BTreeMap;

/// Parse front-matter block (before `---`) into SpecMeta.
#[allow(clippy::too_many_lines)] // Exception: legacy front-matter parser; refactor is outside this migration checkpoint.
pub fn parse_meta(lines: &[&str]) -> Result<SpecMeta, String> {
    let mut level = None;
    let mut name = None;
    let mut inherits = None;
    let mut lang = Vec::new();
    let mut tags = Vec::new();
    let mut depends = Vec::new();
    let mut estimate = None;
    let mut runner = None;
    let mut runner_config = BTreeMap::new();

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        let key = key.trim().to_lowercase();
        let value = value.trim().trim_matches('"');

        match key.as_str() {
            "spec" => {
                level = Some(match value.to_lowercase().as_str() {
                    "org" => SpecLevel::Org,
                    "project" => SpecLevel::Project,
                    "task" => SpecLevel::Task,
                    other => return Err(format!("unknown spec level: {other}")),
                });
            }
            "name" => {
                name = Some(value.to_string());
            }
            "inherits" => {
                let v = value.trim();
                if !v.is_empty() {
                    inherits = Some(v.to_string());
                }
            }
            "lang" => {
                for part in value.split(',') {
                    match part.trim().to_lowercase().as_str() {
                        "zh" => lang.push(Lang::Zh),
                        "en" => lang.push(Lang::En),
                        _ => {}
                    }
                }
            }
            "tags" => {
                let value = value.trim_start_matches('[').trim_end_matches(']');
                for tag in value.split(',') {
                    let t = tag.trim();
                    if !t.is_empty() {
                        tags.push(t.to_string());
                    }
                }
            }
            "depends" => {
                let value = value.trim_start_matches('[').trim_end_matches(']');
                for dep in value.split(',') {
                    let d = dep.trim();
                    if !d.is_empty() {
                        depends.push(d.to_string());
                    }
                }
            }
            "estimate" => {
                let v = value.trim();
                if !v.is_empty() {
                    estimate = Some(v.to_string());
                }
            }
            "runner" => {
                let v = value.trim();
                if !v.is_empty() {
                    runner = Some(v.to_string());
                }
            }
            "runner_config" => {
                runner_config = parse_inline_string_map(value)?;
            }
            _ => {} // ignore unknown keys
        }
    }

    Ok(SpecMeta {
        level: level.ok_or("missing 'spec:' field in front-matter")?,
        name: name.unwrap_or_else(|| "unnamed".to_string()),
        inherits,
        lang: if lang.is_empty() {
            vec![Lang::Zh, Lang::En]
        } else {
            lang
        },
        tags,
        runner,
        runner_config,
        depends,
        estimate,
    })
}

fn parse_inline_string_map(value: &str) -> Result<BTreeMap<String, String>, String> {
    let mut map = BTreeMap::new();
    let value = value.trim();
    if value.is_empty() || value == "{}" {
        return Ok(map);
    }
    let body = value
        .strip_prefix('{')
        .and_then(|v| v.strip_suffix('}'))
        .ok_or_else(|| "runner_config must use inline map syntax".to_string())?;

    for part in split_inline_map_entries(body) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let Some((key, value)) = part.split_once(':') else {
            return Err(format!("invalid runner_config entry: {part}"));
        };
        let key = key.trim().trim_matches('"');
        let value = value.trim().trim_matches('"');
        if !key.is_empty() {
            map.insert(key.to_string(), value.to_string());
        }
    }

    Ok(map)
}

fn split_inline_map_entries(body: &str) -> Vec<&str> {
    let mut entries = Vec::new();
    let mut start = 0;
    let mut in_quotes = false;

    for (index, ch) in body.char_indices() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                entries.push(&body[start..index]);
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }

    entries.push(&body[start..]);
    entries
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_meta() {
        let lines = vec![
            "spec: task",
            r#"name: "退款功能""#,
            "inherits: project",
            "tags: [payment, refund]",
            "lang: zh",
        ];
        let meta = parse_meta(&lines).unwrap();
        assert_eq!(meta.level, SpecLevel::Task);
        assert_eq!(meta.name, "退款功能");
        assert_eq!(meta.inherits, Some("project".into()));
        assert_eq!(meta.tags, vec!["payment", "refund"]);
        assert_eq!(meta.lang, vec![Lang::Zh]);
    }

    #[test]
    fn test_parse_minimal_meta() {
        let lines = vec!["spec: org"];
        let meta = parse_meta(&lines).unwrap();
        assert_eq!(meta.level, SpecLevel::Org);
        assert_eq!(meta.name, "unnamed");
        assert!(meta.inherits.is_none());
        assert_eq!(meta.lang, vec![Lang::Zh, Lang::En]);
    }

    #[test]
    fn test_parse_spec_depends_and_estimate_fields() {
        let lines = vec![
            "spec: task",
            r#"name: "依赖图测试""#,
            "inherits: project",
            "tags: [bootstrap]",
            "depends: [task-goal-gate]",
            "estimate: 3d",
        ];
        let meta = parse_meta(&lines).unwrap();
        assert_eq!(meta.depends, vec!["task-goal-gate"]);
        assert_eq!(meta.estimate, Some("3d".to_string()));
    }

    #[test]
    fn test_parse_meta_multiple_depends() {
        let lines = vec![
            "spec: task",
            r#"name: "多依赖""#,
            "depends: [task-a, task-b, task-c]",
        ];
        let meta = parse_meta(&lines).unwrap();
        assert_eq!(meta.depends, vec!["task-a", "task-b", "task-c"]);
    }

    #[test]
    fn test_parse_meta_no_depends_no_estimate() {
        let lines = vec!["spec: task", r#"name: "无依赖""#];
        let meta = parse_meta(&lines).unwrap();
        assert!(meta.depends.is_empty());
        assert!(meta.estimate.is_none());
    }

    #[test]
    fn test_runner_workspace_carries_runner_config_map() {
        let lines = vec![
            "spec: task",
            r#"name: "Runner config""#,
            "runner: ios",
            r#"runner_config: { destination: "platform=iOS Simulator,name=iPhone 15", scheme: "App" }"#,
        ];

        let meta = parse_meta(&lines).unwrap();
        let value = serde_json::to_value(&meta).unwrap();

        assert_eq!(value["runner"], "ios");
        assert_eq!(
            value["runner_config"]["destination"],
            "platform=iOS Simulator,name=iPhone 15"
        );
        assert_eq!(value["runner_config"]["scheme"], "App");
    }

    #[test]
    fn test_empty_runner_config_round_trips_byte_equivalent() {
        let lines = vec!["spec: task", r#"name: "No runner config""#];
        let meta = parse_meta(&lines).unwrap();
        let value = serde_json::to_value(&meta).unwrap();

        assert!(meta.runner_config.is_empty());
        assert!(value.get("runner_config").is_none());
        assert!(value.get("runner").is_none());
    }
}
