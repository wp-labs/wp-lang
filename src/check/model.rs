use std::path::PathBuf;

use crate::{WplExpress, WplPackage, WplRule};

pub const DEFAULT_RULE_FILE: &str = "rule.wpl";
pub const DEFAULT_SAMPLE_FILE: &str = "sample.txt";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mode {
    Auto,
    Package,
    Rule,
    Expr,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SourceRequest {
    pub mode: Mode,
    pub input: Option<PathBuf>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SampleRequest {
    pub source: SourceRequest,
    pub rule_name: Option<String>,
    pub sample: SampleInput,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SampleInput {
    Inline(String),
    DefaultFile,
    File(PathBuf),
}

#[derive(Debug)]
pub enum ParseResult {
    Package(WplPackage),
    Rule(WplRule),
    Expr(WplExpress),
}

#[derive(Debug)]
pub struct EvalResult {
    pub target: String,
    pub record: String,
    pub residue: String,
    pub field_count: usize,
}

#[derive(Debug)]
pub struct SampleCheckResult {
    pub parsed: ParseResult,
    pub evaluation: EvalResult,
}
