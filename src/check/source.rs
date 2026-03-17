use std::path::PathBuf;

use wp_primitives::Parser;

use crate::parser::wpl_rule::wpl_rule;
use crate::{WplCode, error_detail, wpl_express};

use super::{Mode, ParseResult};

pub fn validate_source(
    source: &str,
    origin: Option<&str>,
    mode: Mode,
) -> Result<ParseResult, String> {
    let code = WplCode::build(PathBuf::from(origin.unwrap_or("stdin")), source)
        .map_err(|err| err.to_string())?;
    let normalized = code.get_code().as_str();
    match mode {
        Mode::Auto => validate_auto_source(normalized, origin),
        Mode::Package => code
            .parse_pkg()
            .map(ParseResult::Package)
            .map_err(|err| format!("source: parse failed in package mode\n{err}")),
        Mode::Rule => wpl_rule
            .parse(normalized)
            .map(ParseResult::Rule)
            .map_err(|err| format!("source: parse failed in rule mode\n{}", error_detail(err))),
        Mode::Expr => wpl_express
            .parse(normalized)
            .map(ParseResult::Expr)
            .map_err(|err| format!("source: parse failed in expr mode\n{}", error_detail(err))),
    }
}

pub fn validate_rule_name_usage(
    parsed: &ParseResult,
    rule_name: Option<&str>,
) -> Result<(), String> {
    if rule_name.is_none() || matches!(parsed, ParseResult::Package(_)) {
        return Ok(());
    }

    Err("--rule-name is only valid for package source".to_string())
}

pub fn source_summary(parsed: &ParseResult) -> String {
    match parsed {
        ParseResult::Package(package) => {
            format!(
                "source: ok (package {}, {} rules)",
                package.name,
                package.rules.len()
            )
        }
        ParseResult::Rule(rule) => format!("source: ok (rule {})", rule.name),
        ParseResult::Expr(expr) => format!(
            "source: ok (expression, {} groups, {} pipe steps)",
            expr.group.len(),
            expr.pipe_process.len()
        ),
    }
}

pub fn normalized_output(parsed: &ParseResult) -> String {
    match parsed {
        ParseResult::Package(package) => package.to_string(),
        ParseResult::Rule(rule) => rule.to_string(),
        ParseResult::Expr(expr) => expr.to_string(),
    }
}

fn validate_auto_source(normalized: &str, origin: Option<&str>) -> Result<ParseResult, String> {
    let attempts = match infer_mode(normalized) {
        Mode::Package => [Mode::Package, Mode::Rule, Mode::Expr],
        Mode::Rule => [Mode::Rule, Mode::Package, Mode::Expr],
        Mode::Expr => [Mode::Expr, Mode::Rule, Mode::Package],
        Mode::Auto => unreachable!("auto mode cannot remain unresolved"),
    };

    let mut first_error = None;
    for mode in attempts {
        match validate_source(normalized, origin, mode) {
            Ok(parsed) => return Ok(parsed),
            Err(err) if first_error.is_none() => first_error = Some(err),
            Err(_) => {}
        }
    }

    Err(first_error.unwrap_or_else(|| "source: parse failed in auto mode".to_string()))
}

fn infer_mode(source: &str) -> Mode {
    let trimmed = strip_leading_annotations(source).trim_start();
    if trimmed.starts_with("package") || trimmed.starts_with("#[") {
        Mode::Package
    } else if trimmed.starts_with("rule") {
        Mode::Rule
    } else {
        Mode::Expr
    }
}

fn strip_leading_annotations(source: &str) -> &str {
    let mut rest = source.trim_start();

    while let Some(after) = rest.strip_prefix("#[") {
        if let Some(offset) = find_annotation_end(after) {
            rest = after[offset..].trim_start();
        } else {
            break;
        }
    }

    rest
}

fn find_annotation_end(input: &str) -> Option<usize> {
    let mut in_string = false;
    let mut escape = false;

    for (idx, ch) in input.char_indices() {
        if in_string {
            if escape {
                escape = false;
                continue;
            }

            match ch {
                '\\' => escape = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            ']' => return Some(idx + ch.len_utf8()),
            _ => {}
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_mode() {
        assert_eq!(
            infer_mode("package demo { rule x { (digit) } }"),
            Mode::Package
        );
        assert_eq!(infer_mode("rule x { (digit) }"), Mode::Rule);
        assert_eq!(infer_mode("(digit:id,chars:name)"), Mode::Expr);
    }

    #[test]
    fn test_infer_mode_skips_leading_annotations() {
        assert_eq!(
            infer_mode("#[tag(t1:\"id\")]\nrule hello { (digit:id) }"),
            Mode::Rule
        );
        assert_eq!(
            infer_mode("#[tag(t1:\"id\")]\npackage demo { rule hello { (digit:id) } }"),
            Mode::Package
        );
    }

    #[test]
    fn test_validate_auto_accepts_annotated_rule() {
        let parsed = validate_source(
            "#[tag(t1:\"id\")]\nrule hello { (digit:id) }",
            Some("annotated_rule.wpl"),
            Mode::Auto,
        )
        .unwrap();

        assert_eq!(source_summary(&parsed), "source: ok (rule hello)");
    }

    #[test]
    fn test_validate_auto_reports_rule_error_for_annotated_rule() {
        let err = validate_source(
            "#[tag(t1:\"id\")]\nrule hello { (digit:id, }",
            Some("annotated_rule_bad.wpl"),
            Mode::Auto,
        )
        .unwrap_err();

        assert!(err.contains("source: parse failed in rule mode"));
        assert!(err.contains("line 2, column 25"));
    }

    #[test]
    fn test_validate_package() {
        let parsed = validate_source(
            include_str!("../../examples/wpl-check/package_demo/rule.wpl"),
            Some("examples/wpl-check/package_demo/rule.wpl"),
            Mode::Auto,
        )
        .unwrap();

        assert_eq!(
            source_summary(&parsed),
            "source: ok (package demo, 2 rules)"
        );
    }

    #[test]
    fn test_validate_error_contains_position() {
        let err =
            validate_source("rule demo { (digit:id, }", Some("bad.wpl"), Mode::Rule).unwrap_err();
        assert!(err.contains("source: parse failed in rule mode"));
        assert!(err.contains("parse error at line"));
    }
}
