use crate::spec_core::{
    Boundary, BoundaryCategory, Constraint, ConstraintCategory, ReviewMode, Scenario, ScenarioMode,
    Section, Span, SpecDocument, SpecError, SpecResult, Step, TestSelector,
};
use std::path::{Path, PathBuf};

use super::keywords::{
    RemovedKeyword, SectionKind, TestSelectorField, detect_removed_cjk_keyword, extract_params,
    match_depends_field, match_mode_field, match_review_field, match_scenario_header,
    match_scenario_tags, match_section_header, match_step_keyword, match_test_selector,
    match_test_selector_field,
};
use super::meta::parse_meta;

/// Build the clear English parse error for a now-removed Chinese keyword.
fn cjk_keyword_error(removed: RemovedKeyword, line_num: usize) -> SpecError {
    SpecError::Parse {
        message: format!(
            "keywords must be English; '{}' is not recognized — use '{}'",
            removed.cjk, removed.english
        ),
        span: Span::line(line_num),
    }
}

/// Parse a .spec/.spec.md file from disk.
pub fn parse_spec(path: &Path) -> SpecResult<SpecDocument> {
    let content = std::fs::read_to_string(path)?;
    let mut doc = parse_spec_from_str(&content)?;
    doc.source_path = path.to_path_buf();
    Ok(doc)
}

/// Parse a .spec/.spec.md string into a SpecDocument.
pub fn parse_spec_from_str(input: &str) -> SpecResult<SpecDocument> {
    let lines: Vec<&str> = input.lines().collect();

    // Split on front-matter separator `---`
    let separator_pos = lines.iter().position(|l| l.trim() == "---");
    let (meta_lines, body_lines, body_offset) = match separator_pos {
        Some(pos) => (&lines[..pos], &lines[pos + 1..], pos + 1),
        None => {
            // No front-matter: try to parse entire content as body
            // with a minimal default meta
            return Err(SpecError::FrontMatter(
                "missing front-matter separator '---'".into(),
            ));
        }
    };

    let meta = parse_meta(meta_lines).map_err(SpecError::FrontMatter)?;

    let sections = parse_body(body_lines, body_offset)?;

    Ok(SpecDocument {
        meta,
        sections,
        source_path: PathBuf::new(),
    })
}

/// Parse the body of a spec (after `---`) into sections.
fn parse_body(lines: &[&str], offset: usize) -> SpecResult<Vec<Section>> {
    let mut sections = Vec::new();
    let mut current_section: Option<(SectionKind, usize)> = None; // (kind, start_line)
    let mut section_lines: Vec<(usize, &str)> = Vec::new(); // (absolute_line, text)

    for (i, &line) in lines.iter().enumerate() {
        let abs_line = offset + i + 1; // 1-indexed

        if let Some(kind) = match_section_header(line) {
            // Flush previous section
            if let Some((prev_kind, start)) = current_section.take() {
                let section = build_section(prev_kind, &section_lines, start)?;
                sections.push(section);
                section_lines.clear();
            }
            current_section = Some((kind, abs_line));
        } else if matches!(markdown_heading_level(line), Some(1 | 2)) {
            let header = line.trim().trim_start_matches('#').trim();
            return Err(SpecError::Parse {
                message: format!(
                    "unknown top-level section header '{header}' - use only Intent/Constraints/Decisions/Boundaries/Acceptance Criteria/Out of Scope"
                ),
                span: Span::line(abs_line),
            });
        } else if current_section.is_some() {
            section_lines.push((abs_line, line));
        }
    }

    // Flush last section
    if let Some((kind, start)) = current_section {
        let section = build_section(kind, &section_lines, start)?;
        sections.push(section);
    }

    Ok(sections)
}

fn markdown_heading_level(line: &str) -> Option<usize> {
    let trimmed = line.trim_start();
    let level = trimmed.chars().take_while(|&ch| ch == '#').count();
    if level == 0 || level == trimmed.len() {
        return None;
    }
    Some(level)
}

fn build_section(
    kind: SectionKind,
    lines: &[(usize, &str)],
    start_line: usize,
) -> SpecResult<Section> {
    let end_line = lines.last().map_or(start_line, |(ln, _)| *ln);
    let span = Span::new(start_line, 0, end_line, 0);

    match kind {
        SectionKind::Intent => {
            let content: String = lines
                .iter()
                .map(|(_, l)| *l)
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_string();
            Ok(Section::Intent { content, span })
        }
        SectionKind::Constraints => {
            let items = parse_constraints(lines)?;
            Ok(Section::Constraints { items, span })
        }
        SectionKind::Decisions => {
            let items = parse_string_list(lines);
            Ok(Section::Decisions { items, span })
        }
        SectionKind::Boundaries => {
            let items = parse_boundaries(lines)?;
            Ok(Section::Boundaries { items, span })
        }
        SectionKind::AcceptanceCriteria => {
            let scenarios = parse_scenarios(lines)?;
            Ok(Section::AcceptanceCriteria { scenarios, span })
        }
        SectionKind::OutOfScope => {
            let items = lines
                .iter()
                .filter_map(|(_, l)| {
                    let trimmed = l.trim().strip_prefix('-').map(str::trim);
                    trimmed.filter(|s| !s.is_empty()).map(String::from)
                })
                .collect();
            Ok(Section::OutOfScope { items, span })
        }
    }
}

fn parse_constraints(lines: &[(usize, &str)]) -> SpecResult<Vec<Constraint>> {
    let mut constraints = Vec::new();
    let mut category = ConstraintCategory::General;

    for &(line_num, line) in lines {
        let trimmed = line.trim();

        // Sub-section headers for constraint categories (English-only)
        if trimmed.starts_with("###") {
            let header = trimmed.trim_start_matches('#').trim().to_lowercase();
            if header.contains("must") && !header.contains("not") {
                category = ConstraintCategory::Must;
            } else if header.contains("must not") {
                category = ConstraintCategory::MustNot;
            } else if header.contains("decided") {
                category = ConstraintCategory::Decided;
            } else if let Some(removed) = detect_removed_cjk_keyword(trimmed) {
                return Err(cjk_keyword_error(removed, line_num));
            }
            continue;
        }

        // Bullet items
        if let Some(text) = trimmed.strip_prefix('-') {
            let text = text.trim();
            if !text.is_empty() {
                constraints.push(Constraint {
                    text: text.to_string(),
                    category,
                    span: Span::line(line_num),
                });
            }
        }
    }

    Ok(constraints)
}

fn parse_string_list(lines: &[(usize, &str)]) -> Vec<String> {
    lines
        .iter()
        .filter_map(|(_, line)| line.trim().strip_prefix('-').map(str::trim))
        .filter(|text| !text.is_empty())
        .map(String::from)
        .collect()
}

fn parse_boundaries(lines: &[(usize, &str)]) -> SpecResult<Vec<Boundary>> {
    let mut items = Vec::new();
    let mut category = BoundaryCategory::General;

    for &(line_num, line) in lines {
        let trimmed = line.trim();

        if trimmed.starts_with("###") {
            let header = trimmed.trim_start_matches('#').trim().to_lowercase();
            if header.contains("allowed") || header.contains("allow") {
                category = BoundaryCategory::Allow;
            } else if header.contains("forbidden")
                || header.contains("must not")
                || header.contains("disallow")
            {
                category = BoundaryCategory::Deny;
            } else if let Some(removed) = detect_removed_cjk_keyword(trimmed) {
                return Err(cjk_keyword_error(removed, line_num));
            }
            continue;
        }

        if let Some(text) = trimmed.strip_prefix('-') {
            let text = text.trim();
            if !text.is_empty() {
                items.push(Boundary {
                    text: text.to_string(),
                    category,
                    span: Span::line(line_num),
                });
            }
        }
    }

    Ok(items)
}

#[allow(clippy::too_many_lines)] // Exception: legacy scenario parser state machine; refactor is outside this migration checkpoint.
fn parse_scenarios(lines: &[(usize, &str)]) -> SpecResult<Vec<Scenario>> {
    let mut scenarios = Vec::new();
    let mut current_name: Option<(String, usize)> = None;
    let mut current_steps: Vec<Step> = Vec::new();
    let mut current_test_selector: Option<TestSelectorDraft> = None;
    let mut current_tags: Vec<String> = Vec::new();
    let mut current_review: ReviewMode = ReviewMode::default();
    let mut current_mode: ScenarioMode = ScenarioMode::Standard;
    let mut current_depends_on: Vec<String> = Vec::new();
    let mut reading_test_selector_block = false;

    for &(line_num, line) in lines {
        if let Some(name) = match_scenario_header(line) {
            // Flush previous scenario
            if let Some((prev_name, start)) = current_name.take() {
                let end = current_steps.last().map_or(start, |s| s.span.end_line);
                scenarios.push(Scenario {
                    name: prev_name,
                    steps: std::mem::take(&mut current_steps),
                    test_selector: finalize_test_selector(current_test_selector.take(), end)?,
                    tags: std::mem::take(&mut current_tags),
                    review: std::mem::take(&mut current_review),
                    mode: std::mem::take(&mut current_mode),
                    depends_on: std::mem::take(&mut current_depends_on),
                    span: Span::new(start, 0, end, 0),
                });
            }
            current_name = Some((name.to_string(), line_num));
            current_tags = Vec::new();
            current_review = ReviewMode::default();
            current_mode = ScenarioMode::Standard;
            current_depends_on = Vec::new();
            reading_test_selector_block = false;
        } else if let Some(tags) = match_scenario_tags(line) {
            if current_name.is_some() {
                current_tags = tags;
            }
        } else if let Some(review_value) = match_review_field(line) {
            if current_name.is_some() {
                let lower = review_value.to_lowercase();
                if lower == "human" {
                    current_review = ReviewMode::Human;
                } else {
                    current_review = ReviewMode::Auto;
                }
            }
        } else if let Some(mode_value) = match_mode_field(line) {
            if current_name.is_some() {
                let lower = mode_value.to_lowercase();
                if lower == "optimize" {
                    current_mode = ScenarioMode::Optimize;
                } else {
                    current_mode = ScenarioMode::Standard;
                }
            }
        } else if let Some(depends_value) = match_depends_field(line) {
            if current_name.is_some() {
                current_depends_on = depends_value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        } else if let Some(selector) = match_test_selector(line) {
            if current_name.is_some() {
                let draft = current_test_selector.get_or_insert_with(TestSelectorDraft::default);
                if selector.is_empty() {
                    reading_test_selector_block = true;
                } else {
                    draft.filter = Some(selector.to_string());
                    reading_test_selector_block = false;
                }
            }
        } else if reading_test_selector_block {
            if let Some((field, value)) = match_test_selector_field(line) {
                let draft = current_test_selector.get_or_insert_with(TestSelectorDraft::default);
                match field {
                    TestSelectorField::Package => draft.package = Some(value.to_string()),
                    TestSelectorField::Filter => draft.filter = Some(value.to_string()),
                    TestSelectorField::Level => draft.level = Some(value.to_string()),
                    TestSelectorField::TestDouble => draft.test_double = Some(value.to_string()),
                    TestSelectorField::Targets => draft.targets = Some(value.to_string()),
                }
                continue;
            }
            if line.trim().is_empty() {
                continue;
            }
            reading_test_selector_block = false;
        }

        if let Some((kind, text)) = match_step_keyword(line) {
            let params = extract_params(text);
            current_steps.push(Step {
                kind,
                text: text.to_string(),
                params,
                table: Vec::new(),
                span: Span::line(line_num),
            });
        } else if let Some(row) = parse_table_row(line)
            && let Some(step) = current_steps.last_mut()
        {
            step.table.push(row);
            step.span.end_line = line_num;
        } else if let Some(removed) = detect_removed_cjk_keyword(line) {
            // A removed Chinese keyword at a scenario/step/selector/tags
            // position must error, not silently vanish from the AST.
            return Err(cjk_keyword_error(removed, line_num));
        }
        // Ignore blank lines and non-step text inside scenarios
    }

    // Flush last scenario
    if let Some((name, start)) = current_name {
        let end = current_steps.last().map_or(start, |s| s.span.end_line);
        scenarios.push(Scenario {
            name,
            steps: current_steps,
            test_selector: finalize_test_selector(current_test_selector, end)?,
            tags: current_tags,
            review: current_review,
            mode: current_mode,
            depends_on: current_depends_on,
            span: Span::new(start, 0, end, 0),
        });
    }

    Ok(scenarios)
}

#[derive(Default)]
struct TestSelectorDraft {
    package: Option<String>,
    filter: Option<String>,
    level: Option<String>,
    test_double: Option<String>,
    targets: Option<String>,
}

fn finalize_test_selector(
    draft: Option<TestSelectorDraft>,
    line_num: usize,
) -> SpecResult<Option<TestSelector>> {
    let Some(draft) = draft else {
        return Ok(None);
    };

    let Some(filter) = draft.filter else {
        return Err(SpecError::Parse {
            message: "test selector is missing required `Filter:` field".into(),
            span: Span::line(line_num),
        });
    };

    Ok(Some(TestSelector {
        filter,
        package: draft.package,
        level: draft.level,
        test_double: draft.test_double,
        targets: draft.targets,
    }))
}

fn parse_table_row(line: &str) -> Option<Vec<String>> {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') {
        return None;
    }

    let row: Vec<String> = trimmed
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .map(String::from)
        .collect();

    if row.is_empty() || row.iter().all(|cell| cell.is_empty()) {
        None
    } else {
        Some(row)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::spec_core::StepKind;

    const SAMPLE_SPEC: &str = r#"spec: task
name: "退款功能"
inherits: project
tags: [payment, refund]
---

## Intent

为支付网关添加退款功能，支持全额和部分退款。

## Constraints

- 退款金额不得超过原始交易金额
- 退款操作需要管理员权限
- 退款必须在原交易后 90 天内发起

## Acceptance Criteria

Scenario: 全额退款
  Given 存在一笔金额为 "100.00" 元的已完成交易 "TXN-001"
  And 当前用户具有管理员权限
  When 用户对 "TXN-001" 发起全额退款
  Then 退款状态变为 "processing"
  And 原始交易状态变为 "refunding"

Scenario: 退款拒绝 - 超期
  Given 存在一笔 91 天前完成的交易 "TXN-003"
  When 用户对 "TXN-003" 发起退款
  Then 系统拒绝退款
  And 返回错误信息包含 "超过退款期限"

## Out of Scope

- 登录功能
- 密码重置
"#;

    #[test]
    fn test_parse_full_spec() {
        let doc = parse_spec_from_str(SAMPLE_SPEC).unwrap();

        assert_eq!(doc.meta.name, "退款功能");
        assert_eq!(doc.meta.level, crate::spec_core::SpecLevel::Task);
        assert_eq!(doc.meta.inherits, Some("project".into()));
        assert_eq!(doc.meta.tags, vec!["payment", "refund"]);

        // Should have 4 sections: intent, constraints, acceptance, out-of-scope
        assert_eq!(doc.sections.len(), 4);

        // Intent
        match &doc.sections[0] {
            Section::Intent { content, .. } => {
                assert!(content.contains("退款功能"));
            }
            other => panic!("expected Intent, got {other:?}"),
        }

        // Constraints
        match &doc.sections[1] {
            Section::Constraints { items, .. } => {
                assert_eq!(items.len(), 3);
                assert!(items[0].text.contains("退款金额"));
            }
            other => panic!("expected Constraints, got {other:?}"),
        }

        // Scenarios
        match &doc.sections[2] {
            Section::AcceptanceCriteria { scenarios, .. } => {
                assert_eq!(scenarios.len(), 2);

                let s1 = &scenarios[0];
                assert_eq!(s1.name, "全额退款");
                assert_eq!(s1.steps.len(), 5);
                assert_eq!(s1.steps[0].kind, StepKind::Given);
                assert_eq!(s1.steps[0].params, vec!["100.00", "TXN-001"]);
                assert_eq!(s1.steps[1].kind, StepKind::And);
                assert_eq!(s1.steps[2].kind, StepKind::When);
                assert_eq!(s1.steps[2].params, vec!["TXN-001"]);
                assert_eq!(s1.steps[3].kind, StepKind::Then);
                assert_eq!(s1.steps[4].kind, StepKind::And);

                let s2 = &scenarios[1];
                assert_eq!(s2.name, "退款拒绝 - 超期");
                assert_eq!(s2.steps.len(), 4);
            }
            other => panic!("expected AcceptanceCriteria, got {other:?}"),
        }

        // Out of scope
        match &doc.sections[3] {
            Section::OutOfScope { items, .. } => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[0], "登录功能");
            }
            other => panic!("expected OutOfScope, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_english_spec() {
        let input = r#"spec: task
name: "User Registration"
---

## Intent

Implement user registration API.

## Constraints

- Passwords must be hashed with bcrypt
- Email must be unique

## Acceptance Criteria

Scenario: Successful registration
  Given no user with email "alice@example.com" exists
  When POST /api/v1/auth/register with email "alice@example.com"
  Then response status should be 201
  And response body should contain "id"
"#;
        let doc = parse_spec_from_str(input).unwrap();
        assert_eq!(doc.meta.name, "User Registration");
        assert_eq!(doc.sections.len(), 3);

        match &doc.sections[2] {
            Section::AcceptanceCriteria { scenarios, .. } => {
                assert_eq!(scenarios.len(), 1);
                assert_eq!(scenarios[0].name, "Successful registration");
                assert_eq!(scenarios[0].steps.len(), 4);
                assert_eq!(scenarios[0].steps[0].params, vec!["alice@example.com"]);
            }
            other => panic!("expected AcceptanceCriteria, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_english_keyword_chinese_description() {
        // Core regression: English keywords + Chinese description + a code
        // identifier parse, with the Chinese text and identifier preserved.
        let input = r#"spec: task
name: "scope 推导"
---

## Completion Criteria

Scenario: 数据 scope 由角色变体推导
  Given 平台角色
  When scope_of 映射每个角色
  Then 平台角色得到 Scope::All
"#;
        let doc = parse_spec_from_str(input).unwrap();
        match &doc.sections[0] {
            Section::AcceptanceCriteria { scenarios, .. } => {
                let s = &scenarios[0];
                assert_eq!(s.name, "数据 scope 由角色变体推导");
                assert_eq!(s.steps.len(), 3);
                assert_eq!(s.steps[0].kind, StepKind::Given);
                assert_eq!(s.steps[0].text, "平台角色");
                assert_eq!(s.steps[1].kind, StepKind::When);
                assert_eq!(s.steps[1].text, "scope_of 映射每个角色");
                assert_eq!(s.steps[2].kind, StepKind::Then);
                assert_eq!(s.steps[2].text, "平台角色得到 Scope::All");
            }
            other => panic!("expected AcceptanceCriteria, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_step_table_and_preserve_json_output() {
        let input = r#"spec: task
name: "表格测试"
---

## Acceptance Criteria

Scenario: 注册请求
  When 发送 POST /api/v1/auth/register 请求:
    | field    | value             |
    | email    | alice@example.com |
    | password | Str0ng!Pass#2024  |
  Then 响应状态码应为 201
"#;

        let doc = parse_spec_from_str(input).unwrap();
        match &doc.sections[0] {
            Section::AcceptanceCriteria { scenarios, .. } => {
                let when_step = &scenarios[0].steps[0];
                assert_eq!(when_step.kind, StepKind::When);
                assert_eq!(when_step.table.len(), 3);
                assert_eq!(when_step.table[0], vec!["field", "value"]);
                assert_eq!(when_step.table[1], vec!["email", "alice@example.com"]);
            }
            other => panic!("expected AcceptanceCriteria, got {other:?}"),
        }

        let json = serde_json::to_string_pretty(&doc).unwrap();
        assert!(json.contains("\"table\""));
        assert!(json.contains("alice@example.com"));
        assert!(json.contains("Str0ng!Pass#2024"));
    }

    #[test]
    fn test_parse_scenario_without_table_stays_unchanged() {
        let input = r#"spec: task
name: "普通场景"
---

## Acceptance Criteria

Scenario: 无表格
  Given 用户已登录
  When 用户点击提交
  Then 页面显示成功
"#;

        let doc = parse_spec_from_str(input).unwrap();
        match &doc.sections[0] {
            Section::AcceptanceCriteria { scenarios, .. } => {
                let scenario = &scenarios[0];
                assert_eq!(scenario.steps.len(), 3);
                assert!(scenario.steps.iter().all(|step| step.table.is_empty()));
                assert_eq!(scenario.steps[1].text, "用户点击提交");
            }
            other => panic!("expected AcceptanceCriteria, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_task_contract_sections() {
        let input = r#"spec: task
name: "Contract"
---

## Intent

Implement the task safely.

## Decisions

- Use existing parser module

## Boundaries

### Allowed Changes
- crates/spec-parser/**

### Forbidden
- Do not modify crates/spec-verify/**

## Completion Criteria

Scenario: Parse succeeds
  Given a valid contract
  When the parser reads it
  Then the parser should succeed
"#;

        let doc = parse_spec_from_str(input).unwrap();
        assert_eq!(doc.sections.len(), 4);

        match &doc.sections[1] {
            Section::Decisions { items, .. } => {
                assert_eq!(items, &vec!["Use existing parser module".to_string()]);
            }
            other => panic!("expected Decisions, got {other:?}"),
        }

        match &doc.sections[2] {
            Section::Boundaries { items, .. } => {
                assert_eq!(items.len(), 2);
                assert_eq!(items[0].category, BoundaryCategory::Allow);
                assert_eq!(items[1].category, BoundaryCategory::Deny);
            }
            other => panic!("expected Boundaries, got {other:?}"),
        }

        match &doc.sections[3] {
            Section::AcceptanceCriteria { scenarios, .. } => {
                assert_eq!(scenarios.len(), 1);
            }
            other => panic!("expected AcceptanceCriteria, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_scenario_with_explicit_test_selector() {
        let input = r#"spec: task
name: "绑定测试"
---

## Completion Criteria

Scenario: 显式绑定
  Test: test_parse_scenario_with_explicit_test_selector
  Given 某个场景声明测试选择器
  When parser 解析该场景
  Then AST 中保留该 selector
"#;

        let doc = parse_spec_from_str(input).unwrap();
        match &doc.sections[0] {
            Section::AcceptanceCriteria { scenarios, .. } => {
                assert_eq!(scenarios.len(), 1);
                assert_eq!(
                    scenarios[0]
                        .test_selector
                        .as_ref()
                        .map(|selector| selector.filter.as_str()),
                    Some("test_parse_scenario_with_explicit_test_selector")
                );
                assert_eq!(scenarios[0].steps.len(), 3);
            }
            other => panic!("expected AcceptanceCriteria, got {other:?}"),
        }

        let json = serde_json::to_string_pretty(&doc).unwrap();
        assert!(json.contains("\"test_selector\""));
        assert!(json.contains("\"filter\""));
        assert!(json.contains("test_parse_scenario_with_explicit_test_selector"));
    }

    #[test]
    fn test_parse_structured_test_selector_block() {
        let input = r#"spec: task
name: "结构化绑定"
---

## Completion Criteria

Scenario: 结构化绑定
  Test:
    Package: spec-parser
    Filter: test_parse_structured_test_selector_block
  Given 某个场景声明结构化测试选择器
  When parser 解析该场景
  Then AST 中保留结构化字段
"#;

        let doc = parse_spec_from_str(input).unwrap();
        match &doc.sections[0] {
            Section::AcceptanceCriteria { scenarios, .. } => {
                assert_eq!(scenarios.len(), 1);
                let selector = scenarios[0].test_selector.as_ref().unwrap();
                assert_eq!(selector.package.as_deref(), Some("spec-parser"));
                assert_eq!(selector.filter, "test_parse_structured_test_selector_block");
            }
            other => panic!("expected AcceptanceCriteria, got {other:?}"),
        }

        let json = serde_json::to_string_pretty(&doc).unwrap();
        assert!(json.contains("\"package\""));
        assert!(json.contains("\"spec-parser\""));
        assert!(json.contains("\"filter\""));
        assert!(json.contains("test_parse_structured_test_selector_block"));
    }

    #[test]
    fn test_parse_scenario_verification_metadata_fields() {
        let input = r#"spec: task
name: "验证元数据"
---

## Completion Criteria

Scenario: 结构化验证强度
  Test:
    Package: specwright
    Filter: test_parse_scenario_verification_metadata_fields
    Level: integration
    Test Double: local_http_stub
    Targets: commands/update
  Given 某个场景声明验证元数据
  When parser 解析该场景
  Then AST 中保留这些字段
"#;

        let doc = parse_spec_from_str(input).unwrap();
        match &doc.sections[0] {
            Section::AcceptanceCriteria { scenarios, .. } => {
                let selector = scenarios[0].test_selector.as_ref().unwrap();
                assert_eq!(selector.package.as_deref(), Some("specwright"));
                assert_eq!(
                    selector.filter,
                    "test_parse_scenario_verification_metadata_fields"
                );
                assert_eq!(selector.level.as_deref(), Some("integration"));
                assert_eq!(selector.test_double.as_deref(), Some("local_http_stub"));
                assert_eq!(selector.targets.as_deref(), Some("commands/update"));
            }
            other => panic!("expected AcceptanceCriteria, got {other:?}"),
        }

        let json = serde_json::to_string_pretty(&doc).unwrap();
        assert!(json.contains("\"level\""));
        assert!(json.contains("\"test_double\""));
        assert!(json.contains("\"targets\""));
    }

    #[test]
    fn test_parse_english_verification_metadata_fields() {
        let input = r#"spec: task
name: "verification metadata"
---

## Completion Criteria

Scenario: verification metadata
  Test:
    Package: specwright
    Filter: test_parse_english_verification_metadata_fields
    Level: integration
    Test Double: local_http_stub
    Targets: commands/update
  Given a scenario declares verification metadata
  When the parser reads it
  Then the AST keeps the metadata
"#;

        let doc = parse_spec_from_str(input).unwrap();
        match &doc.sections[0] {
            Section::AcceptanceCriteria { scenarios, .. } => {
                let selector = scenarios[0].test_selector.as_ref().unwrap();
                assert_eq!(selector.level.as_deref(), Some("integration"));
                assert_eq!(selector.test_double.as_deref(), Some("local_http_stub"));
                assert_eq!(selector.targets.as_deref(), Some("commands/update"));
            }
            other => panic!("expected AcceptanceCriteria, got {other:?}"),
        }
    }

    #[test]
    fn test_existing_specs_without_verification_metadata_remain_valid() {
        let input = r#"spec: task
name: "legacy selector"
---

## Completion Criteria

Scenario: legacy selector
  Test:
    Package: specwright
    Filter: test_existing_specs_without_verification_metadata_remain_valid
  Given a legacy spec
  When the parser reads it
  Then the selector remains valid
"#;

        let doc = parse_spec_from_str(input).unwrap();
        match &doc.sections[0] {
            Section::AcceptanceCriteria { scenarios, .. } => {
                let selector = scenarios[0].test_selector.as_ref().unwrap();
                assert_eq!(selector.package.as_deref(), Some("specwright"));
                assert_eq!(
                    selector.filter,
                    "test_existing_specs_without_verification_metadata_remain_valid"
                );
                assert_eq!(selector.level, None);
                assert_eq!(selector.test_double, None);
                assert_eq!(selector.targets, None);
            }
            other => panic!("expected AcceptanceCriteria, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_shorthand_test_selector_as_filter_only() {
        let input = r#"spec: task
name: "单行绑定"
---

## Completion Criteria

Scenario: 单行绑定
  Test: test_parse_shorthand_test_selector_as_filter_only
  Given 某个场景继续使用单行测试绑定
  When parser 解析该场景
  Then filter 字段被保留
"#;

        let doc = parse_spec_from_str(input).unwrap();
        match &doc.sections[0] {
            Section::AcceptanceCriteria { scenarios, .. } => {
                let selector = scenarios[0].test_selector.as_ref().unwrap();
                assert_eq!(
                    selector.filter,
                    "test_parse_shorthand_test_selector_as_filter_only"
                );
                assert_eq!(selector.package, None);
            }
            other => panic!("expected AcceptanceCriteria, got {other:?}"),
        }
    }

    #[test]
    fn test_missing_front_matter() {
        let input = "## Intent\nSome content\n";
        let result = parse_spec_from_str(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_unknown_top_level_section_header_is_rejected() {
        let input = r#"spec: task
name: "未知章节"
---

## Intent

Describe the task.

## Milestones

- phase 1
"#;

        let err = parse_spec_from_str(input).unwrap_err();
        match err {
            SpecError::Parse { message, span } => {
                assert!(message.contains("unknown top-level section header"));
                assert_eq!(span.start_line, 9);
            }
            other => panic!("expected parse error, got {other:?}"),
        }
    }

    #[test]
    fn test_markdown_heading_scenarios_and_test_selectors_are_accepted() {
        let input = r#"spec: task
name: "Markdown Scenario"
---

## Completion Criteria

### Scenario: Happy path
  ### Test: test_markdown_heading_scenarios_and_test_selectors_are_accepted
  Given valid input
  When parser reads the scenario
  Then the scenario is preserved
"#;

        let doc = parse_spec_from_str(input).unwrap();
        match &doc.sections[0] {
            Section::AcceptanceCriteria { scenarios, .. } => {
                assert_eq!(scenarios.len(), 1);
                assert_eq!(scenarios[0].name, "Happy path");
                assert_eq!(
                    scenarios[0]
                        .test_selector
                        .as_ref()
                        .map(|selector| selector.filter.as_str()),
                    Some("test_markdown_heading_scenarios_and_test_selectors_are_accepted")
                );
            }
            other => panic!("expected AcceptanceCriteria, got {other:?}"),
        }
    }

    #[test]
    fn test_serialization_roundtrip() {
        let doc = parse_spec_from_str(SAMPLE_SPEC).unwrap();
        let json = serde_json::to_string_pretty(&doc).unwrap();
        let _: SpecDocument = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_parse_mode_field_in_scenario() {
        let input = r#"spec: task
name: "模式测试"
---

## Completion Criteria

Scenario: 优化场景
  Mode: optimize
  Test: test_parse_mode_field_in_scenario
  Given 某个场景声明 optimize 模式
  When parser 解析该场景
  Then AST 中 mode 字段为 Optimize
"#;
        let doc = parse_spec_from_str(input).unwrap();
        match &doc.sections[0] {
            Section::AcceptanceCriteria { scenarios, .. } => {
                assert_eq!(scenarios.len(), 1);
                assert_eq!(scenarios[0].mode, crate::spec_core::ScenarioMode::Optimize);
            }
            other => panic!("expected AcceptanceCriteria, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_mode_field_english() {
        let input = r#"spec: task
name: "mode test"
---

## Completion Criteria

Scenario: optimize scenario
  Mode: optimize
  Given an optimize-mode scenario
  When parser reads it
  Then mode is Optimize
"#;
        let doc = parse_spec_from_str(input).unwrap();
        match &doc.sections[0] {
            Section::AcceptanceCriteria { scenarios, .. } => {
                assert_eq!(scenarios[0].mode, crate::spec_core::ScenarioMode::Optimize);
            }
            other => panic!("expected AcceptanceCriteria, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_mode_field_standard_is_default() {
        let input = r#"spec: task
name: "default mode"
---

## Completion Criteria

Scenario: standard scenario
  Mode: standard
  Given a standard scenario
  When parser reads it
  Then mode is Standard

Scenario: no mode declared
  Given no mode field
  When parser reads it
  Then mode defaults to Standard
"#;
        let doc = parse_spec_from_str(input).unwrap();
        match &doc.sections[0] {
            Section::AcceptanceCriteria { scenarios, .. } => {
                assert_eq!(scenarios[0].mode, crate::spec_core::ScenarioMode::Standard);
                assert_eq!(scenarios[1].mode, crate::spec_core::ScenarioMode::Standard);
            }
            other => panic!("expected AcceptanceCriteria, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_depends_field_in_scenario() {
        let input = r#"spec: task
name: "依赖测试"
---

## Completion Criteria

Scenario: 用户注册
  Given 注册表单已打开
  When 用户提交注册
  Then 注册成功

Scenario: 用户登录
  Depends: 用户注册
  Given 已有注册用户
  When 用户登录
  Then 登录成功
"#;
        let doc = parse_spec_from_str(input).unwrap();
        match &doc.sections[0] {
            Section::AcceptanceCriteria { scenarios, .. } => {
                assert_eq!(scenarios.len(), 2);
                assert!(scenarios[0].depends_on.is_empty());
                assert_eq!(scenarios[1].depends_on, vec!["用户注册"]);
            }
            other => panic!("expected AcceptanceCriteria, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_depends_field_multiple() {
        let input = r#"spec: task
name: "multi depends"
---

## Completion Criteria

Scenario: A
  Given A
  When A
  Then A

Scenario: B
  Given B
  When B
  Then B

Scenario: C
  Depends: A, B
  Given C depends on A and B
  When parser reads it
  Then depends_on contains both
"#;
        let doc = parse_spec_from_str(input).unwrap();
        match &doc.sections[0] {
            Section::AcceptanceCriteria { scenarios, .. } => {
                assert_eq!(scenarios[2].depends_on, vec!["A", "B"]);
            }
            other => panic!("expected AcceptanceCriteria, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_review_field_in_scenario() {
        let input = r#"spec: task
name: "审核测试"
---

## Completion Criteria

Scenario: 需要人类审核
  Review: human
  Test: test_parse_review_field_in_scenario
  Given 某个场景声明审核为 human
  When parser 解析该场景
  Then AST 中 review 字段为 Human

Scenario: 默认自动审核
  Test: test_default_auto_review
  Given 某个场景不声明审核字段
  When parser 解析该场景
  Then AST 中 review 字段为 Auto
"#;

        let doc = parse_spec_from_str(input).unwrap();
        match &doc.sections[0] {
            Section::AcceptanceCriteria { scenarios, .. } => {
                assert_eq!(scenarios.len(), 2);
                assert_eq!(
                    scenarios[0].review,
                    crate::spec_core::ReviewMode::Human,
                    "scenario with '审核: human' should have ReviewMode::Human"
                );
                assert_eq!(
                    scenarios[1].review,
                    crate::spec_core::ReviewMode::Auto,
                    "scenario without review field should default to ReviewMode::Auto"
                );
            }
            other => panic!("expected AcceptanceCriteria, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_rejects_cjk_section_header() {
        let input = "spec: task\nname: \"t\"\n---\n\n## 意图\n\nx\n";
        match parse_spec_from_str(input) {
            Err(SpecError::Parse { message, .. }) => {
                assert!(message.contains("意图"), "message: {message}");
                assert!(
                    message.contains("Intent"),
                    "should name the allowed English section set: {message}"
                );
            }
            other => panic!("expected Parse error, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_rejects_cjk_scenario_header() {
        let input =
            "spec: task\nname: \"t\"\n---\n\n## Completion Criteria\n\n场景: 全额退款\n  Given x\n";
        match parse_spec_from_str(input) {
            Err(SpecError::Parse { message, .. }) => {
                assert!(message.contains("场景:"), "message: {message}");
                assert!(message.contains("Scenario:"), "message: {message}");
            }
            other => panic!("expected Parse error, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_rejects_cjk_step_keyword() {
        let input = "spec: task\nname: \"t\"\n---\n\n## Completion Criteria\n\nScenario: s\n  假设 用户已登录\n";
        match parse_spec_from_str(input) {
            Err(SpecError::Parse { message, .. }) => {
                assert!(message.contains("假设"), "message: {message}");
                assert!(message.contains("Given"), "message: {message}");
            }
            other => panic!("expected Parse error, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_rejects_cjk_test_selector() {
        let input = "spec: task\nname: \"t\"\n---\n\n## Completion Criteria\n\nScenario: s\n  测试: test_refund\n  Given x\n";
        match parse_spec_from_str(input) {
            Err(SpecError::Parse { message, .. }) => {
                assert!(message.contains("测试:"), "message: {message}");
                assert!(message.contains("Test:"), "message: {message}");
            }
            other => panic!("expected Parse error, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_rejects_cjk_boundary_subheading() {
        let input = "spec: task\nname: \"t\"\n---\n\n## Boundaries\n\n### 允许修改\n- src/x\n";
        match parse_spec_from_str(input) {
            Err(SpecError::Parse { message, .. }) => {
                assert!(message.contains("允许修改"), "message: {message}");
                assert!(message.contains("Allowed Changes"), "message: {message}");
            }
            other => panic!("expected Parse error, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_rejects_cjk_constraint_subheading() {
        let input = "spec: task\nname: \"t\"\n---\n\n## Constraints\n\n### 禁止做\n- 不要做 x\n";
        match parse_spec_from_str(input) {
            Err(SpecError::Parse { message, .. }) => {
                assert!(message.contains("禁止做"), "message: {message}");
            }
            other => panic!("expected Parse error, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_keeps_chinese_description_bullet() {
        // 当 and 场景 here are prose words, not structural keywords; the bullet
        // must parse and be preserved as a Chinese description, not rejected.
        let input = "spec: task\nname: \"t\"\n---\n\n## Decisions\n\n- 当 critical 场景 fail 时退出码为 2\n";
        let doc = parse_spec_from_str(input).unwrap();
        match &doc.sections[0] {
            Section::Decisions { items, .. } => {
                assert_eq!(items.len(), 1);
                assert_eq!(items[0], "当 critical 场景 fail 时退出码为 2");
            }
            other => panic!("expected Decisions, got {other:?}"),
        }
    }
}
