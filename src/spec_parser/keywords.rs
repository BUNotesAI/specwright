use crate::spec_core::StepKind;

/// English-only keyword recognition for BDD steps (case-insensitive).
///
/// Chinese step aliases were removed: structural keywords must be English.
/// The step description text after the keyword may still be Chinese.
pub fn match_step_keyword(line: &str) -> Option<(StepKind, &str)> {
    let trimmed = line.trim();

    let en_mappings: &[(&str, StepKind)] = &[
        ("given ", StepKind::Given),
        ("when ", StepKind::When),
        ("then ", StepKind::Then),
        ("and ", StepKind::And),
        ("but ", StepKind::But),
    ];

    let lower = trimmed.to_lowercase();
    for &(kw, kind) in en_mappings {
        if lower.starts_with(kw) {
            let rest = trimmed[kw.len()..].trim();
            return Some((kind, rest));
        }
    }

    None
}

/// English-only section header recognition (case-insensitive).
pub fn match_section_header(line: &str) -> Option<SectionKind> {
    let trimmed = line.trim().trim_start_matches('#').trim();
    let lower = trimmed.to_lowercase();

    if lower.starts_with("intent") {
        Some(SectionKind::Intent)
    } else if lower.starts_with("constraint") {
        Some(SectionKind::Constraints)
    } else if lower.starts_with("decision") {
        Some(SectionKind::Decisions)
    } else if lower.starts_with("boundaries") || lower.starts_with("boundary") {
        Some(SectionKind::Boundaries)
    } else if lower.starts_with("acceptance criter") || lower.starts_with("completion criter") {
        Some(SectionKind::AcceptanceCriteria)
    } else if lower.starts_with("out of scope") {
        Some(SectionKind::OutOfScope)
    } else {
        None
    }
}

/// English-only scenario header recognition.
pub fn match_scenario_header(line: &str) -> Option<&str> {
    let trimmed = line.trim().trim_start_matches('#').trim();
    let lower = trimmed.to_lowercase();
    if lower.starts_with("scenario:") {
        Some(trimmed["scenario:".len()..].trim())
    } else {
        None
    }
}

/// Scenario-level test selector binding (English-only).
pub fn match_test_selector(line: &str) -> Option<&str> {
    let trimmed = line.trim().trim_start_matches('#').trim();
    let lower = trimmed.to_lowercase();
    if lower.starts_with("test:") {
        Some(trimmed["test:".len()..].trim())
    } else {
        None
    }
}

/// Scenario-level tags line recognition (e.g., `Tags: [critical]`).
pub fn match_scenario_tags(line: &str) -> Option<Vec<String>> {
    let trimmed = line.trim().trim_start_matches('#').trim();

    let lower = trimmed.to_lowercase();
    let value = if lower.starts_with("tags:") {
        Some(trimmed["tags:".len()..].trim())
    } else {
        None
    };

    value.map(|v| {
        let v = v.trim_start_matches('[').trim_end_matches(']');
        v.split(',')
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect()
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestSelectorField {
    Package,
    Filter,
    Level,
    TestDouble,
    Targets,
}

/// Structured fields under a `Test:` selector block (English-only).
pub fn match_test_selector_field(line: &str) -> Option<(TestSelectorField, &str)> {
    let trimmed = line.trim().trim_start_matches('#').trim();
    let lower = trimmed.to_lowercase();

    if lower.starts_with("package:") {
        return Some((
            TestSelectorField::Package,
            trimmed["package:".len()..].trim(),
        ));
    }
    if lower.starts_with("filter:") {
        return Some((TestSelectorField::Filter, trimmed["filter:".len()..].trim()));
    }
    if lower.starts_with("level:") {
        return Some((TestSelectorField::Level, trimmed["level:".len()..].trim()));
    }
    if lower.starts_with("test double:") {
        return Some((
            TestSelectorField::TestDouble,
            trimmed["test double:".len()..].trim(),
        ));
    }
    if lower.starts_with("targets:") {
        return Some((
            TestSelectorField::Targets,
            trimmed["targets:".len()..].trim(),
        ));
    }

    None
}

/// Review field recognition: `Review: human`.
/// Returns Some("human") or Some("auto"), or None if not a review line.
pub fn match_review_field(line: &str) -> Option<&str> {
    let trimmed = line.trim().trim_start_matches('#').trim();
    let lower = trimmed.to_lowercase();
    if lower.starts_with("review:") {
        return Some(trimmed["review:".len()..].trim());
    }
    None
}

/// Mode field recognition: `Mode: optimize`.
/// Returns Some("optimize") or Some("standard"), or None if not a mode line.
pub fn match_mode_field(line: &str) -> Option<&str> {
    let trimmed = line.trim().trim_start_matches('#').trim();
    let lower = trimmed.to_lowercase();
    if lower.starts_with("mode:") {
        return Some(trimmed["mode:".len()..].trim());
    }
    None
}

/// Depends field recognition: `Depends: A, B`.
/// Returns Some("A, B") or None if not a depends line.
pub fn match_depends_field(line: &str) -> Option<&str> {
    let trimmed = line.trim().trim_start_matches('#').trim();
    let lower = trimmed.to_lowercase();
    if lower.starts_with("depends:") {
        return Some(trimmed["depends:".len()..].trim());
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionKind {
    Intent,
    Constraints,
    Decisions,
    Boundaries,
    AcceptanceCriteria,
    OutOfScope,
}

/// A structural keyword that the parser used to accept in Chinese but now
/// rejects. Returned only to build a clear English error message; it never
/// makes a Chinese keyword parse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemovedKeyword {
    /// The Chinese token as it appears in source (for the error message).
    pub cjk: &'static str,
    /// The English keyword the author should use instead.
    pub english: &'static str,
}

/// Detect a line whose structural-keyword position holds a now-removed Chinese
/// keyword (step / scenario / test-selector / tags / selector-field /
/// constraint-or-boundary `###` sub-heading).
///
/// Used ONLY to turn what would otherwise be a silent drop or miscategorization
/// into a clear English parse error. It is position-anchored exactly like the
/// real matchers, so ordinary Chinese description text is never flagged: a
/// `- 当 ...` bullet starts with `-`, so its `当` is prose, not a step keyword.
pub fn detect_removed_cjk_keyword(line: &str) -> Option<RemovedKeyword> {
    let s = line.trim_start();
    let has_hash = s.starts_with('#');
    let body = s.trim_start_matches('#').trim_start();

    // Header keywords, which may carry a leading `#` (e.g. `### 场景:`).
    const HEADERS: &[(&str, &str)] = &[
        ("场景:", "Scenario:"),
        ("场景：", "Scenario:"),
        ("测试:", "Test:"),
        ("测试：", "Test:"),
        ("标签:", "Tags:"),
        ("标签：", "Tags:"),
    ];
    for &(cjk, english) in HEADERS {
        if body.starts_with(cjk) {
            return Some(RemovedKeyword { cjk, english });
        }
    }

    // Constraint / boundary `###` sub-headings (only when `#`-prefixed).
    if has_hash {
        const SUBHEADINGS: &[(&str, &str)] = &[
            ("允许修改", "Allowed Changes"),
            ("禁止做", "Forbidden / Must Not"),
            ("禁止", "Forbidden / Must Not"),
            ("必须做", "Must"),
            ("已定决策", "Decided"),
            ("已定", "Decided"),
        ];
        for &(cjk, english) in SUBHEADINGS {
            if body.starts_with(cjk) {
                return Some(RemovedKeyword { cjk, english });
            }
        }
        // A `#`-prefixed line is a heading, not a step or field line.
        return None;
    }

    // Selector / meta fields (line-anchored, colon-terminated).
    const FIELDS: &[(&str, &str)] = &[
        ("包:", "Package:"),
        ("包：", "Package:"),
        ("过滤:", "Filter:"),
        ("过滤：", "Filter:"),
        ("层级:", "Level:"),
        ("层级：", "Level:"),
        ("替身:", "Test Double:"),
        ("替身：", "Test Double:"),
        ("命中:", "Targets:"),
        ("命中：", "Targets:"),
        ("审核:", "Review:"),
        ("审核：", "Review:"),
        ("模式:", "Mode:"),
        ("模式：", "Mode:"),
        ("前置:", "Depends:"),
        ("前置：", "Depends:"),
    ];
    for &(cjk, english) in FIELDS {
        if s.starts_with(cjk) {
            return Some(RemovedKeyword { cjk, english });
        }
    }

    // Step keywords: the first token after indentation, followed by whitespace
    // (or end of line). A `- 当 ...` bullet starts with `-`, so it is not a step.
    const STEPS: &[(&str, &str)] = &[
        ("假设", "Given"),
        ("那么", "Then"),
        ("并且", "And"),
        ("而且", "And"),
        ("但是", "But"),
        ("当", "When"),
    ];
    for &(cjk, english) in STEPS {
        if let Some(rest) = s.strip_prefix(cjk)
            && (rest.is_empty() || rest.starts_with(char::is_whitespace))
        {
            return Some(RemovedKeyword { cjk, english });
        }
    }

    None
}

/// Extract quoted parameters from step text.
/// e.g., `存在一笔金额为 "100.00" 元的交易 "TXN-001"` → ["100.00", "TXN-001"]
pub fn extract_params(text: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch == '"' || ch == '\u{201C}' || ch == '\u{201D}' {
            // collect until closing quote
            let mut param = String::new();
            for inner in chars.by_ref() {
                if inner == '"' || inner == '\u{201C}' || inner == '\u{201D}' {
                    break;
                }
                param.push(inner);
            }
            if !param.is_empty() {
                params.push(param);
            }
        }
    }
    params
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    // --- English keywords are still recognized (case-insensitive) ---

    #[test]
    fn test_match_step_english() {
        let (kind, rest) = match_step_keyword("  Given a user exists").unwrap();
        assert_eq!(kind, StepKind::Given);
        assert_eq!(rest, "a user exists");
    }

    #[test]
    fn test_english_keywords_remain_case_insensitive() {
        assert_eq!(match_step_keyword("given x").unwrap().0, StepKind::Given);
        assert_eq!(match_step_keyword("WHEN x").unwrap().0, StepKind::When);
        assert_eq!(match_step_keyword("Then x").unwrap().0, StepKind::Then);
        assert_eq!(match_step_keyword("AND x").unwrap().0, StepKind::And);
        assert_eq!(
            match_section_header("## intent").unwrap(),
            SectionKind::Intent
        );
        assert_eq!(
            match_section_header("## BOUNDARIES").unwrap(),
            SectionKind::Boundaries
        );
        assert_eq!(match_scenario_header("SCENARIO: x").unwrap(), "x");
        assert_eq!(match_test_selector("TEST: t").unwrap(), "t");
        assert_eq!(
            match_test_selector_field("PACKAGE: p").unwrap(),
            (TestSelectorField::Package, "p")
        );
    }

    #[test]
    fn test_scenario_header_english() {
        assert_eq!(
            match_scenario_header("Scenario: Full refund"),
            Some("Full refund")
        );
        assert_eq!(
            match_scenario_header("### Scenario: Full refund"),
            Some("Full refund")
        );
    }

    #[test]
    fn test_match_test_selector_english() {
        assert_eq!(
            match_test_selector("  Test: test_parse_contract"),
            Some("test_parse_contract")
        );
        assert_eq!(
            match_test_selector("### Test: test_parse_contract"),
            Some("test_parse_contract")
        );
    }

    #[test]
    fn test_match_test_selector_fields_english() {
        assert_eq!(
            match_test_selector_field("  Package: spec-parser"),
            Some((TestSelectorField::Package, "spec-parser"))
        );
        assert_eq!(
            match_test_selector_field("  Filter: test_parse_contract"),
            Some((TestSelectorField::Filter, "test_parse_contract"))
        );
        assert_eq!(
            match_test_selector_field("  Level: integration"),
            Some((TestSelectorField::Level, "integration"))
        );
        assert_eq!(
            match_test_selector_field("  Test Double: local_http_stub"),
            Some((TestSelectorField::TestDouble, "local_http_stub"))
        );
        assert_eq!(
            match_test_selector_field("  Targets: commands/update"),
            Some((TestSelectorField::Targets, "commands/update"))
        );
    }

    #[test]
    fn test_section_headers_english() {
        assert_eq!(match_section_header("## Intent"), Some(SectionKind::Intent));
        assert_eq!(
            match_section_header("## Constraints"),
            Some(SectionKind::Constraints)
        );
        assert_eq!(
            match_section_header("## Decisions"),
            Some(SectionKind::Decisions)
        );
        assert_eq!(
            match_section_header("## Boundaries"),
            Some(SectionKind::Boundaries)
        );
        assert_eq!(
            match_section_header("## Acceptance Criteria"),
            Some(SectionKind::AcceptanceCriteria)
        );
        assert_eq!(
            match_section_header("## Completion Criteria"),
            Some(SectionKind::AcceptanceCriteria)
        );
        assert_eq!(
            match_section_header("## Out of Scope"),
            Some(SectionKind::OutOfScope)
        );
    }

    // --- Chinese keywords are now rejected (return None) ---

    #[test]
    fn test_match_step_keyword_rejects_cjk() {
        assert!(match_step_keyword("假设 数据库中存在用户").is_none());
        assert!(match_step_keyword("当 用户登录").is_none());
        assert!(match_step_keyword("那么 显示成功").is_none());
        assert!(match_step_keyword("并且 用户已登录").is_none());
        assert!(match_step_keyword("但是 余额不足").is_none());
    }

    #[test]
    fn test_match_section_header_rejects_cjk() {
        for header in [
            "## 意图",
            "## 约束",
            "## 决策",
            "## 已定决策",
            "## 边界",
            "## 验收标准",
            "## 完成条件",
            "## 排除范围",
        ] {
            assert!(
                match_section_header(header).is_none(),
                "expected None for {header}"
            );
        }
    }

    #[test]
    fn test_match_scenario_test_tags_reject_cjk() {
        assert!(match_scenario_header("场景: 全额退款").is_none());
        assert!(match_scenario_header("场景：全额退款").is_none());
        assert!(match_test_selector("测试: test_refund").is_none());
        assert!(match_test_selector("测试：test_refund").is_none());
        assert!(match_scenario_tags("标签: [critical]").is_none());
        assert!(match_scenario_tags("标签：[critical]").is_none());
    }

    #[test]
    fn test_match_selector_and_meta_fields_reject_cjk() {
        assert!(match_test_selector_field("包: spec-parser").is_none());
        assert!(match_test_selector_field("过滤: test_x").is_none());
        assert!(match_test_selector_field("层级: unit").is_none());
        assert!(match_test_selector_field("替身: stub").is_none());
        assert!(match_test_selector_field("命中: cmd").is_none());
        assert!(match_review_field("审核: human").is_none());
        assert!(match_mode_field("模式: optimize").is_none());
        assert!(match_depends_field("前置: A, B").is_none());
    }

    // --- detect_removed_cjk_keyword: precise, position-anchored ---

    #[test]
    fn test_detect_removed_cjk_keyword_maps_to_english() {
        // Headers, steps, fields, and sub-headings map to their English forms.
        assert_eq!(
            detect_removed_cjk_keyword("场景: 全额退款").map(|r| r.english),
            Some("Scenario:")
        );
        assert_eq!(
            detect_removed_cjk_keyword("  假设 用户已登录").map(|r| r.english),
            Some("Given")
        );
        // 而且 is an extra "And" alias the original matchers never accepted; it
        // must still be rejected with a clear hint, not silently dropped.
        assert_eq!(
            detect_removed_cjk_keyword("  而且 数据库中有新记录").map(|r| r.english),
            Some("And")
        );
        assert_eq!(
            detect_removed_cjk_keyword("  包: spec-parser").map(|r| r.english),
            Some("Package:")
        );
        assert_eq!(
            detect_removed_cjk_keyword("### 允许修改").map(|r| r.english),
            Some("Allowed Changes")
        );
        assert_eq!(
            detect_removed_cjk_keyword("### 必须做").map(|r| r.english),
            Some("Must")
        );

        // English keyword lines are not flagged.
        assert!(detect_removed_cjk_keyword("Given a user").is_none());
        assert!(detect_removed_cjk_keyword("### Allowed Changes").is_none());

        // Ordinary Chinese description text is not flagged: a "- 当 ..." bullet
        // (当 is the prose word "when", not a step keyword) and 当前 (当 + 前).
        assert!(detect_removed_cjk_keyword("- 当 critical 场景 fail 时退出码为 2").is_none());
        assert!(detect_removed_cjk_keyword("当然这是描述").is_none());
        assert!(detect_removed_cjk_keyword("普通中文描述文字").is_none());
    }

    // --- params extraction is unchanged (description content stays Chinese) ---

    #[test]
    fn test_extract_params() {
        let params = extract_params(r#"金额为 "100.00" 元的交易 "TXN-001""#);
        assert_eq!(params, vec!["100.00", "TXN-001"]);
    }

    #[test]
    fn test_extract_params_chinese_quotes() {
        let params = extract_params("金额为\u{201C}100.00\u{201D}元");
        assert_eq!(params, vec!["100.00"]);
    }

    #[test]
    fn test_not_a_step() {
        assert!(match_step_keyword("这是普通文字").is_none());
        assert!(match_step_keyword("- 约束条目").is_none());
    }
}
