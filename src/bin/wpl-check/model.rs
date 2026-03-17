pub(crate) use wpl::check::{DEFAULT_RULE_FILE, Mode, SampleInput, SampleRequest, SourceRequest};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HelpTopic {
    Global,
    Syntax,
    Sample,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Cli {
    Help(HelpTopic),
    Command(Command),
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum Command {
    Syntax(SourceConfig),
    Sample(SampleConfig),
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct SourceConfig {
    pub(crate) request: SourceRequest,
    pub(crate) print_source: bool,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct SampleConfig {
    pub(crate) request: SampleRequest,
    pub(crate) print_source: bool,
}
