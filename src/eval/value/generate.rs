#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use wp_primitives::Parser;

    use crate::parser::error::WplCodeResult;
    use crate::parser::error::IntoWplCodeError;

    use crate::parser::wpl_rule::wpl_rule;

    use crate::ParserFactory;
    use crate::generator::FieldGenConf;
    use wp_model_core::model::DataType;

    use crate::ast::{WplRule, WplSep, WplStatementType};
    use crate::generator::{FmtFieldVec, GenChannel};

    fn gen_one_line(
        log_line: &WplRule,
        ups_sep: &WplSep,
        rules: &HashMap<String, FieldGenConf>,
    ) -> WplCodeResult<FmtFieldVec> {
        let mut fieldset = FmtFieldVec::new();
        let WplStatementType::Express(rule) = &log_line.statement;
        for group in &rule.group {
            for field in &group.fields {
                let rule = field.name.clone().and_then(|name| rules.get(name.as_str()));
                let mut ch = GenChannel::new();
                let meta = DataType::from(field.meta_name.as_str()).map_err(|e| e.into_wpl_err())?;
                let parser = ParserFactory::create(&meta)?;
                let field = parser.generate(&mut ch, ups_sep, field, rule)?;
                fieldset.push(field);
            }
        }
        Ok(fieldset)
    }

    pub fn parser_by_conf(conf: &str) -> WplRule {
        wpl_rule
            .parse(conf)
            .unwrap_or_else(|_| panic!("parse conf error:{}", conf))
    }

    #[test]
    fn test_gen() -> WplCodeResult<()> {
        let conf = r#"rulegen {(digit\,,time\:,sn,chars\|)}"#;
        let conf_vec = parser_by_conf(conf);
        let rules = HashMap::new();
        let sep = WplSep::default();
        let fields = gen_one_line(&conf_vec, &sep, &rules)?;
        assert_eq!(fields.len(), 4);
        Ok(())
    }

    #[test]
    fn test_gen_ty_log() -> WplCodeResult<()> {
        let conf = r#"rule ty_log {
        (kv:message_type<:,|>,chars:sensor_log,chars[0]<{,|>,kv:serial_num,kv:access_time,kv:sip,kv:sport,
        kv:dip,kv:dport,kv:proto,kv:passwd,kv:info,kv:user,kv:db_type,kv:vendor_id,kv:device_ip,chars[0]<user_define {,|>,
        kv:name,kv:type,kv:value,chars[0]<},|>,chars[0]<},|>)
 }
"#;
        let conf_vec = parser_by_conf(conf);
        let rules = HashMap::new();
        let sep = WplSep::default();
        let fields = gen_one_line(&conf_vec, &sep, &rules)?;
        assert_eq!(fields.len(), 22);
        Ok(())
    }
}
