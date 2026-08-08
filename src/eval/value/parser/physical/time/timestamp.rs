use crate::eval::runtime::field::FieldEvalUnit;
use crate::eval::value::parse_def::PatternParser;
use crate::generator::{FieldGenConf, GenChannel};
use crate::parser::error::WplCodeResult;
use crate::winnow::Parser;
use winnow::ascii::digit1;
use winnow::combinator::alt;
use winnow::stream::Stream as _;
use winnow::token::take;
use wp_model_core::model::DataField;
use wp_model_core::model::FNameStr;
use wp_primitives::WResult;

#[derive(Default)]
pub struct TimeStampPSR {}

impl PatternParser for TimeStampPSR {
    fn pattern_parse(
        &self,
        _e_id: u64,
        _: &FieldEvalUnit,
        _: &crate::ast::WplSep,
        data: &mut &str,
        name: FNameStr,
        out: &mut Vec<DataField>,
    ) -> WResult<()> {
        let dt = alt((parse_timestamp_us, parse_timestamp_ms, parse_timestamp)).parse_next(data)?;
        out.push(DataField::from_time(name, dt.naive_local()));
        Ok(())
    }
    fn patten_gen(
        &self,
        gnc: &mut GenChannel,
        f_conf: &crate::ast::WplField,
        g_conf: Option<&FieldGenConf>,
    ) -> WplCodeResult<DataField> {
        super::gen_time(gnc, f_conf, g_conf)
    }
}

// ----- helpers moved from old time.rs -----
fn parse_timestamp(data: &mut &str) -> WResult<chrono::DateTime<chrono::Utc>> {
    let ts_s = digit1.parse_next(data)?;
    // 1~10 位整数按秒解析（0 → Unix epoch）；超过 10 位交给 ms/us 分支或报错
    if ts_s.len() > 10 {
        let cp = (*data).checkpoint();
        return Err(winnow::error::ErrMode::Backtrack(
            wp_primitives::utils::context_error(data, &cp, "timestamp fail"),
        ));
    }
    if let Ok(Some(dt)) = ts_s.parse().map(|x| chrono::DateTime::from_timestamp(x, 0)) {
        Ok(dt)
    } else {
        let cp = (*data).checkpoint();
        Err(winnow::error::ErrMode::Backtrack(
            wp_primitives::utils::context_error(data, &cp, "timestamp fail"),
        ))
    }
}
fn parse_timestamp_ms(data: &mut &str) -> WResult<chrono::DateTime<chrono::Utc>> {
    let ts_ms = take(13usize).parse_next(data)?;
    if let Ok(Some(dt)) = ts_ms.parse().map(chrono::DateTime::from_timestamp_millis) {
        Ok(dt)
    } else {
        let cp = (*data).checkpoint();
        Err(winnow::error::ErrMode::Backtrack(
            wp_primitives::utils::context_error(data, &cp, "timestamp_millis fail"),
        ))
    }
}
fn parse_timestamp_us(data: &mut &str) -> WResult<chrono::DateTime<chrono::Utc>> {
    let ts_us = take(16usize).parse_next(data)?;
    if let Ok(Some(dt)) = ts_us.parse().map(chrono::DateTime::from_timestamp_micros) {
        Ok(dt)
    } else {
        let cp = (*data).checkpoint();
        Err(winnow::error::ErrMode::Backtrack(
            wp_primitives::utils::context_error(data, &cp, "timestamp_micros fail"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_timestamp, parse_timestamp_ms, parse_timestamp_us};
    use crate::winnow::Parser;

    fn ts_eq(input: &str, expected: i64) {
        let mut data = input;
        let dt = parse_timestamp.parse_next(&mut data).unwrap();
        assert_eq!(dt.timestamp(), expected);
        assert_eq!(data, "", "parse_timestamp 应消费完整输入");
    }

    #[test]
    fn test_timestamp_zero_is_epoch() {
        // issue #347：0 → Unix epoch
        ts_eq("0", 0);
        // 补零到 10 位同样为 epoch
        ts_eq("0000000000", 0);
    }

    #[test]
    fn test_timestamp_short_seconds() {
        // 1~9 位按秒解析
        ts_eq("1", 1);
        ts_eq("1652174567", 1652174567); // 10 位秒级
    }

    #[test]
    fn test_timestamp_unit_regression() {
        // 1652174567 秒 / 1652174567000 毫秒 / 1652174567000000 微秒 → 同一时刻 2022-05-10 09:22:47
        let mut s = "1652174567";
        let dt = parse_timestamp.parse_next(&mut s).unwrap();
        assert_eq!(dt.timestamp(), 1652174567);

        let mut ms = "1652174567000";
        let dt = parse_timestamp_ms.parse_next(&mut ms).unwrap();
        assert_eq!(dt.timestamp_millis(), 1652174567000);

        let mut us = "1652174567000000";
        let dt = parse_timestamp_us.parse_next(&mut us).unwrap();
        assert_eq!(dt.timestamp_micros(), 1652174567000000);
    }

    #[test]
    fn test_timestamp_too_long_seconds_fails() {
        use winnow::combinator::alt;
        // 11 位不按秒处理，也不应部分消费（原 take(10) 会漏字符给下游）
        let mut data = "12345678901";
        assert!(parse_timestamp.parse_next(&mut data).is_err());
        // us/ms 也匹配不上（11 位），整体应失败
        let mut all = "12345678901";
        assert!(alt((parse_timestamp_us, parse_timestamp_ms, parse_timestamp))
            .parse_next(&mut all)
            .is_err());
    }
}
