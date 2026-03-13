use crate::DataTypeParser;
use smol_str::SmolStr;
use std::fmt::Display;
use winnow::ascii::multispace0;
use winnow::combinator::fail;
use winnow::error::StrContext;
use winnow::stream::Stream;
use winnow::token::take_while;
use wp_model_core::model::FNameStr;
use wp_model_core::model::{DataField, DataType};
use wp_primitives::Parser;
use wp_primitives::WResult;
use wp_primitives::atom::{take_parentheses_val, take_var_name};
use wp_primitives::symbol::ctx_desc;

fn take_meta_name<'a>(input: &mut &'a str) -> WResult<&'a str> {
    take_while(1.., |c: char| c.is_alphanumeric() || c == '_' || c == '/').parse_next(input)
}

pub fn take_datatype(data: &mut &str) -> WResult<DataType> {
    take_datatype_impl
        .context(StrContext::Label("<datatype>"))
        .parse_next(data)
}

pub fn take_datatype_impl(data: &mut &str) -> WResult<DataType> {
    let _ = multispace0.parse_next(data)?;
    let cp = data.checkpoint();
    let meta_str = take_meta_name.parse_next(data)?;
    if let Ok(meta) = DataType::from(meta_str) {
        Ok(meta)
    } else {
        data.reset(&cp);
        fail.context(ctx_desc("DataType from str fail"))
            .parse_next(&mut "")
    }
}

pub fn field_ins<N: Into<FNameStr>, V: Into<SmolStr> + Display>(
    meta: DataType,
    name: N,
    val: V,
) -> WResult<DataField> {
    if let Ok(tdo) = DataField::from_str(meta, name, val) {
        Ok(tdo)
    } else {
        fail.context(ctx_desc("DataField from str fail"))
            .parse_next(&mut "")
    }
}

pub fn take_field(data: &mut &str) -> WResult<DataField> {
    // Typed literals are nameless values: keep only meta + value.
    let key = take_var_name.parse_next(data)?;
    let value = take_parentheses_val.parse_next(data)?;
    let mut key_for_meta = key;
    let meta = take_datatype(&mut key_for_meta)?;
    let target = field_ins(meta, "", &value)?;
    Ok(target)
}
