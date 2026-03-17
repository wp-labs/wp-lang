use std::path::PathBuf;

use crate::model::{
    Cli, Command, DEFAULT_RULE_FILE, HelpTopic, Mode, SampleConfig, SampleInput, SampleRequest,
    SourceConfig, SourceRequest,
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

pub(crate) fn parse_args<I>(args: I) -> Result<Cli, String>
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

pub(crate) fn help_text(topic: HelpTopic) -> &'static str {
    match topic {
        HelpTopic::Global => HELP,
        HelpTopic::Syntax => SYNTAX_HELP,
        HelpTopic::Sample => SAMPLE_HELP,
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
        request: SourceRequest {
            mode,
            input: Some(input.unwrap_or_else(|| PathBuf::from(DEFAULT_RULE_FILE))),
        },
        print_source,
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
        request: SampleRequest {
            source: SourceRequest {
                mode,
                input: Some(source_input),
            },
            rule_name,
            sample,
        },
        print_source,
    })))
}

fn next_value<I>(args: &mut I, flag: &str, help: &str) -> Result<String, String>
where
    I: Iterator<Item = String>,
{
    args.next()
        .ok_or_else(|| format!("missing value for {flag}\n\n{help}"))
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
                request: SourceRequest {
                    mode: Mode::Rule,
                    input: Some(PathBuf::from("demo.wpl")),
                },
                print_source: true,
            }))
        );
    }

    #[test]
    fn test_parse_args_syntax_defaults_to_rule_file() {
        let cli = parse_args(["syntax"].into_iter().map(str::to_string)).unwrap();

        assert_eq!(
            cli,
            Cli::Command(Command::Syntax(SourceConfig {
                request: SourceRequest {
                    mode: Mode::Auto,
                    input: Some(PathBuf::from("rule.wpl")),
                },
                print_source: false,
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
                request: SampleRequest {
                    source: SourceRequest {
                        mode: Mode::Package,
                        input: Some(PathBuf::from("demo.wpl")),
                    },
                    rule_name: Some("demo_rule".to_string()),
                    sample: SampleInput::File(PathBuf::from("sample.txt")),
                },
                print_source: false,
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
                request: SampleRequest {
                    source: SourceRequest {
                        mode: Mode::Rule,
                        input: Some(PathBuf::from("demo.wpl")),
                    },
                    rule_name: None,
                    sample: SampleInput::Inline("1,alice".to_string()),
                },
                print_source: false,
            }))
        );
    }

    #[test]
    fn test_parse_args_sample_defaults_to_rule_and_sample_files() {
        let cli = parse_args(["sample"].into_iter().map(str::to_string)).unwrap();

        assert_eq!(
            cli,
            Cli::Command(Command::Sample(SampleConfig {
                request: SampleRequest {
                    source: SourceRequest {
                        mode: Mode::Auto,
                        input: Some(PathBuf::from("rule.wpl")),
                    },
                    rule_name: None,
                    sample: SampleInput::DefaultFile,
                },
                print_source: false,
            }))
        );
    }
}
