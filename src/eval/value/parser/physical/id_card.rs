use super::super::prelude::*;
use crate::derive_base_prs;
use crate::eval::runtime::field::FieldEvalUnit;
use crate::eval::value::parse_def::PatternParser;
use crate::generator::FieldGenConf;
use crate::idcard::fake::new;
use crate::idcard::{Gender, Identity};
use crate::parser::error::IntoWplCodeError;
use smol_str::SmolStr;
use wp_model_core::model::FNameStr;
use wp_model_core::model::Value;

derive_base_prs!(IdCardP);

impl PatternParser for IdCardP {
    fn pattern_parse(
        &self,
        _e_id: u64,
        _fpu: &FieldEvalUnit,
        _ups_sep: &WplSep,
        data: &mut &str,
        name: FNameStr,
        out: &mut Vec<DataField>,
    ) -> ModalResult<()> {
        let start = data.checkpoint();
        let id_card = alphanumeric1
            .context(ctx_desc("<id_card>"))
            .parse_next(data)?;
        if Identity::new(id_card).is_valid() {
            out.push(DataField::from_id_card(name, id_card));
            Ok(())
        } else {
            Err(ErrMode::Backtrack(context_error(
                data,
                &start,
                "id_card format not match",
            )))
        }
    }

    fn patten_gen(
        &self,
        _gen: &mut GenChannel,
        _f_conf: &WplField,
        _g_conf: Option<&FieldGenConf>,
    ) -> WplCodeResult<DataField> {
        let id = new("310104", 2020, 2, 29, Gender::Male).map_err(|e| e.into_wpl_err())?;
        Ok(DataField::new(
            DataType::IdCard,
            "id_card",
            Value::Chars(SmolStr::from(id)),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::value::test_utils::ParserTUnit;
    use orion_error::testcase::TestAssert;

    #[test]
    fn test_id_card() {
        let mut data = "310104202002299069";
        let y = ParserTUnit::new(IdCardP::default(), WplField::try_parse("id_card").assert())
            .verify_parse_suc(&mut data)
            .assert();
        assert_eq!(
            y.first(),
            Some(&DataField::from_id_card("id_card", "310104202002299069"))
        );
    }
}
