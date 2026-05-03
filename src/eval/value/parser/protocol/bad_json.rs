use super::super::prelude::*;
use crate::eval::builtins::json_like::is_json_like_text;
use serde::Deserialize;
use serde_json::{Deserializer, Value as JsonValue};
use std::io::Cursor;
use wp_model_core::model::{DataField, DataType, FNameStr, Value};
use wp_primitives::symbol::ctx_desc;

use crate::eval::runtime::field::FieldEvalUnit;
use crate::eval::value::parse_def::PatternParser;

#[derive(Default)]
pub struct BadJsonP {}

impl PatternParser for BadJsonP {
    fn pattern_parse<'a>(
        &self,
        _e_id: u64,
        _fpu: &FieldEvalUnit,
        _ups_sep: &WplSep,
        data: &mut &str,
        name: FNameStr,
        out: &mut Vec<DataField>,
    ) -> ModalResult<()> {
        if !is_json_like_text(data) {
            return fail.parse_next(data);
        }

        let mut cursor = Cursor::new(data.as_bytes());
        let mut deserializer = Deserializer::from_reader(&mut cursor);
        if JsonValue::deserialize(&mut deserializer).is_ok() {
            return fail
                .context(ctx_desc("bad_json does not match valid json input"))
                .parse_next(data);
        }

        out.push(DataField::new_opt(
            DataType::Chars,
            Some(name),
            Value::Chars((*data).into()),
        ));
        *data = "";
        Ok(())
    }

    fn patten_gen(
        &self,
        _gen: &mut GenChannel,
        _f_conf: &WplField,
        _g_conf: Option<&FieldGenConf>,
    ) -> WplCodeResult<DataField> {
        unimplemented!("bad_json generate")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::WplField;
    use crate::eval::runtime::vm_unit::WplEvaluator;
    use crate::eval::value::test_utils::ParserTUnit;
    use crate::parser::error::WplCodeResult;
    use orion_error::dev::testing::TestAssert;

    const UNKNOWN_JSON_SAMPLE: &str = include_str!("../../../../../tests/unknow.json");

    use crate::parser::error::IntoWplCodeError;
    #[test]
    fn bad_json_parser_outputs_raw_chars() {
        let mut data = UNKNOWN_JSON_SAMPLE;
        let conf = WplField::try_parse("bad_json:raw").assert();
        let out = ParserTUnit::new(BadJsonP::default(), conf)
            .verify_parse_suc(&mut data)
            .assert();
        assert_eq!(out[0], DataField::from_chars("raw", UNKNOWN_JSON_SAMPLE));
    }

    #[test]
    fn evaluator_supports_bad_json_alias() -> WplCodeResult<()> {
        let rule = r#"rule test { (bad_json:raw) }"#;
        let pipe = WplEvaluator::from_code(rule)?;
        let (tdc, rest) = pipe
            .proc(0, UNKNOWN_JSON_SAMPLE, 0)
            .map_err(|e| e.into_wpl_err())?;
        assert_eq!(rest, "");
        assert_eq!(
            tdc.field("raw").map(|f| f.as_field()),
            Some(&DataField::from_chars("raw", UNKNOWN_JSON_SAMPLE))
        );
        Ok(())
    }

    #[test]
    fn bad_json_rejects_valid_json() -> WplCodeResult<()> {
        let rule = r#"rule test { (bad_json:raw) }"#;
        let pipe = WplEvaluator::from_code(rule)?;
        let err = pipe
            .proc(0, r#"{"host":"ok","method":"POST"}"#, 0)
            .expect_err("valid json should not match bad_json");
        let detail = err
            .detail()
            .as_ref()
            .expect("bad_json valid-json rejection should carry detail");
        assert!(detail.contains("bad_json does not match valid json input"));
        Ok(())
    }

    #[test]
    fn bad_json_rejects_plain_text() -> WplCodeResult<()> {
        let rule = r#"rule test { (bad_json:raw) }"#;
        let pipe = WplEvaluator::from_code(rule)?;
        assert!(pipe.proc(0, "plain text log line", 0).is_err());
        Ok(())
    }

    #[test]
    fn json_like_and_bad_json_work_together_end_to_end() -> WplCodeResult<()> {
        let rule = r#"rule test { |json_like| (bad_json:raw) }"#;
        let pipe = WplEvaluator::from_code(rule)?;
        let (tdc, rest) = pipe
            .proc(0, UNKNOWN_JSON_SAMPLE, 0)
            .map_err(|e| e.into_wpl_err())?;
        assert_eq!(rest, "");
        assert_eq!(
            tdc.field("raw").map(|f| f.as_field()),
            Some(&DataField::from_chars("raw", UNKNOWN_JSON_SAMPLE))
        );
        Ok(())
    }

    #[test]
    fn json_like_and_bad_json_reject_valid_json_end_to_end() -> WplCodeResult<()> {
        let rule = r#"rule test { |json_like| (bad_json:raw) }"#;
        let pipe = WplEvaluator::from_code(rule)?;
        assert!(pipe.proc(0, r#"{"host":"ok","method":"POST"}"#, 0).is_err());
        Ok(())
    }
}
