use super::super::prelude::*;
use wp_model_core::model::{DataField, DataType, FNameStr, Value};

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
    use orion_error::testcase::TestAssert;

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
        let (tdc, rest) = pipe.proc(0, UNKNOWN_JSON_SAMPLE, 0).map_err(|e| e.into_wpl_err())?;
        assert_eq!(rest, "");
        assert_eq!(
            tdc.field("raw").map(|f| f.as_field()),
            Some(&DataField::from_chars("raw", UNKNOWN_JSON_SAMPLE))
        );
        Ok(())
    }

    #[test]
    fn json_like_and_bad_json_work_together_end_to_end() -> WplCodeResult<()> {
        let rule = r#"rule test { |json_like| (bad_json:raw) }"#;
        let pipe = WplEvaluator::from_code(rule)?;
        let (tdc, rest) = pipe.proc(0, UNKNOWN_JSON_SAMPLE, 0).map_err(|e| e.into_wpl_err())?;
        assert_eq!(rest, "");
        assert_eq!(
            tdc.field("raw").map(|f| f.as_field()),
            Some(&DataField::from_chars("raw", UNKNOWN_JSON_SAMPLE))
        );
        Ok(())
    }
}
