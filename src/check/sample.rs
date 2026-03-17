use wp_model_core::raw::RawData;

use crate::{WplEvaluator, WplPackage, WplRule, WplStatementType};

use super::{EvalResult, ParseResult};

pub fn validate_sample_target(parsed: &ParseResult, rule_name: Option<&str>) -> Result<(), String> {
    match parsed {
        ParseResult::Expr(_) | ParseResult::Rule(_) => {
            if rule_name.is_some() {
                return Err("--rule-name is only valid for package source".to_string());
            }
            Ok(())
        }
        ParseResult::Package(package) => {
            select_rule(package, rule_name)?;
            Ok(())
        }
    }
}

pub fn evaluate_sample(
    parsed: &ParseResult,
    rule_name: Option<&str>,
    sample: &str,
) -> Result<EvalResult, String> {
    let plan = build_eval_plan(parsed, rule_name)?;
    let prepared = prepare_parser_input(&plan.evaluator, sample)?;
    let mut input = prepared.text.as_str();

    let record = plan
        .evaluator
        .parse_groups(0, &mut input)
        .map_err(|err| format_sample_error(&plan, &prepared, input, &err.to_string()))?;

    Ok(EvalResult {
        target: plan.target,
        record: record.to_string(),
        residue: input.to_string(),
        field_count: record.items.len(),
    })
}

struct EvalPlan {
    evaluator: WplEvaluator,
    target: String,
}

struct PreparedInput {
    text: String,
    label: &'static str,
}

fn prepare_parser_input(evaluator: &WplEvaluator, sample: &str) -> Result<PreparedInput, String> {
    let steps = evaluator
        .preorder_proc(RawData::from_string(sample.to_string()))
        .map_err(|err| format!("preprocess: failed\n{err}"))?;

    if let Some(step) = steps.last() {
        Ok(PreparedInput {
            text: step.result.clone(),
            label: "parser input",
        })
    } else {
        Ok(PreparedInput {
            text: sample.to_string(),
            label: "sample",
        })
    }
}

fn format_sample_error(
    plan: &EvalPlan,
    prepared: &PreparedInput,
    remaining_input: &str,
    parser: &str,
) -> String {
    let offset = prepared.text.len().saturating_sub(remaining_input.len());
    let near = sample_preview(&prepared.text, offset);
    let (line, column) = translate_position(&prepared.text, offset);
    let pointer = render_sample_pointer(&prepared.text, offset);
    let reason = humanize_parser_error(parser);
    let hints = collect_sample_hints(plan, offset, &reason);

    let mut out = String::new();
    out.push_str("ERROR wpl-check sample\n");
    out.push_str(&format!("reason: {reason}\n"));
    out.push_str(&format!("target: {}\n", plan.target));
    out.push_str(&format!("offset: {offset}\n"));
    out.push_str(&format!("line: {}, column: {}\n", line + 1, column + 1));
    out.push_str(&format!("near: {near}\n"));
    out.push_str(&format!("{}:\n", prepared.label));
    out.push_str(&pointer);
    out.push('\n');
    if !hints.is_empty() {
        out.push_str("hints:\n");
        for hint in hints {
            out.push_str("  - ");
            out.push_str(hint);
            out.push('\n');
        }
    }
    out.push_str("\nparser:\n");
    out.push_str(parser);
    out
}

fn sample_preview(sample: &str, offset: usize) -> String {
    let start = floor_char_boundary(sample, offset.saturating_sub(24));
    let end = ceil_char_boundary(sample, usize::min(sample.len(), offset + 24));
    let prefix = if start > 0 { "..." } else { "" };
    let suffix = if end < sample.len() { "..." } else { "" };
    let snippet = &sample[start..end];
    format!("{}{:?}{}", prefix, snippet, suffix)
}

fn translate_position(input: &str, index: usize) -> (usize, usize) {
    let safe_index = floor_char_boundary(input, usize::min(index, input.len()));
    let prefix = &input[..safe_index];
    let line = prefix.chars().filter(|ch| *ch == '\n').count();
    let line_start = prefix.rfind('\n').map(|pos| pos + 1).unwrap_or(0);
    let column = input[line_start..safe_index].chars().count();

    (line, column)
}

fn render_sample_pointer(sample: &str, offset: usize) -> String {
    let safe_offset = floor_char_boundary(sample, usize::min(offset, sample.len()));
    let line_start = sample[..safe_offset]
        .rfind('\n')
        .map(|pos| pos + 1)
        .unwrap_or(0);
    let line_end = sample[safe_offset..]
        .find('\n')
        .map(|pos| safe_offset + pos)
        .unwrap_or(sample.len());
    let line_text = &sample[line_start..line_end];
    let (line, column) = translate_position(sample, safe_offset);
    let line_num = line + 1;
    let gutter = line_num.to_string().len();
    let (display_line, display_column) = clip_line_for_display(line_text, column, 80);

    let mut out = String::new();
    out.push_str(&format!("{} | {}\n", line_num, display_line));
    for _ in 0..gutter {
        out.push(' ');
    }
    out.push_str(" | ");
    for _ in 0..display_column {
        out.push(' ');
    }
    out.push('^');
    out
}

fn clip_line_for_display(line: &str, column: usize, max_chars: usize) -> (String, usize) {
    let total_chars = line.chars().count();
    if total_chars <= max_chars {
        return (line.to_string(), column);
    }

    let half = max_chars / 2;
    let start_char = column.saturating_sub(half);
    let end_char = usize::min(total_chars, start_char + max_chars);
    let start_char = end_char.saturating_sub(max_chars);
    let start_byte = nth_char_boundary(line, start_char);
    let end_byte = nth_char_boundary(line, end_char);
    let prefix = if start_char > 0 { "..." } else { "" };
    let suffix = if end_char < total_chars { "..." } else { "" };
    let shown_column = prefix.chars().count() + column.saturating_sub(start_char);

    (
        format!("{prefix}{}{suffix}", &line[start_byte..end_byte]),
        shown_column,
    )
}

fn nth_char_boundary(text: &str, char_idx: usize) -> usize {
    if char_idx == 0 {
        return 0;
    }

    text.char_indices()
        .nth(char_idx)
        .map(|(idx, _)| idx)
        .unwrap_or(text.len())
}

fn humanize_parser_error(parser: &str) -> String {
    let descriptions = extract_descriptions(parser);
    if descriptions.is_empty() {
        return parser.to_string();
    }

    let mut expected = Vec::new();
    let mut locations = Vec::new();
    for item in descriptions {
        if item.starts_with("group[") {
            locations.push(item);
        } else {
            expected.push(item);
        }
    }

    match (expected.is_empty(), locations.is_empty()) {
        (false, false) => format!(
            "expected {} in {}",
            expected.join(" or "),
            locations.join(" / ")
        ),
        (false, true) => format!("expected {}", expected.join(" or ")),
        (true, false) => format!("failed in {}", locations.join(" / ")),
        (true, true) => parser.to_string(),
    }
}

fn extract_descriptions(text: &str) -> Vec<String> {
    let needle = "Description(\"";
    let mut out = Vec::new();
    let mut rest = text;

    while let Some(start) = rest.find(needle) {
        let after = &rest[start + needle.len()..];
        if let Some(end) = after.find("\")") {
            out.push(after[..end].to_string());
            rest = &after[end + 2..];
        } else {
            break;
        }
    }

    out
}

fn collect_sample_hints<'a>(plan: &EvalPlan, offset: usize, reason: &'a str) -> Vec<&'a str> {
    let mut hints = Vec::new();

    if offset == 0 {
        hints.push("The sample failed at the start; check the first field in rule.wpl.");
    }
    if reason.contains("<digit>") {
        hints.push("The current field expects a digit-like value.");
    }
    if reason.contains("<ip>") {
        hints.push("The current field expects an IP address.");
    }
    if reason.contains("<time>") {
        hints.push("The current field expects a time value in the configured format.");
    }
    if plan.target.starts_with("package ") {
        hints.push(
            "If this package has multiple rules, confirm --rule-name selects the intended rule.",
        );
    }
    hints.push("Use `wpl-check syntax --print ...` to inspect the normalized WPL.");

    hints
}

fn floor_char_boundary(text: &str, index: usize) -> usize {
    let mut index = usize::min(index, text.len());
    while index > 0 && !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn ceil_char_boundary(text: &str, index: usize) -> usize {
    let mut index = usize::min(index, text.len());
    while index < text.len() && !text.is_char_boundary(index) {
        index += 1;
    }
    index
}

fn build_eval_plan(parsed: &ParseResult, rule_name: Option<&str>) -> Result<EvalPlan, String> {
    match parsed {
        ParseResult::Expr(expr) => {
            if rule_name.is_some() {
                return Err("--rule-name is only valid for package source".to_string());
            }

            Ok(EvalPlan {
                evaluator: WplEvaluator::from(expr, None).map_err(|err| err.to_string())?,
                target: "expression".to_string(),
            })
        }
        ParseResult::Rule(rule) => {
            if rule_name.is_some() {
                return Err("--rule-name is only valid for package source".to_string());
            }

            let WplStatementType::Express(expr) = &rule.statement;
            Ok(EvalPlan {
                evaluator: WplEvaluator::from(expr, None).map_err(|err| err.to_string())?,
                target: format!("rule {}", rule.name),
            })
        }
        ParseResult::Package(package) => {
            let rule = select_rule(package, rule_name)?;
            let WplStatementType::Express(expr) = &rule.statement;
            Ok(EvalPlan {
                evaluator: WplEvaluator::from(expr, None).map_err(|err| err.to_string())?,
                target: format!("package {} / rule {}", package.name, rule.name),
            })
        }
    }
}

fn select_rule<'a>(
    package: &'a WplPackage,
    rule_name: Option<&str>,
) -> Result<&'a WplRule, String> {
    if let Some(rule_name) = rule_name {
        return package
            .rules
            .iter()
            .find(|rule| rule.name.as_str() == rule_name)
            .ok_or_else(|| {
                format!(
                    "rule '{rule_name}' not found in package {}; available rules: {}",
                    package.name,
                    package
                        .rules
                        .iter()
                        .map(|rule| rule.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            });
    }

    if package.rules.len() == 1 {
        return package
            .rules
            .front()
            .ok_or_else(|| format!("package {} has no rules", package.name));
    }

    Err(format!(
        "package {} has {} rules; use --rule-name. available rules: {}",
        package.name,
        package.rules.len(),
        package
            .rules
            .iter()
            .map(|rule| rule.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

#[cfg(test)]
mod tests {
    use crate::check::{Mode, validate_source};

    use super::*;

    #[test]
    fn test_evaluate_rule_sample() {
        let parsed = validate_source(
            include_str!("../../examples/wpl-check/csv_demo/rule.wpl"),
            Some("examples/wpl-check/csv_demo/rule.wpl"),
            Mode::Rule,
        )
        .unwrap();
        let result = evaluate_sample(
            &parsed,
            None,
            include_str!("../../examples/wpl-check/csv_demo/sample.txt").trim_end(),
        )
        .unwrap();

        assert_eq!(result.target, "rule demo");
        assert_eq!(result.field_count, 2);
        assert_eq!(result.residue, "");
        assert!(result.record.contains("id"));
        assert!(result.record.contains("alice"));
    }

    #[test]
    fn test_select_rule_requires_name_for_multi_rule_package() {
        let parsed = validate_source(
            include_str!("../../examples/wpl-check/package_demo/rule.wpl"),
            Some("examples/wpl-check/package_demo/rule.wpl"),
            Mode::Package,
        )
        .unwrap();

        let err = evaluate_sample(&parsed, None, "1").unwrap_err();
        assert!(err.contains("use --rule-name"));
        assert!(err.contains("available rules: csv_user, json_env"));
    }

    #[test]
    fn test_rule_name_is_rejected_for_non_package_source() {
        let parsed = validate_source(
            include_str!("../../examples/wpl-check/csv_demo/rule.wpl"),
            Some("examples/wpl-check/csv_demo/rule.wpl"),
            Mode::Rule,
        )
        .unwrap();

        let err = validate_sample_target(&parsed, Some("csv_user")).unwrap_err();
        assert_eq!(err, "--rule-name is only valid for package source");
    }

    #[test]
    fn test_validate_sample_target_requires_rule_name_for_multi_rule_package() {
        let parsed = validate_source(
            include_str!("../../examples/wpl-check/package_demo/rule.wpl"),
            Some("examples/wpl-check/package_demo/rule.wpl"),
            Mode::Package,
        )
        .unwrap();

        let err = validate_sample_target(&parsed, None).unwrap_err();
        assert!(err.contains("use --rule-name"));
        assert!(err.contains("available rules: csv_user, json_env"));
    }

    #[test]
    fn test_validate_sample_target_rejects_unknown_rule_name() {
        let parsed = validate_source(
            include_str!("../../examples/wpl-check/package_demo/rule.wpl"),
            Some("examples/wpl-check/package_demo/rule.wpl"),
            Mode::Package,
        )
        .unwrap();

        let err = validate_sample_target(&parsed, Some("missing_rule")).unwrap_err();
        assert!(err.contains("rule 'missing_rule' not found"));
        assert!(err.contains("available rules: csv_user, json_env"));
    }

    #[test]
    fn test_evaluate_log_rule_sample_from_example() {
        let parsed = validate_source(
            include_str!("../../examples/wpl-check/log_line/rule.wpl"),
            Some("examples/wpl-check/log_line/rule.wpl"),
            Mode::Rule,
        )
        .unwrap();
        let result = evaluate_sample(
            &parsed,
            None,
            include_str!("../../examples/wpl-check/log_line/sample.txt").trim_end(),
        )
        .unwrap();

        assert_eq!(result.residue, "");
        assert!(result.record.contains("level"));
        assert!(result.record.contains("ctrl"));
        assert!(
            result
                .record
                .contains("log conf: level: warn,ctrl=info,dfx=info,data=info")
        );
    }

    #[test]
    fn test_sample_error_is_friendly() {
        let parsed = validate_source(
            include_str!("../../examples/wpl-check/csv_demo/rule.wpl"),
            Some("examples/wpl-check/csv_demo/rule.wpl"),
            Mode::Rule,
        )
        .unwrap();

        let err = evaluate_sample(&parsed, None, "oops").unwrap_err();
        assert!(err.contains("ERROR wpl-check sample"));
        assert!(err.contains("target: rule demo"));
        assert!(err.contains("offset:"));
        assert!(err.contains("line: 1, column: 1"));
        assert!(err.contains("near:"));
        assert!(err.contains("sample:"));
        assert!(err.contains("^"));
        assert!(err.contains("reason: expected <digit> in group[1]"));
        assert!(err.contains("hints:"));
        assert!(err.contains("parser:"));
    }

    #[test]
    fn test_render_sample_pointer_clips_long_line() {
        let sample = format!("{}oops{}", "a".repeat(100), "b".repeat(100));
        let pointer = render_sample_pointer(&sample, 100);

        assert!(pointer.contains("..."));
        assert!(pointer.contains("^"));
        assert!(!pointer.contains(&sample));
    }

    #[test]
    fn test_translate_position_handles_eof_after_newline() {
        assert_eq!(translate_position("abc\n", 4), (1, 0));
    }

    #[test]
    fn test_render_sample_pointer_handles_eof_after_newline() {
        let pointer = render_sample_pointer("abc\n", 4);

        assert!(pointer.contains("2 | "));
        assert!(pointer.ends_with("^"));
    }

    #[test]
    fn test_translate_position_counts_unicode_columns() {
        assert_eq!(translate_position("你好x", "你好".len()), (0, 2));
    }
}
