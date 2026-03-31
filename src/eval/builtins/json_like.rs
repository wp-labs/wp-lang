use wp_model_core::raw::RawData;
use wp_parse_api::{WparseError, WparseReason, WparseResult};

use super::raw_to_utf8_string;

#[derive(Debug)]
pub struct JsonLikeProc;

pub(crate) fn is_json_like_text(input: &str) -> bool {
    let sample = input.trim_start_matches(|ch: char| ch.is_whitespace() || ch == '\u{feff}');
    let Some(first) = sample.chars().next() else {
        return false;
    };

    match first {
        '{' => sample.contains(':') && sample.contains('"'),
        '[' => sample.contains(',') || sample.contains(']') || sample.contains('{'),
        _ => false,
    }
}

impl wp_parse_api::PipeProcessor for JsonLikeProc {
    fn process(&self, data: RawData) -> WparseResult<RawData> {
        if is_json_like_text(&raw_to_utf8_string(&data)) {
            Ok(data)
        } else {
            Err(WparseError::from(WparseReason::NotMatch))
        }
    }

    fn name(&self) -> &'static str {
        "json_like"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AnyResult;
    use wp_parse_api::PipeProcessor;

    const UNKNOWN_JSON_SAMPLE: &str = include_str!("../../../tests/unknow.json");

    #[test]
    fn json_like_matches_valid_and_broken_json_objects() {
        assert!(is_json_like_text(r#"{"a":1}"#));
        assert!(is_json_like_text(UNKNOWN_JSON_SAMPLE));
    }

    #[test]
    fn json_like_rejects_plain_text() {
        assert!(!is_json_like_text("hello world"));
        assert!(!is_json_like_text("  2026-03-31 plain log"));
    }

    #[test]
    fn json_like_pipe_preserves_input() -> AnyResult<()> {
        let raw = RawData::from_string(UNKNOWN_JSON_SAMPLE.to_string());
        let out = JsonLikeProc.process(raw)?;
        assert_eq!(raw_to_utf8_string(&out), UNKNOWN_JSON_SAMPLE);
        Ok(())
    }

    #[test]
    fn json_like_pipe_returns_not_match_for_plain_text() {
        let raw = RawData::from_string("plain-text".to_string());
        let err = JsonLikeProc
            .process(raw)
            .expect_err("plain text should not match");
        assert_eq!(*err.reason(), WparseReason::NotMatch);
    }
}
