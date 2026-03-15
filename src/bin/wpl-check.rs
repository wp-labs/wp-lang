use std::env;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use wp_model_core::raw::RawData;
use wp_primitives::Parser;
use wpl::parser::wpl_rule::wpl_rule;
use wpl::{
    WplCode, WplEvaluator, WplExpress, WplPackage, WplRule, WplStatementType, error_detail,
    wpl_express,
};

const HELP: &str = "\
Quick WPL validation tool.

Usage:
  wpl-check syntax [--auto|--package|--rule|--expr] [--print] [WPL_FILE|-|DIR]
  wpl-check sample [--auto|--package|--rule|--expr] [--print] [--rule-name NAME] <WPL_FILE|-> <DATA_FILE>
  wpl-check sample [--auto|--package|--rule|--expr] [--print] [--rule-name NAME] [WPL_FILE|-|DIR] [DATA_FILE]
  wpl-check sample [--auto|--package|--rule|--expr] [--print] [--rule-name NAME] [--data TEXT] [WPL_FILE|-|DIR]
  wpl-check help

Commands:
  syntax   Parse WPL source and validate syntax only
  sample   Parse WPL source, then run one sample payload
  help     Show this help message

Compatibility:
  None. Use an explicit subcommand.
";

const SYNTAX_HELP: &str = "\
Validate WPL source syntax.

Usage:
  wpl-check syntax [--auto|--package|--rule|--expr] [--print] [WPL_FILE|-|DIR]

Options:
  --auto      Infer mode from source prefix (default)
  --package   Parse as package
  --rule      Parse as single rule
  --expr      Parse as expression
  --print     Print normalized source after parsing
  -h, --help  Show this help message

When omitted, syntax uses `rule.wpl` in the current directory. Pass `-` to read source from stdin.
";

const SAMPLE_HELP: &str = "\
Run one sample payload through WPL source.

Usage:
  wpl-check sample [--auto|--package|--rule|--expr] [--print] [--rule-name NAME] <WPL_FILE|-> <DATA_FILE>
  wpl-check sample [--auto|--package|--rule|--expr] [--print] [--rule-name NAME] [WPL_FILE|-|DIR] [DATA_FILE]
  wpl-check sample [--auto|--package|--rule|--expr] [--print] [--rule-name NAME] [--data TEXT] [WPL_FILE|-|DIR]

Options:
  --auto       Infer mode from source prefix (default)
  --package    Parse source as package
  --rule       Parse source as single rule
  --expr       Parse source as expression
  --rule-name  Select one rule from a package
  --data       Sample payload text, for quick one-off checks
  --print      Print normalized source before running sample
  -h, --help   Show this help message

Defaults:
  WPL file: `rule.wpl`
  sample file: `sample.txt`

If you pass a directory, `wpl-check` resolves `rule.wpl` and `sample.txt` inside it. For package input with multiple rules, use --rule-name.
";

const DEFAULT_RULE_FILE: &str = "rule.wpl";
const DEFAULT_SAMPLE_FILE: &str = "sample.txt";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    Auto,
    Package,
    Rule,
    Expr,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HelpTopic {
    Global,
    Syntax,
    Sample,
}

#[derive(Debug, Eq, PartialEq)]
enum Cli {
    Help(HelpTopic),
    Command(Command),
}

#[derive(Debug, Eq, PartialEq)]
enum Command {
    Syntax(SourceConfig),
    Sample(SampleConfig),
}

#[derive(Debug, Eq, PartialEq)]
struct SourceConfig {
    mode: Mode,
    print_source: bool,
    input: Option<PathBuf>,
}

#[derive(Debug, Eq, PartialEq)]
struct SampleConfig {
    source: SourceConfig,
    rule_name: Option<String>,
    sample: SampleInput,
}

#[derive(Debug, Eq, PartialEq)]
enum SampleInput {
    Inline(String),
    DefaultFile,
    File(PathBuf),
}

#[derive(Debug)]
enum ParseResult {
    Package(WplPackage),
    Rule(WplRule),
    Expr(WplExpress),
}

#[derive(Debug)]
struct EvalResult {
    target: String,
    record: String,
    residue: String,
    field_count: usize,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    match parse_args(env::args().skip(1))? {
        Cli::Help(topic) => {
            print!("{}", help_text(topic));
            Ok(())
        }
        Cli::Command(Command::Syntax(config)) => run_syntax(config),
        Cli::Command(Command::Sample(config)) => run_sample(config),
    }
}

fn run_syntax(config: SourceConfig) -> Result<(), String> {
    let input = resolve_source_path(config.input.as_deref());
    let (source, origin) = load_input(input.as_deref())?;
    let parsed = validate_source(&source, origin.as_deref(), config.mode)?;

    println!("{}", source_summary(&parsed));
    if config.print_source {
        println!();
        println!("{}", normalized_output(&parsed));
    }
    Ok(())
}

fn run_sample(config: SampleConfig) -> Result<(), String> {
    let source_input = resolve_source_path(config.source.input.as_deref());
    let sample_input = resolve_sample_input(&config.sample, source_input.as_deref());
    let (source, origin) = load_input(source_input.as_deref())?;
    let parsed = validate_source(&source, origin.as_deref(), config.source.mode)?;
    validate_rule_name_usage(&parsed, config.rule_name.as_deref())?;
    let sample = load_sample_data(&sample_input)?;
    let result = evaluate_sample(&parsed, config.rule_name.as_deref(), &sample)?;

    println!("{}", source_summary(&parsed));
    if config.source.print_source {
        println!();
        println!("{}", normalized_output(&parsed));
    }
    println!();
    println!(
        "data: ok ({}, {} fields, {} bytes residue)",
        result.target,
        result.field_count,
        result.residue.len()
    );
    println!();
    println!("{}", result.record);
    if !result.residue.is_empty() {
        println!();
        println!("residue:");
        println!("{}", result.residue);
    }
    Ok(())
}

fn parse_args<I>(args: I) -> Result<Cli, String>
where
    I: IntoIterator<Item = String>,
{
    let mut args = args.into_iter();
    let Some(first) = args.next() else {
        return Ok(Cli::Help(HelpTopic::Global));
    };

    match first.as_str() {
        "-h" | "--help" | "help" => Ok(Cli::Help(HelpTopic::Global)),
        "syntax" => parse_syntax_args(args),
        "sample" => parse_sample_args(args),
        _ => Err(format!("unknown command: {first}\n\n{HELP}")),
    }
}

fn parse_syntax_args<I>(args: I) -> Result<Cli, String>
where
    I: IntoIterator<Item = String>,
{
    let mut mode = Mode::Auto;
    let mut print_source = false;
    let mut input = None;

    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Cli::Help(HelpTopic::Syntax)),
            "--auto" => mode = Mode::Auto,
            "--package" => mode = Mode::Package,
            "--rule" => mode = Mode::Rule,
            "--expr" => mode = Mode::Expr,
            "--print" => print_source = true,
            value if value.starts_with('-') && value != "-" => {
                return Err(format!("unknown option: {value}\n\n{SYNTAX_HELP}"));
            }
            _ if input.is_some() => {
                return Err(format!("only one input file is supported\n\n{SYNTAX_HELP}"));
            }
            _ => input = Some(PathBuf::from(arg)),
        }
    }

    Ok(Cli::Command(Command::Syntax(SourceConfig {
        mode,
        print_source,
        input: Some(input.unwrap_or_else(|| PathBuf::from(DEFAULT_RULE_FILE))),
    })))
}

fn parse_sample_args<I>(args: I) -> Result<Cli, String>
where
    I: IntoIterator<Item = String>,
{
    let mut mode = Mode::Auto;
    let mut print_source = false;
    let mut source_input = None;
    let mut sample_path = None;
    let mut rule_name = None;
    let mut sample_data = None;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Cli::Help(HelpTopic::Sample)),
            "--auto" => mode = Mode::Auto,
            "--package" => mode = Mode::Package,
            "--rule" => mode = Mode::Rule,
            "--expr" => mode = Mode::Expr,
            "--print" => print_source = true,
            "--rule-name" => rule_name = Some(next_value(&mut args, "--rule-name", SAMPLE_HELP)?),
            "--data" => sample_data = Some(next_value(&mut args, "--data", SAMPLE_HELP)?),
            value if value.starts_with('-') && value != "-" => {
                return Err(format!("unknown option: {value}\n\n{SAMPLE_HELP}"));
            }
            _ if source_input.is_none() => source_input = Some(PathBuf::from(arg)),
            _ if sample_path.is_none() => sample_path = Some(PathBuf::from(arg)),
            _ => {
                return Err(format!(
                    "sample accepts at most two positional files\n\n{SAMPLE_HELP}"
                ));
            }
        }
    }

    let source_input = source_input.unwrap_or_else(|| PathBuf::from(DEFAULT_RULE_FILE));

    let sample = match (sample_data, sample_path) {
        (Some(_), Some(_)) => {
            return Err("use either --data or a positional data file, not both".to_string());
        }
        (Some(data), None) => SampleInput::Inline(data),
        (None, Some(path)) => SampleInput::File(path),
        (None, None) => SampleInput::DefaultFile,
    };

    Ok(Cli::Command(Command::Sample(SampleConfig {
        source: SourceConfig {
            mode,
            print_source,
            input: Some(source_input),
        },
        rule_name,
        sample,
    })))
}

fn next_value<I>(args: &mut I, flag: &str, help: &str) -> Result<String, String>
where
    I: Iterator<Item = String>,
{
    args.next()
        .ok_or_else(|| format!("missing value for {flag}\n\n{help}"))
}

fn help_text(topic: HelpTopic) -> &'static str {
    match topic {
        HelpTopic::Global => HELP,
        HelpTopic::Syntax => SYNTAX_HELP,
        HelpTopic::Sample => SAMPLE_HELP,
    }
}

fn resolve_source_path(path: Option<&Path>) -> Option<PathBuf> {
    match path {
        Some(path) if path == Path::new("-") => Some(path.to_path_buf()),
        Some(path) if path.is_dir() => Some(path.join(DEFAULT_RULE_FILE)),
        Some(path) => Some(path.to_path_buf()),
        None => None,
    }
}

fn resolve_sample_input(sample: &SampleInput, source_path: Option<&Path>) -> SampleInput {
    match sample {
        SampleInput::Inline(data) => SampleInput::Inline(data.clone()),
        SampleInput::DefaultFile => {
            let path = if let Some(source_path) = source_path {
                if source_path != Path::new("-") {
                    source_path
                        .parent()
                        .map(|base| base.join(DEFAULT_SAMPLE_FILE))
                        .unwrap_or_else(|| PathBuf::from(DEFAULT_SAMPLE_FILE))
                } else {
                    PathBuf::from(DEFAULT_SAMPLE_FILE)
                }
            } else {
                PathBuf::from(DEFAULT_SAMPLE_FILE)
            };
            SampleInput::File(path)
        }
        SampleInput::File(path) if path.is_absolute() => SampleInput::File(path.clone()),
        SampleInput::File(path) => {
            if path.is_dir() {
                SampleInput::File(path.join(DEFAULT_SAMPLE_FILE))
            } else {
                SampleInput::File(path.clone())
            }
        }
    }
}

fn load_input(path: Option<&Path>) -> Result<(String, Option<String>), String> {
    match path {
        Some(path) if path != Path::new("-") => {
            let source = std::fs::read_to_string(path)
                .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
            Ok((source, Some(path.display().to_string())))
        }
        _ => {
            let mut source = String::new();
            io::stdin()
                .read_to_string(&mut source)
                .map_err(|err| format!("failed to read stdin: {err}"))?;
            Ok((source, None))
        }
    }
}

fn load_sample_data(sample: &SampleInput) -> Result<String, String> {
    match sample {
        SampleInput::Inline(data) => Ok(data.clone()),
        SampleInput::DefaultFile => {
            Err("internal error: default sample path was not resolved".to_string())
        }
        SampleInput::File(path) => std::fs::read_to_string(path)
            .map_err(|err| format!("failed to read sample data {}: {err}", path.display())),
    }
}

fn validate_source(source: &str, origin: Option<&str>, mode: Mode) -> Result<ParseResult, String> {
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

fn validate_rule_name_usage(parsed: &ParseResult, rule_name: Option<&str>) -> Result<(), String> {
    if rule_name.is_none() || matches!(parsed, ParseResult::Package(_)) {
        return Ok(());
    }

    Err("--rule-name is only valid for package source".to_string())
}

fn source_summary(parsed: &ParseResult) -> String {
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

fn normalized_output(parsed: &ParseResult) -> String {
    match parsed {
        ParseResult::Package(package) => package.to_string(),
        ParseResult::Rule(rule) => rule.to_string(),
        ParseResult::Expr(expr) => expr.to_string(),
    }
}

fn evaluate_sample(
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
    use super::*;

    #[test]
    fn test_parse_args_without_command_shows_help() {
        assert_eq!(
            parse_args(Vec::<String>::new()).unwrap(),
            Cli::Help(HelpTopic::Global)
        );
    }

    #[test]
    fn test_parse_args_syntax_subcommand() {
        let cli = parse_args(
            ["syntax", "--rule", "--print", "demo.wpl"]
                .into_iter()
                .map(str::to_string),
        )
        .unwrap();

        assert_eq!(
            cli,
            Cli::Command(Command::Syntax(SourceConfig {
                mode: Mode::Rule,
                print_source: true,
                input: Some(PathBuf::from("demo.wpl")),
            }))
        );
    }

    #[test]
    fn test_parse_args_syntax_defaults_to_rule_file() {
        let cli = parse_args(["syntax"].into_iter().map(str::to_string)).unwrap();

        assert_eq!(
            cli,
            Cli::Command(Command::Syntax(SourceConfig {
                mode: Mode::Auto,
                print_source: false,
                input: Some(PathBuf::from("rule.wpl")),
            }))
        );
    }

    #[test]
    fn test_parse_args_requires_subcommand() {
        let err = parse_args(["--expr", "demo.wpl"].into_iter().map(str::to_string)).unwrap_err();
        assert!(err.contains("unknown command"));
    }

    #[test]
    fn test_parse_args_sample_subcommand() {
        let cli = parse_args(
            [
                "sample",
                "--package",
                "--rule-name",
                "demo_rule",
                "demo.wpl",
                "sample.txt",
            ]
            .into_iter()
            .map(str::to_string),
        )
        .unwrap();

        assert_eq!(
            cli,
            Cli::Command(Command::Sample(SampleConfig {
                source: SourceConfig {
                    mode: Mode::Package,
                    print_source: false,
                    input: Some(PathBuf::from("demo.wpl")),
                },
                rule_name: Some("demo_rule".to_string()),
                sample: SampleInput::File(PathBuf::from("sample.txt")),
            }))
        );
    }

    #[test]
    fn test_parse_args_sample_with_inline_data() {
        let cli = parse_args(
            ["sample", "--rule", "--data", "1,alice", "demo.wpl"]
                .into_iter()
                .map(str::to_string),
        )
        .unwrap();

        assert_eq!(
            cli,
            Cli::Command(Command::Sample(SampleConfig {
                source: SourceConfig {
                    mode: Mode::Rule,
                    print_source: false,
                    input: Some(PathBuf::from("demo.wpl")),
                },
                rule_name: None,
                sample: SampleInput::Inline("1,alice".to_string()),
            }))
        );
    }

    #[test]
    fn test_parse_args_sample_defaults_to_rule_and_sample_files() {
        let cli = parse_args(["sample"].into_iter().map(str::to_string)).unwrap();

        assert_eq!(
            cli,
            Cli::Command(Command::Sample(SampleConfig {
                source: SourceConfig {
                    mode: Mode::Auto,
                    print_source: false,
                    input: Some(PathBuf::from("rule.wpl")),
                },
                rule_name: None,
                sample: SampleInput::DefaultFile,
            }))
        );
    }

    #[test]
    fn test_resolve_directory_defaults() {
        let source = resolve_source_path(Some(Path::new("examples/wpl-check/csv_demo"))).unwrap();
        let sample = resolve_sample_input(&SampleInput::DefaultFile, Some(source.as_path()));

        assert_eq!(
            source,
            PathBuf::from("examples/wpl-check/csv_demo/rule.wpl")
        );
        assert_eq!(
            sample,
            SampleInput::File(PathBuf::from("examples/wpl-check/csv_demo/sample.txt"))
        );
    }

    #[test]
    fn test_explicit_relative_sample_path_is_not_rebased() {
        let source = PathBuf::from("examples/wpl-check/package_demo/rule.wpl");
        let sample = resolve_sample_input(
            &SampleInput::File(PathBuf::from("custom.txt")),
            Some(source.as_path()),
        );

        assert_eq!(sample, SampleInput::File(PathBuf::from("custom.txt")));
    }

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

        let err = evaluate_sample(&parsed, Some("csv_user"), "42,alice").unwrap_err();
        assert_eq!(err, "--rule-name is only valid for package source");
    }

    #[test]
    fn test_rule_name_is_rejected_before_sample_file_read() {
        let config = SampleConfig {
            source: SourceConfig {
                mode: Mode::Rule,
                print_source: false,
                input: Some(PathBuf::from("examples/wpl-check/csv_demo/rule.wpl")),
            },
            rule_name: Some("csv_user".to_string()),
            sample: SampleInput::File(PathBuf::from("missing.txt")),
        };

        let err = run_sample(config).unwrap_err();
        assert_eq!(err, "--rule-name is only valid for package source");
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
    fn test_validate_error_contains_position() {
        let err =
            validate_source("rule demo { (digit:id, }", Some("bad.wpl"), Mode::Rule).unwrap_err();
        assert!(err.contains("source: parse failed in rule mode"));
        assert!(err.contains("parse error at line"));
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

    #[test]
    fn test_load_sample_data_preserves_trailing_newline() {
        let path = PathBuf::from(format!(
            "/tmp/wpl_check_sample_{}_{}.txt",
            std::process::id(),
            "newline"
        ));
        std::fs::write(&path, "42,alice,\n").unwrap();

        let loaded = load_sample_data(&SampleInput::File(path.clone())).unwrap();
        assert_eq!(loaded, "42,alice,\n");

        let _ = std::fs::remove_file(path);
    }
}
