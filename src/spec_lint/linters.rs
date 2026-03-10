use crate::spec_core::{
    LintDiagnostic, Section, Severity, Span, SpecDocument, SpecLevel, StepKind,
};

use super::pipeline::SpecLinter;

// =============================================================================
// 1. VagueVerbLinter - detects vague/imprecise verbs in constraints and steps
// =============================================================================

pub struct VagueVerbLinter;

const VAGUE_VERBS_ZH: &[&str] = &["处理", "管理", "支持", "优化", "改善", "增强", "完善"];

const VAGUE_VERBS_EN: &[&str] = &[
    "handle", "manage", "support", "process", "optimize", "improve", "enhance",
];

impl SpecLinter for VagueVerbLinter {
    fn name(&self) -> &str {
        "vague-verb"
    }

    fn lint(&self, doc: &SpecDocument) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();

        for section in &doc.sections {
            match section {
                Section::Constraints { items, .. } => {
                    for c in items {
                        if let Some(verb) = find_vague_verb(&c.text) {
                            diags.push(LintDiagnostic {
                                rule: "vague-verb".into(),
                                severity: Severity::Warning,
                                message: format!(
                                    "constraint uses vague verb '{verb}' - use precise verbs like create/delete/validate"
                                ),
                                span: c.span,
                                suggestion: Some(
                                    "replace with specific action: 创建/删除/校验/查询 or create/delete/validate/query".into(),
                                ),
                            });
                        }
                    }
                }
                Section::Intent { content, span } => {
                    if let Some(verb) = find_vague_verb(content) {
                        diags.push(LintDiagnostic {
                            rule: "vague-verb".into(),
                            severity: Severity::Info,
                            message: format!(
                                "intent uses vague verb '{verb}' - consider being more specific"
                            ),
                            span: *span,
                            suggestion: None,
                        });
                    }
                }
                _ => {}
            }
        }

        diags
    }
}

fn find_vague_verb(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    for &v in VAGUE_VERBS_ZH {
        if text.contains(v) {
            return Some(v.to_string());
        }
    }
    for &v in VAGUE_VERBS_EN {
        if lower.contains(v) {
            return Some(v.to_string());
        }
    }
    None
}

// =============================================================================
// 2. UnquantifiedLinter - detects constraints without measurable values
// =============================================================================

pub struct UnquantifiedLinter;

const VAGUE_QUALIFIERS_ZH: &[&str] = &["快速", "高效", "及时", "合理", "适当", "足够", "良好"];

const VAGUE_QUALIFIERS_EN: &[&str] = &[
    "fast",
    "efficient",
    "timely",
    "reasonable",
    "appropriate",
    "sufficient",
    "good",
    "quickly",
];

impl SpecLinter for UnquantifiedLinter {
    fn name(&self) -> &str {
        "unquantified"
    }

    fn lint(&self, doc: &SpecDocument) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();

        for section in &doc.sections {
            if let Section::Constraints { items, .. } = section {
                for c in items {
                    if let Some(qualifier) = find_vague_qualifier(&c.text) {
                        diags.push(LintDiagnostic {
                            rule: "unquantified".into(),
                            severity: Severity::Warning,
                            message: format!(
                                "constraint uses vague qualifier '{qualifier}' without a measurable value"
                            ),
                            span: c.span,
                            suggestion: Some(
                                "add a measurable threshold: e.g., '< 200ms', '>= 80%', '不超过 5 次'".into(),
                            ),
                        });
                    }
                }
            }
        }

        diags
    }
}

fn find_vague_qualifier(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    for &q in VAGUE_QUALIFIERS_ZH {
        if text.contains(q) {
            return Some(q.to_string());
        }
    }
    for &q in VAGUE_QUALIFIERS_EN {
        if lower.contains(q) {
            return Some(q.to_string());
        }
    }
    None
}

// =============================================================================
// 3. TestabilityLinter - checks if Then steps are mechanically verifiable
// =============================================================================

pub struct TestabilityLinter;

const UNTESTABLE_ZH: &[&str] = &["美观", "友好", "直观", "舒适", "合适", "自然"];

const UNTESTABLE_EN: &[&str] = &[
    "beautiful",
    "user-friendly",
    "intuitive",
    "comfortable",
    "natural",
    "clean",
    "nice",
];

impl SpecLinter for TestabilityLinter {
    fn name(&self) -> &str {
        "testability"
    }

    fn lint(&self, doc: &SpecDocument) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();

        for section in &doc.sections {
            if let Section::AcceptanceCriteria { scenarios, .. } = section {
                for scenario in scenarios {
                    for step in &scenario.steps {
                        if (step.kind == StepKind::Then || step.kind == StepKind::And)
                            && let Some(term) = find_untestable_term(&step.text)
                        {
                            diags.push(LintDiagnostic {
                                rule: "testability".into(),
                                severity: Severity::Warning,
                                message: format!(
                                    "step uses subjective term '{term}' that cannot be mechanically verified"
                                ),
                                span: step.span,
                                suggestion: Some(
                                    "replace with a measurable assertion: score >= 90, contains 'X', status == 200".into(),
                                ),
                            });
                        }
                    }
                }
            }
        }

        diags
    }
}

fn find_untestable_term(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    for &t in UNTESTABLE_ZH {
        if text.contains(t) {
            return Some(t.to_string());
        }
    }
    for &t in UNTESTABLE_EN {
        if lower.contains(t) {
            return Some(t.to_string());
        }
    }
    None
}

// =============================================================================
// 4. CoverageLinter - checks if constraints are covered by scenarios
// =============================================================================

pub struct CoverageLinter;

impl SpecLinter for CoverageLinter {
    fn name(&self) -> &str {
        "coverage"
    }

    fn lint(&self, doc: &SpecDocument) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();

        let all_step_text: Vec<&str> = doc
            .sections
            .iter()
            .filter_map(|s| match s {
                Section::AcceptanceCriteria { scenarios, .. } => Some(
                    scenarios
                        .iter()
                        .flat_map(|sc| sc.steps.iter().map(|st| st.text.as_str())),
                ),
                _ => None,
            })
            .flatten()
            .collect();

        for section in &doc.sections {
            if let Section::Constraints { items, .. } = section {
                for c in items {
                    let keywords = extract_keywords(&c.text);
                    let covered = keywords.iter().any(|kw| {
                        all_step_text
                            .iter()
                            .any(|step| step.to_lowercase().contains(&kw.to_lowercase()))
                    });

                    if !covered && !keywords.is_empty() {
                        diags.push(LintDiagnostic {
                            rule: "coverage".into(),
                            severity: Severity::Warning,
                            message: format!(
                                "constraint '{}' has no matching scenario step",
                                truncate(&c.text, 60),
                            ),
                            span: c.span,
                            suggestion: Some("add a scenario that verifies this constraint".into()),
                        });
                    }
                }
            }
        }

        diags
    }
}

fn extract_keywords(text: &str) -> Vec<String> {
    let stop_words = [
        "的", "是", "在", "了", "和", "与", "或", "为", "被", "将", "不", "应", "必须", "使用",
        "所有", "每个", "a", "the", "is", "are", "must", "should", "all", "be", "to", "in", "of",
        "and", "or", "not", "no", "with", "for", "by",
    ];

    text.split(|c: char| c.is_whitespace() || c == ',' || c == '、' || c == '。')
        .filter(|w| {
            let w_lower = w.to_lowercase();
            w.len() > 1 && !stop_words.iter().any(|sw| w_lower == *sw)
        })
        .map(String::from)
        .collect()
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max - 3).collect();
        format!("{truncated}...")
    }
}

// =============================================================================
// 5. DeterminismLinter
// =============================================================================

pub struct DeterminismLinter;

const NONDETERMINISTIC_ZH: &[&str] = &["大约", "大概", "可能", "也许", "随机", "有时"];

const NONDETERMINISTIC_EN: &[&str] = &[
    "approximately",
    "roughly",
    "maybe",
    "possibly",
    "random",
    "sometimes",
    "might",
    "could",
    "about",
];

impl SpecLinter for DeterminismLinter {
    fn name(&self) -> &str {
        "determinism"
    }

    fn lint(&self, doc: &SpecDocument) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();

        for section in &doc.sections {
            if let Section::AcceptanceCriteria { scenarios, .. } = section {
                for scenario in scenarios {
                    for step in &scenario.steps {
                        if let Some(term) = find_nondeterministic(&step.text) {
                            diags.push(LintDiagnostic {
                                rule: "determinism".into(),
                                severity: Severity::Warning,
                                message: format!(
                                    "step uses non-deterministic term '{term}' - outcomes should be exact"
                                ),
                                span: step.span,
                                suggestion: Some(
                                    "use exact values: '== 100', 'contains X', 'status is 200'".into(),
                                ),
                            });
                        }
                    }
                }
            }
        }

        diags
    }
}

fn find_nondeterministic(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    for &t in NONDETERMINISTIC_ZH {
        if text.contains(t) {
            return Some(t.to_string());
        }
    }
    for &t in NONDETERMINISTIC_EN {
        if lower.contains(t) {
            return Some(t.to_string());
        }
    }
    None
}

// =============================================================================
// 6. ImplicitDepLinter
// =============================================================================

pub struct ImplicitDepLinter;

impl SpecLinter for ImplicitDepLinter {
    fn name(&self) -> &str {
        "implicit-dep"
    }

    fn lint(&self, doc: &SpecDocument) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();

        for section in &doc.sections {
            if let Section::AcceptanceCriteria { scenarios, .. } = section {
                for scenario in scenarios {
                    let given_entities: Vec<String> = scenario
                        .steps
                        .iter()
                        .filter(|s| s.kind == StepKind::Given || s.kind == StepKind::And)
                        .flat_map(|s| s.params.clone())
                        .collect();

                    let mut seen_when = false;
                    for step in &scenario.steps {
                        if step.kind == StepKind::When {
                            seen_when = true;
                        }
                        if seen_when {
                            for param in &step.params {
                                if !given_entities.contains(param) && !is_likely_literal(param) {
                                    diags.push(LintDiagnostic {
                                        rule: "implicit-dep".into(),
                                        severity: Severity::Info,
                                        message: format!(
                                            "parameter '{param}' referenced but not defined in Given steps"
                                        ),
                                        span: step.span,
                                        suggestion: Some(
                                            "add a Given step that establishes this value".into(),
                                        ),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        diags
    }
}

fn is_likely_literal(value: &str) -> bool {
    value.parse::<f64>().is_ok()
        || value.chars().all(|c| c.is_ascii_digit() || c == '.')
        || value.starts_with("http")
        || value.contains('@')
}

// =============================================================================
// 7. ExplicitTestBindingLinter
// =============================================================================

pub struct ExplicitTestBindingLinter;

impl SpecLinter for ExplicitTestBindingLinter {
    fn name(&self) -> &str {
        "explicit-test-binding"
    }

    fn lint(&self, doc: &SpecDocument) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();

        if doc.meta.level != SpecLevel::Task {
            return diags;
        }

        for section in &doc.sections {
            if let Section::AcceptanceCriteria { scenarios, .. } = section {
                for scenario in scenarios {
                    if scenario.test_selector.is_none() {
                        diags.push(LintDiagnostic {
                            rule: "explicit-test-binding".into(),
                            severity: Severity::Error,
                            message: format!(
                                "scenario '{}' is missing an explicit test selector",
                                scenario.name
                            ),
                            span: scenario.span,
                            suggestion: Some(
                                "add `测试: test_name` or `Test: test_name` directly under the scenario header".into(),
                            ),
                        });
                    }
                }
            }
        }

        diags
    }
}

// =============================================================================
// 8. ScenarioPresenceLinter
// =============================================================================

pub struct ScenarioPresenceLinter;

impl SpecLinter for ScenarioPresenceLinter {
    fn name(&self) -> &str {
        "scenario-presence"
    }

    fn lint(&self, doc: &SpecDocument) -> Vec<LintDiagnostic> {
        if doc.meta.level != SpecLevel::Task {
            return Vec::new();
        }

        let acceptance_sections: Vec<_> = doc
            .sections
            .iter()
            .filter_map(|section| match section {
                Section::AcceptanceCriteria { scenarios, span } => Some((scenarios, span)),
                _ => None,
            })
            .collect();

        if acceptance_sections.is_empty() {
            return vec![LintDiagnostic {
                rule: "scenario-presence".into(),
                severity: Severity::Error,
                message: "task spec is missing an Acceptance Criteria / Completion Criteria section"
                    .into(),
                span: crate::spec_core::Span::line(0),
                suggestion: Some(
                    "add `## 验收标准` / `## Completion Criteria` with at least one `场景:` / `Scenario:` block".into(),
                ),
            }];
        }

        let scenario_count = acceptance_sections
            .iter()
            .map(|(scenarios, _)| scenarios.len())
            .sum::<usize>();

        if scenario_count == 0 {
            return vec![LintDiagnostic {
                rule: "scenario-presence".into(),
                severity: Severity::Error,
                message:
                    "task spec has an Acceptance Criteria section but no parseable scenarios".into(),
                span: *acceptance_sections[0].1,
                suggestion: Some(
                    "write scenarios using bare `场景:` / `Scenario:` lines, or run `agent-spec parse` to inspect the AST".into(),
                ),
            }];
        }

        Vec::new()
    }
}

// =============================================================================
// 9. SycophancyLinter
// =============================================================================

pub struct SycophancyLinter;

const SYCOPHANCY_ZH: &[&str] = &[
    "找出所有",
    "必须找到",
    "尽可能多地发现",
    "不要遗漏任何",
    "确保发现所有",
];

const SYCOPHANCY_EN: &[&str] = &[
    "find all bugs",
    "find every bug",
    "must find",
    "discover as many",
    "do not miss any",
    "ensure you find all",
    "catch all issues",
    "identify all problems",
    "find all issues",
];

impl SpecLinter for SycophancyLinter {
    fn name(&self) -> &str {
        "sycophancy"
    }

    fn lint(&self, doc: &SpecDocument) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();

        for section in &doc.sections {
            let (texts, span): (Vec<&str>, _) = match section {
                Section::Intent { content, span } => (vec![content.as_str()], *span),
                Section::Constraints { items, span } => {
                    (items.iter().map(|c| c.text.as_str()).collect(), *span)
                }
                Section::AcceptanceCriteria { scenarios, span } => {
                    let texts: Vec<&str> = scenarios
                        .iter()
                        .flat_map(|s| s.steps.iter().map(|st| st.text.as_str()))
                        .collect();
                    (texts, *span)
                }
                _ => continue,
            };

            for text in texts {
                if let Some(phrase) = find_sycophancy_phrase(text) {
                    diags.push(LintDiagnostic {
                        rule: "sycophancy".into(),
                        severity: Severity::Warning,
                        message: format!(
                            "spec uses bug-finding bias phrase '{phrase}' which may induce sycophantic AI behavior"
                        ),
                        span,
                        suggestion: Some(
                            "use neutral language: 'verify the contract holds' instead of 'find all bugs'".into(),
                        ),
                    });
                }
            }
        }

        diags
    }
}

fn find_sycophancy_phrase(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    for &p in SYCOPHANCY_ZH {
        if text.contains(p) {
            return Some(p.to_string());
        }
    }
    for &p in SYCOPHANCY_EN {
        if lower.contains(p) {
            return Some(p.to_string());
        }
    }
    None
}

// =============================================================================
// 10. DecisionCoverageLinter - checks if decisions are covered by scenarios
// =============================================================================

pub struct DecisionCoverageLinter;

impl SpecLinter for DecisionCoverageLinter {
    fn name(&self) -> &str {
        "decision-coverage"
    }

    fn lint(&self, doc: &SpecDocument) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();

        // Only check task-level specs (decisions in project specs are inherited)
        if doc.meta.level != SpecLevel::Task {
            return diags;
        }

        // Collect all step text from scenarios
        let all_step_text: Vec<&str> = doc
            .sections
            .iter()
            .filter_map(|s| match s {
                Section::AcceptanceCriteria { scenarios, .. } => Some(
                    scenarios
                        .iter()
                        .flat_map(|sc| sc.steps.iter().map(|st| st.text.as_str())),
                ),
                _ => None,
            })
            .flatten()
            .collect();

        // Also collect scenario names (decisions are often reflected in scenario names)
        let scenario_names: Vec<&str> = doc
            .sections
            .iter()
            .filter_map(|s| match s {
                Section::AcceptanceCriteria { scenarios, .. } => {
                    Some(scenarios.iter().map(|sc| sc.name.as_str()))
                }
                _ => None,
            })
            .flatten()
            .collect();

        for section in &doc.sections {
            if let Section::Decisions { items, span } = section {
                for (i, decision) in items.iter().enumerate() {
                    let keywords = extract_decision_keywords(decision);
                    if keywords.is_empty() {
                        continue;
                    }

                    let covered_by_step = keywords.iter().any(|kw| {
                        all_step_text
                            .iter()
                            .any(|step| step.to_lowercase().contains(&kw.to_lowercase()))
                    });

                    let covered_by_name = keywords.iter().any(|kw| {
                        scenario_names
                            .iter()
                            .any(|name| name.to_lowercase().contains(&kw.to_lowercase()))
                    });

                    if !covered_by_step && !covered_by_name {
                        diags.push(LintDiagnostic {
                            rule: "decision-coverage".into(),
                            severity: Severity::Warning,
                            message: format!(
                                "decision '{}' has no matching scenario",
                                truncate(decision, 60),
                            ),
                            span: Span::new(span.start_line + i + 1, 0, span.start_line + i + 1, 0),
                            suggestion: Some(
                                "add a scenario that verifies this decision is implemented correctly".into(),
                            ),
                        });
                    }
                }
            }
        }

        diags
    }
}

/// Extract meaningful keywords from a decision text, filtering common verbs and articles.
fn extract_decision_keywords(text: &str) -> Vec<String> {
    let stop_words = [
        // Chinese
        "的", "是", "在", "了", "和", "与", "或", "为", "被", "将", "不", "应", "必须", "使用",
        "所有", "每个", "通过", "可以", "需要", "这个", "那个", "一个", // English
        "a", "the", "is", "are", "must", "should", "all", "be", "to", "in", "of", "and", "or",
        "not", "no", "with", "for", "by", "use", "using", "this", "that", "when", "if", "then",
        "will", "does", "do", "has", "have", "can",
    ];

    // Extract backtick-quoted identifiers first (highest signal)
    let mut keywords: Vec<String> = Vec::new();
    let mut in_backtick = false;
    let mut current = String::new();
    for c in text.chars() {
        if c == '`' {
            if in_backtick && !current.is_empty() {
                keywords.push(current.clone());
                current.clear();
            }
            in_backtick = !in_backtick;
        } else if in_backtick {
            current.push(c);
        }
    }

    // Also extract regular words (lower priority)
    let words: Vec<String> = text
        .split(|c: char| c.is_whitespace() || c == ',' || c == '、' || c == '。' || c == '`')
        .filter(|w| {
            let w_lower = w.to_lowercase();
            w.len() > 2 && !stop_words.iter().any(|sw| w_lower == *sw)
        })
        .map(String::from)
        .collect();

    keywords.extend(words);
    keywords
}

// =============================================================================
// 11. ErrorPathLinter - checks if scenarios include error/failure paths
// =============================================================================

pub struct ErrorPathLinter;

const ERROR_PATH_INDICATORS_ZH: &[&str] = &[
    "错误",
    "失败",
    "拒绝",
    "异常",
    "超时",
    "不存在",
    "无效",
    "禁止",
    "返回错误",
    "命令失败",
    "返回 error",
    "isError",
];

const ERROR_PATH_INDICATORS_EN: &[&str] = &[
    "error",
    "fail",
    "reject",
    "invalid",
    "forbidden",
    "timeout",
    "not found",
    "not exist",
    "denied",
    "unauthorized",
    "4xx",
    "5xx",
    "non-2xx",
    "panic",
    "abort",
];

impl SpecLinter for ErrorPathLinter {
    fn name(&self) -> &str {
        "error-path"
    }

    fn lint(&self, doc: &SpecDocument) -> Vec<LintDiagnostic> {
        let mut diags = Vec::new();

        if doc.meta.level != SpecLevel::Task {
            return diags;
        }

        for section in &doc.sections {
            if let Section::AcceptanceCriteria { scenarios, span } = section {
                if scenarios.is_empty() {
                    continue;
                }

                let has_error_scenario = scenarios.iter().any(|sc| {
                    let name_lower = sc.name.to_lowercase();
                    let has_error_name = ERROR_PATH_INDICATORS_ZH
                        .iter()
                        .any(|ind| sc.name.contains(ind))
                        || ERROR_PATH_INDICATORS_EN
                            .iter()
                            .any(|ind| name_lower.contains(ind));

                    let has_error_step = sc.steps.iter().any(|step| {
                        let text_lower = step.text.to_lowercase();
                        ERROR_PATH_INDICATORS_ZH
                            .iter()
                            .any(|ind| step.text.contains(ind))
                            || ERROR_PATH_INDICATORS_EN
                                .iter()
                                .any(|ind| text_lower.contains(ind))
                    });

                    has_error_name || has_error_step
                });

                if !has_error_scenario {
                    diags.push(LintDiagnostic {
                        rule: "error-path".into(),
                        severity: Severity::Warning,
                        message: format!(
                            "no error/failure path scenarios found ({} scenarios are all happy paths)",
                            scenarios.len()
                        ),
                        span: *span,
                        suggestion: Some(
                            "add at least one scenario that tests error handling (e.g., invalid input, network failure, malformed data)".into(),
                        ),
                    });
                }
            }
        }

        diags
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec_parser::parse_spec_from_str;

    #[test]
    fn test_vague_verb_linter() {
        let input = r#"spec: task
name: "test"
---

## 约束

- 系统应处理用户请求
- 退款金额不得超过原始交易金额
"#;
        let doc = parse_spec_from_str(input).unwrap();
        let diags = VagueVerbLinter.lint(&doc);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("处理"));
    }

    #[test]
    fn test_unquantified_linter() {
        let input = r#"spec: task
name: "test"
---

## Constraints

- Response should be fast
- Timeout must be less than 500ms
"#;
        let doc = parse_spec_from_str(input).unwrap();
        let diags = UnquantifiedLinter.lint(&doc);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("fast"));
    }

    #[test]
    fn test_testability_linter() {
        let input = r#"spec: task
name: "test"
---

## 验收标准

场景: UI测试
  假设 用户已登录
  当 用户打开页面
  那么 界面应该美观
  并且 响应状态码为 200
"#;
        let doc = parse_spec_from_str(input).unwrap();
        let diags = TestabilityLinter.lint(&doc);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("美观"));
    }

    #[test]
    fn test_determinism_linter() {
        let input = r#"spec: task
name: "test"
---

## Acceptance Criteria

Scenario: test
  Given a user exists
  When user sends request
  Then response should take approximately 100ms
"#;
        let doc = parse_spec_from_str(input).unwrap();
        let diags = DeterminismLinter.lint(&doc);
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("approximately"));
    }

    #[test]
    fn test_full_pipeline() {
        let input = r#"spec: task
name: "退款功能"
---

## 意图

为支付网关添加退款功能。

## 约束

- 退款金额不得超过原始交易金额
- 退款操作需要管理员权限

## 验收标准

场景: 全额退款
  测试: test_full_refund
  假设 存在一笔金额为 "100.00" 元的已完成交易 "TXN-001"
  当 用户对 "TXN-001" 发起全额退款
  那么 退款状态变为 "processing"
"#;
        let doc = parse_spec_from_str(input).unwrap();
        let pipeline = crate::spec_lint::LintPipeline::with_defaults();
        let report = pipeline.run(&doc);
        assert!(!report.spec_name.is_empty());
        assert!(report.quality_score.overall >= 0.0);
        assert!(report.quality_score.overall <= 1.0);
    }

    #[test]
    fn test_explicit_test_binding_linter_requires_task_scenario_selectors() {
        let input = r#"spec: task
name: "test"
---

## 完成条件

场景: 缺失绑定
  假设 存在某个任务
  当 verifier 检查规格
  那么 应报告缺少 selector
"#;
        let doc = parse_spec_from_str(input).unwrap();
        let diags = ExplicitTestBindingLinter.lint(&doc);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
        assert!(
            diags[0]
                .message
                .contains("missing an explicit test selector")
        );
    }

    #[test]
    fn test_sycophancy_linter_flags_bug_finding_bias() {
        let input = r#"spec: task
name: "test"
---

## Intent

Review the code to find all bugs and catch all issues.

## Constraints

- You must find every bug in the implementation
"#;
        let doc = parse_spec_from_str(input).unwrap();
        let diags = SycophancyLinter.lint(&doc);
        assert!(
            !diags.is_empty(),
            "should flag sycophancy-inducing language"
        );
        assert!(diags.iter().any(|d| d.rule == "sycophancy"));
        assert!(diags.iter().any(|d| d.suggestion.is_some()));
    }

    #[test]
    fn test_quality_report_scores_testability_and_smells() {
        let input = r#"spec: task
name: "quality"
---

## Constraints

- Response should be fast and efficient
- All errors must use structured types

## Acceptance Criteria

Scenario: good path
  Test: test_quality_report_scores_testability_and_smells
  Given a user exists
  When user submits a request
  Then response status should be 200
  And the UI should look beautiful
"#;
        let doc = parse_spec_from_str(input).unwrap();
        let pipeline = crate::spec_lint::LintPipeline::with_defaults();
        let report = pipeline.run(&doc);

        assert!(
            report.diagnostics.iter().any(|d| d.rule == "testability"),
            "should flag untestable term"
        );
        assert!(
            report.diagnostics.iter().any(|d| d.rule == "unquantified"),
            "should flag unquantified qualifier"
        );
        assert!(
            report.quality_score.testability < 1.0,
            "testability penalized"
        );
        assert!(report.quality_score.overall > 0.0, "overall score positive");
        assert!(
            report.quality_score.overall < 1.0,
            "overall score penalized"
        );
    }

    #[test]
    fn test_cross_check_reports_boundary_and_decision_conflicts() {
        let spec_a = parse_spec_from_str(
            r#"spec: task
name: "Spec A"
---

## Decisions

- Use tokio for async runtime

## Boundaries

### Allowed Changes
- crates/spec-core/**
"#,
        )
        .unwrap();

        let spec_b = parse_spec_from_str(
            r#"spec: task
name: "Spec B"
---

## Decisions

- Do not use tokio for async runtime

## Boundaries

### Forbidden
- crates/spec-core/**
"#,
        )
        .unwrap();

        let diags = crate::spec_lint::cross_check(&[spec_a, spec_b]);

        assert!(
            diags.iter().any(|d| d.rule == "cross-check-boundary"),
            "should detect boundary conflict: {:?}",
            diags
        );

        assert!(
            diags.iter().any(|d| d.rule == "cross-check-decision"),
            "should detect decision conflict: {:?}",
            diags
        );
    }

    #[test]
    fn test_explicit_test_binding_linter_accepts_explicit_selector() {
        let input = r#"spec: task
name: "test"
---

## 完成条件

场景: 显式绑定
  测试: test_explicit_test_binding_linter_accepts_explicit_selector
  假设 存在某个任务
  当 verifier 检查规格
  那么 不应报告绑定错误
"#;
        let doc = parse_spec_from_str(input).unwrap();
        let diags = ExplicitTestBindingLinter.lint(&doc);
        assert!(diags.is_empty());
    }

    #[test]
    fn test_scenario_presence_linter_requires_acceptance_criteria() {
        let input = r#"spec: task
name: "missing scenarios"
---

## Intent

Describe the task.
"#;
        let doc = parse_spec_from_str(input).unwrap();
        let diags = ScenarioPresenceLinter.lint(&doc);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
        assert!(diags[0].message.contains("missing an Acceptance Criteria"));
    }

    // ── DecisionCoverageLinter tests ────────────────────────────────

    #[test]
    fn test_decision_coverage_warns_on_uncovered_decision() {
        let input = r#"spec: task
name: "test"
---

## 决策

- 使用 `BTreeMap` 确保输出顺序确定性。
- 本地源优先从 `source.path` 读取 registry。

## 验收标准

场景: 输出是确定性的
  测试: output_is_deterministic
  假设 已构建注册表
  当 运行 build 命令
  那么 输出使用 BTreeMap 排序
"#;
        let doc = parse_spec_from_str(input).unwrap();
        let diags = DecisionCoverageLinter.lint(&doc);
        // "BTreeMap" is covered by the scenario, but "source.path" / "本地源" is NOT
        assert!(
            diags.len() >= 1,
            "should warn about uncovered 'source.path' decision, got {} diags",
            diags.len()
        );
        assert!(diags.iter().any(|d| d.rule == "decision-coverage"));
    }

    #[test]
    fn test_decision_coverage_passes_when_all_covered() {
        let input = r#"spec: task
name: "test"
---

## 决策

- 使用 `BTreeMap` 确保输出顺序确定性。

## 验收标准

场景: 输出是确定性的
  测试: output_is_deterministic
  假设 已构建注册表
  当 运行 build 命令
  那么 输出使用 BTreeMap 排序
"#;
        let doc = parse_spec_from_str(input).unwrap();
        let diags = DecisionCoverageLinter.lint(&doc);
        assert!(diags.is_empty(), "all decisions covered, got: {:?}", diags);
    }

    // ── ErrorPathLinter tests ────────────────────────────────────────

    #[test]
    fn test_error_path_warns_on_all_happy_paths() {
        let input = r#"spec: task
name: "test"
---

## 验收标准

场景: 成功创建用户
  测试: create_user_success
  假设 数据库可用
  当 提交有效用户数据
  那么 用户被创建

场景: 成功查询用户
  测试: query_user_success
  假设 用户已存在
  当 查询用户列表
  那么 返回用户数据
"#;
        let doc = parse_spec_from_str(input).unwrap();
        let diags = ErrorPathLinter.lint(&doc);
        assert_eq!(diags.len(), 1, "should warn about missing error paths");
        assert_eq!(diags[0].rule, "error-path");
        assert!(diags[0].message.contains("happy paths"));
    }

    #[test]
    fn test_error_path_passes_with_error_scenario() {
        let input = r#"spec: task
name: "test"
---

## 验收标准

场景: 成功创建用户
  测试: create_user_success
  假设 数据库可用
  当 提交有效用户数据
  那么 用户被创建

场景: 无效数据返回错误
  测试: create_user_invalid_error
  假设 数据库可用
  当 提交无效用户数据
  那么 返回错误消息
"#;
        let doc = parse_spec_from_str(input).unwrap();
        let diags = ErrorPathLinter.lint(&doc);
        assert!(diags.is_empty(), "has error scenario, should pass");
    }

    #[test]
    fn test_error_path_detects_english_error_indicators() {
        let input = r#"spec: task
name: "test"
---

## Completion Criteria

Scenario: successful operation
  Test: op_success
  Given a valid input
  When the operation runs
  Then it returns 200

Scenario: rejects invalid input
  Test: op_rejects_invalid
  Given an invalid input
  When the operation runs
  Then it returns an error response
"#;
        let doc = parse_spec_from_str(input).unwrap();
        let diags = ErrorPathLinter.lint(&doc);
        assert!(diags.is_empty(), "has 'error' in step text, should pass");
    }

    #[test]
    fn test_scenario_presence_linter_rejects_empty_acceptance_criteria() {
        let input = r#"spec: task
name: "empty scenarios"
---

## Completion Criteria
"#;
        let doc = parse_spec_from_str(input).unwrap();
        let diags = ScenarioPresenceLinter.lint(&doc);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
        assert!(diags[0].message.contains("no parseable scenarios"));
    }
}
