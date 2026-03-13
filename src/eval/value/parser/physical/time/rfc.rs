use super::common::parse_fixed;
use crate::ast::WplSep;
use crate::eval::runtime::field::FieldEvalUnit;
use crate::eval::value::parse_def::PatternParser;
use crate::eval::value::parser::physical::time::gen_time;
use crate::generator::{FieldGenConf, GenChannel};
use crate::types::AnyResult;
use crate::winnow::Parser;
use chrono::format::Fixed;
use chrono::{Datelike, NaiveDate, NaiveTime};
use winnow::ascii::{alpha1, digit1, multispace0, multispace1};
use winnow::combinator::{alt, dispatch, fail, opt, peek, preceded};
use winnow::error::StrContext;
use winnow::stream::Stream as _;
use winnow::token::literal;
use wp_model_core::model::FNameStr;
use wp_model_core::model::{DataField, DateTimeValue};
use wp_primitives::WResult;
use wp_primitives::symbol::ctx_desc;

pub fn parse_rfc3339(data: &mut &str) -> WResult<DateTimeValue> {
    let items = &[chrono::format::Item::Fixed(Fixed::RFC3339)];
    // `map_err(|e| e.into())` is a no-op here; just use `?`
    let dt = parse_fixed(data, items)?;
    Ok(dt.naive_local())
}

pub fn parse_rfc2822(data: &mut &str) -> WResult<DateTimeValue> {
    let items = &[chrono::format::Item::Fixed(Fixed::RFC2822)];
    let dt = parse_fixed(data, items)?;
    Ok(dt.naive_local())
}

pub fn parse_time(data: &mut &str) -> WResult<DateTimeValue> {
    alt((
        parse_rfc3339.context(StrContext::Label("parse_rfc3339")),
        parse_rfc2822.context(StrContext::Label("parse_rfc2822")),
        parse_timep.context(StrContext::Label("parse_timep")),
    ))
    .parse_next(data)
}

fn parse_timep(data: &mut &str) -> WResult<DateTimeValue> {
    parse_timep_inner(data, true)
}

fn parse_timep_inner(data: &mut &str, allow_fraction: bool) -> WResult<DateTimeValue> {
    let date = preceded(
        multispace0,
        alt((parse_date_1, parse_date_2, parse_date_3, parse_date_4)),
    )
    .parse_next(data)?;
    let (h, min, s) = (multispace0, digit1, ":", digit1, ":", digit1)
        .map(|x| (x.1, x.3, x.5))
        .parse_next(data)?;
    let nanos = if allow_fraction {
        opt(parse_fraction).parse_next(data)?.unwrap_or(0)
    } else {
        0
    };
    let _ = opt(alt((parse_zone_utc_z, parse_zone_2))).parse_next(data)?;

    let hour = parse_u32_digits(h).ok_or_else(|| {
        let cp = (*data).checkpoint();
        winnow::error::ErrMode::Backtrack(wp_primitives::utils::context_error(
            data,
            &cp,
            "time parse fail",
        ))
    })?;
    let minute = parse_u32_digits(min).ok_or_else(|| {
        let cp = (*data).checkpoint();
        winnow::error::ErrMode::Backtrack(wp_primitives::utils::context_error(
            data,
            &cp,
            "time parse fail",
        ))
    })?;
    let second = parse_u32_digits(s).ok_or_else(|| {
        let cp = (*data).checkpoint();
        winnow::error::ErrMode::Backtrack(wp_primitives::utils::context_error(
            data,
            &cp,
            "time parse fail",
        ))
    })?;
    let time = NaiveTime::from_hms_nano_opt(hour, minute, second, nanos).ok_or_else(|| {
        let cp = (*data).checkpoint();
        winnow::error::ErrMode::Backtrack(wp_primitives::utils::context_error(
            data,
            &cp,
            "time parse fail",
        ))
    })?;
    Ok(date.and_time(time))
}

// ----- helpers moved from old time.rs -----

fn parse_fraction(data: &mut &str) -> WResult<u32> {
    preceded(".", digit1)
        .map(|digits: &str| fraction_to_nanos(digits))
        .parse_next(data)
}

fn fraction_to_nanos(digits: &str) -> u32 {
    let kept = &digits[..digits.len().min(9)];
    let nanos = parse_u32_digits(kept).unwrap_or(0);
    nanos * 10u32.pow((9 - kept.len()) as u32)
}

fn parse_u32_digits(digits: &str) -> Option<u32> {
    let mut value = 0u32;
    for &b in digits.as_bytes() {
        if !b.is_ascii_digit() {
            return None;
        }
        value = value.checked_mul(10)?.checked_add((b - b'0') as u32)?;
    }
    Some(value)
}

fn parse_zone_utc_z<'a>(data: &mut &'a str) -> WResult<&'a str> {
    literal("Z").parse_next(data)
}

fn parse_zone_2<'a>(data: &mut &'a str) -> WResult<&'a str> {
    let flag = alt((literal("+"), literal("-")));
    (multispace0, flag, digit1).map(|x| x.2).parse_next(data)
}

//2023-05-31 00:22:10.894600Z
//06/Aug/2019
fn parse_date_2(data: &mut &str) -> WResult<NaiveDate> {
    let (_, d, _, m, _, y) =
        (multispace0, digit1, "/", month_patten, "/", digit1).parse_next(data)?;
    let _ = opt(literal(":")).parse_next(data)?;
    let day = parse_u32_digits(d).ok_or_else(|| {
        let cp = (*data).checkpoint();
        winnow::error::ErrMode::Backtrack(wp_primitives::utils::context_error(
            data,
            &cp,
            "time parse fail",
        ))
    })?;
    let year = parse_i32_digits(y).ok_or_else(|| {
        let cp = (*data).checkpoint();
        winnow::error::ErrMode::Backtrack(wp_primitives::utils::context_error(
            data,
            &cp,
            "time parse fail",
        ))
    })?;
    let month = month_to_u32(m).ok_or_else(|| {
        let cp = (*data).checkpoint();
        winnow::error::ErrMode::Backtrack(wp_primitives::utils::context_error(
            data,
            &cp,
            "time parse fail",
        ))
    })?;
    NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| {
        let cp = (*data).checkpoint();
        winnow::error::ErrMode::Backtrack(wp_primitives::utils::context_error(
            data,
            &cp,
            "time parse fail",
        ))
    })
}

fn parse_date_1(data: &mut &str) -> WResult<NaiveDate> {
    let sep = alt((literal("-"), literal("/")));
    let sep2 = alt((literal("-"), literal("/")));
    let (_, y, _, m_digit, _, d) = (
        multispace0,
        digit1,
        sep,
        digit1.try_map(str::parse::<u32>),
        sep2,
        digit1,
    )
        .parse_next(data)?;
    let year = parse_i32_digits(y).ok_or_else(|| {
        let cp = (*data).checkpoint();
        winnow::error::ErrMode::Backtrack(wp_primitives::utils::context_error(
            data,
            &cp,
            "time parse fail",
        ))
    })?;
    let day = parse_u32_digits(d).ok_or_else(|| {
        let cp = (*data).checkpoint();
        winnow::error::ErrMode::Backtrack(wp_primitives::utils::context_error(
            data,
            &cp,
            "time parse fail",
        ))
    })?;
    NaiveDate::from_ymd_opt(year, m_digit, day).ok_or_else(|| {
        let cp = (*data).checkpoint();
        winnow::error::ErrMode::Backtrack(wp_primitives::utils::context_error(
            data,
            &cp,
            "time parse fail",
        ))
    })
}

//May 17 08:28:12
fn parse_date_4(data: &mut &str) -> WResult<NaiveDate> {
    let (_, m, _, d, _) =
        (multispace0, month_patten, multispace1, digit1, multispace1).parse_next(data)?;
    let year = chrono::Local::now().year();
    let day = parse_u32_digits(d).ok_or_else(|| {
        let cp = (*data).checkpoint();
        winnow::error::ErrMode::Backtrack(wp_primitives::utils::context_error(
            data,
            &cp,
            "time parse fail",
        ))
    })?;
    let month = month_to_u32(m).ok_or_else(|| {
        let cp = (*data).checkpoint();
        winnow::error::ErrMode::Backtrack(wp_primitives::utils::context_error(
            data,
            &cp,
            "time parse fail",
        ))
    })?;
    NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| {
        let cp = (*data).checkpoint();
        winnow::error::ErrMode::Backtrack(wp_primitives::utils::context_error(
            data,
            &cp,
            "time parse fail",
        ))
    })
}

//May 15 2023
fn parse_date_3(data: &mut &str) -> WResult<NaiveDate> {
    let (m, _, d, _, y) =
        (month_patten, multispace1, digit1, multispace1, digit1).parse_next(data)?;
    let day = parse_u32_digits(d).ok_or_else(|| {
        let cp = (*data).checkpoint();
        winnow::error::ErrMode::Backtrack(wp_primitives::utils::context_error(
            data,
            &cp,
            "time parse fail",
        ))
    })?;
    let year = parse_u32_digits(y)
        .filter(|year| *year > 1970)
        .ok_or_else(|| {
            let cp = (*data).checkpoint();
            winnow::error::ErrMode::Backtrack(wp_primitives::utils::context_error(
                data,
                &cp,
                "time parse fail",
            ))
        })?;
    let month = month_to_u32(m).ok_or_else(|| {
        let cp = (*data).checkpoint();
        winnow::error::ErrMode::Backtrack(wp_primitives::utils::context_error(
            data,
            &cp,
            "time parse fail",
        ))
    })?;
    NaiveDate::from_ymd_opt(year as i32, month, day).ok_or_else(|| {
        let cp = (*data).checkpoint();
        winnow::error::ErrMode::Backtrack(wp_primitives::utils::context_error(
            data,
            &cp,
            "time parse fail",
        ))
    })
}

fn parse_i32_digits(digits: &str) -> Option<i32> {
    let value = parse_u32_digits(digits)?;
    i32::try_from(value).ok()
}

fn month_to_u32(month: &str) -> Option<u32> {
    match month {
        "Jan" => Some(1),
        "Feb" => Some(2),
        "Mar" => Some(3),
        "Apr" => Some(4),
        "May" => Some(5),
        "Jun" => Some(6),
        "Jul" => Some(7),
        "Aug" => Some(8),
        "Sep" => Some(9),
        "Oct" => Some(10),
        "Nov" => Some(11),
        "Dec" => Some(12),
        _ => None,
    }
}

//Sat Jun  3 20:17:21 2023
fn month_patten<'a>(input: &mut &'a str) -> WResult<&'a str> {
    dispatch!( peek(alpha1);
        "Jan" => alpha1,
        "Feb" => alpha1,
        "Mar" => alpha1,
        "Apr" => alpha1,
        "May" => alpha1,
        "Jun" => alpha1,
        "Jul" => alpha1,
        "Aug" => alpha1,
        "Sep" => alpha1,
        "Oct" => alpha1,
        "Nov" => alpha1,
        "Dec" => alpha1,
        _ => fail,
    )
    .parse_next(input)
}

#[derive(Default)]
pub struct TimeP {}
#[derive(Default)]
pub struct TimeISOP {}
#[derive(Default)]
pub struct TimeRFC3339 {}
#[derive(Default)]
pub struct TimeRFC2822 {}

impl PatternParser for TimeP {
    fn pattern_parse(
        &self,
        _e_id: u64,
        _fpu: &FieldEvalUnit,
        ups_sep: &crate::ast::WplSep,
        data: &mut &str,
        name: FNameStr,
        out: &mut Vec<DataField>,
    ) -> WResult<()> {
        let keep_fraction = !explicit_dot_separator(ups_sep);
        let time = alt((
            parse_rfc3339.context(StrContext::Label("<rfc3339>")),
            parse_rfc2822.context(StrContext::Label("<rfc2822>")),
            (|input: &mut &str| parse_timep_inner(input, keep_fraction))
                .context(StrContext::Label("parse_timep")),
        ))
        .context(ctx_desc("<time>"))
        .parse_next(data)?;
        out.push(DataField::from_time(name, time));
        Ok(())
    }

    fn patten_gen(
        &self,
        gnc: &mut GenChannel,
        f_conf: &crate::ast::WplField,
        g_conf: Option<&FieldGenConf>,
    ) -> AnyResult<DataField> {
        gen_time(gnc, f_conf, g_conf)
    }
}

impl PatternParser for TimeISOP {
    fn pattern_parse(
        &self,
        _e_id: u64,
        _fpu: &FieldEvalUnit,
        _: &crate::ast::WplSep,
        data: &mut &str,
        name: FNameStr,
        out: &mut Vec<DataField>,
    ) -> WResult<()> {
        let time = parse_rfc3339.parse_next(data)?;
        out.push(DataField::from_time(name, time));
        Ok(())
    }
    fn patten_gen(
        &self,
        gnc: &mut GenChannel,
        f_conf: &crate::ast::WplField,
        g_conf: Option<&FieldGenConf>,
    ) -> AnyResult<DataField> {
        gen_time(gnc, f_conf, g_conf)
    }
}
impl PatternParser for TimeRFC3339 {
    fn pattern_parse(
        &self,
        e_id: u64,
        fpu: &FieldEvalUnit,
        s: &crate::ast::WplSep,
        d: &mut &str,
        n: FNameStr,
        o: &mut Vec<DataField>,
    ) -> WResult<()> {
        TimeISOP {}.pattern_parse(e_id, fpu, s, d, n, o)
    }
    fn patten_gen(
        &self,
        g: &mut GenChannel,
        f: &crate::ast::WplField,
        c: Option<&FieldGenConf>,
    ) -> AnyResult<DataField> {
        gen_time(g, f, c)
    }
}
impl PatternParser for TimeRFC2822 {
    fn pattern_parse(
        &self,
        _e_id: u64,
        _: &FieldEvalUnit,
        _: &crate::ast::WplSep,
        data: &mut &str,
        name: FNameStr,
        out: &mut Vec<DataField>,
    ) -> WResult<()> {
        let time = parse_rfc2822.parse_next(data)?;
        out.push(DataField::from_time(name, time));
        Ok(())
    }
    fn patten_gen(
        &self,
        gnc: &mut GenChannel,
        f_conf: &crate::ast::WplField,
        g_conf: Option<&FieldGenConf>,
    ) -> AnyResult<DataField> {
        gen_time(gnc, f_conf, g_conf)
    }
}

fn explicit_dot_separator(sep: &WplSep) -> bool {
    sep.need_take_sep() && !sep.is_pattern() && sep.sep_str() == "."
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::WplSepT;

    #[test]
    fn parse_timep_preserves_fractional_seconds() {
        let mut input = "2026-01-17 18:38:51.468263000 [INFO]";
        let dt = parse_timep_inner(&mut input, true).expect("parse time with fraction");
        assert_eq!(
            dt,
            NaiveDate::from_ymd_opt(2026, 1, 17)
                .unwrap()
                .and_hms_nano_opt(18, 38, 51, 468_263_000)
                .unwrap()
        );
        assert_eq!(input, " [INFO]");
    }

    #[test]
    fn parse_timep_keeps_legacy_dot_split_when_separator_is_explicit_dot() {
        let mut input = "2026-01-17 18:38:51.468263000 [INFO]";
        let dt = parse_timep_inner(
            &mut input,
            !explicit_dot_separator(&WplSepT::field_sep(".")),
        )
        .expect("parse time without fraction when split by dot");
        assert_eq!(
            dt,
            NaiveDate::from_ymd_opt(2026, 1, 17)
                .unwrap()
                .and_hms_opt(18, 38, 51)
                .unwrap()
        );
        assert_eq!(input, ".468263000 [INFO]");
    }
}
