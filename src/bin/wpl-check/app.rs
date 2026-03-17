use std::env;

use wpl::check::{normalized_output, run_sample_request, run_syntax_request, source_summary};

use crate::cli::{help_text, parse_args};
use crate::model::{Cli, Command, SampleConfig, SourceConfig};

pub(crate) fn run() -> Result<(), String> {
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
    let parsed = run_syntax_request(&config.request)?;

    println!("{}", source_summary(&parsed));
    if config.print_source {
        println!();
        println!("{}", normalized_output(&parsed));
    }
    Ok(())
}

fn run_sample(config: SampleConfig) -> Result<(), String> {
    let result = run_sample_request(&config.request)?;

    println!("{}", source_summary(&result.parsed));
    if config.print_source {
        println!();
        println!("{}", normalized_output(&result.parsed));
    }
    println!();
    println!(
        "data: ok ({}, {} fields, {} bytes residue)",
        result.evaluation.target,
        result.evaluation.field_count,
        result.evaluation.residue.len()
    );
    println!();
    println!("{}", result.evaluation.record);
    if !result.evaluation.residue.is_empty() {
        println!();
        println!("residue:");
        println!("{}", result.evaluation.residue);
    }
    Ok(())
}
