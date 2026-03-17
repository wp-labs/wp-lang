use super::input::{load_input, load_sample_data, resolve_sample_input, resolve_source_path};
use super::{
    ParseResult, SampleCheckResult, SampleRequest, SourceRequest, evaluate_sample,
    validate_rule_name_usage, validate_sample_target, validate_source,
};

pub fn run_syntax_request(request: &SourceRequest) -> Result<ParseResult, String> {
    let input = resolve_source_path(request.input.as_deref());
    let (source, origin) = load_input(input.as_deref())?;
    validate_source(&source, origin.as_deref(), request.mode)
}

pub fn run_sample_request(request: &SampleRequest) -> Result<SampleCheckResult, String> {
    let source_input = resolve_source_path(request.source.input.as_deref());
    let sample_input = resolve_sample_input(&request.sample, source_input.as_deref());
    let (source, origin) = load_input(source_input.as_deref())?;
    let parsed = validate_source(&source, origin.as_deref(), request.source.mode)?;
    validate_rule_name_usage(&parsed, request.rule_name.as_deref())?;
    validate_sample_target(&parsed, request.rule_name.as_deref())?;
    let sample = load_sample_data(&sample_input)?;
    let evaluation = evaluate_sample(&parsed, request.rule_name.as_deref(), &sample)?;

    Ok(SampleCheckResult { parsed, evaluation })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::check::{Mode, SampleInput};

    #[test]
    fn test_rule_name_is_rejected_before_sample_file_read() {
        let request = SampleRequest {
            source: SourceRequest {
                mode: Mode::Rule,
                input: Some(PathBuf::from("examples/wpl-check/csv_demo/rule.wpl")),
            },
            rule_name: Some("csv_user".to_string()),
            sample: SampleInput::File(PathBuf::from("missing.txt")),
        };

        let err = run_sample_request(&request).unwrap_err();
        assert_eq!(err, "--rule-name is only valid for package source");
    }

    #[test]
    fn test_missing_rule_name_is_rejected_before_sample_file_read() {
        let request = SampleRequest {
            source: SourceRequest {
                mode: Mode::Package,
                input: Some(PathBuf::from("examples/wpl-check/package_demo/rule.wpl")),
            },
            rule_name: None,
            sample: SampleInput::File(PathBuf::from("missing.txt")),
        };

        let err = run_sample_request(&request).unwrap_err();
        assert!(err.contains("use --rule-name"));
        assert!(err.contains("available rules: csv_user, json_env"));
    }

    #[test]
    fn test_unknown_rule_name_is_rejected_before_sample_file_read() {
        let request = SampleRequest {
            source: SourceRequest {
                mode: Mode::Package,
                input: Some(PathBuf::from("examples/wpl-check/package_demo/rule.wpl")),
            },
            rule_name: Some("missing_rule".to_string()),
            sample: SampleInput::File(PathBuf::from("missing.txt")),
        };

        let err = run_sample_request(&request).unwrap_err();
        assert!(err.contains("rule 'missing_rule' not found"));
        assert!(err.contains("available rules: csv_user, json_env"));
    }
}
