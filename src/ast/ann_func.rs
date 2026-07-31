use crate::WparseError;
use crate::WplEvaluator;
use crate::ast::AnnFun;
use smol_str::SmolStr;
use std::collections::BTreeMap;
use wp_model_core::model::{DataField, DataRecord};
use wp_model_core::raw::RawData;
use wp_source_types::SourceEvent;

/// 注解执行产出的旁路 record：(wpl_key, record)。
/// 由 engine 层按 `wpl_key` 独立路由到对应 sink（不并入当前 rule 的 record）。
/// 目前仅 `CopyEventParse` 产出旁路 record——目标 rule 的解析结果作为一条新 record 流出。
pub type SideRecords = Vec<(SmolStr, DataRecord)>;

pub trait AnnotationFunc {
    /// 执行注解：就地修改 `data`（当前 record），并返回要独立路由的旁路 record。
    fn proc(&self, src: &SourceEvent, data: &mut DataRecord) -> Result<SideRecords, WparseError>;
}

#[derive(Clone, Debug)]
pub struct TagAnnotation {
    args: BTreeMap<SmolStr, SmolStr>,
}

impl AnnotationFunc for TagAnnotation {
    fn proc(&self, _src: &SourceEvent, data: &mut DataRecord) -> Result<SideRecords, WparseError> {
        for (key, val) in &self.args {
            data.append(DataField::from_chars(key.clone(), val.clone()));
        }
        Ok(SideRecords::new())
    }
}

#[derive(Clone, Debug)]
pub struct NoopAnnotation;

impl AnnotationFunc for NoopAnnotation {
    fn proc(&self, _src: &SourceEvent, _data: &mut DataRecord) -> Result<SideRecords, WparseError> {
        Ok(SideRecords::new())
    }
}

#[derive(Clone, Debug)]
pub struct RawCopy {
    raw_key: SmolStr,
}

impl AnnotationFunc for RawCopy {
    fn proc(&self, src: &SourceEvent, data: &mut DataRecord) -> Result<SideRecords, WparseError> {
        match &src.payload {
            RawData::String(raw) => {
                data.append(DataField::from_chars(self.raw_key.clone(), raw.clone()));
            }
            RawData::Bytes(raw) => {
                data.append(DataField::from_chars(
                    self.raw_key.clone(),
                    String::from_utf8_lossy(raw).into_owned(),
                ));
            }
            RawData::ArcBytes(raw) => {
                data.append(DataField::from_chars(
                    self.raw_key.clone(),
                    String::from_utf8_lossy(raw).into_owned(),
                ));
            }
        }
        Ok(SideRecords::new())
    }
}

/// 将原始 payload 复制给指定 rule 的 parser 解析，产出一条**独立的旁路 record**。
///
/// 与并入主 record 不同：目标 rule 的解析结果作为一条新 record 流出，由 engine 层
/// 按 `rule_name`（即目标 rule 的 wpl_key，如 `/fun/raw_event`）独立路由到对应 sink。
/// `target` 在构建期由 motor 层注入；未注入时（如 editor/station 解析路径）no-op。
/// 目标 rule 只解析，不执行其自身注解。
#[derive(Clone)]
pub struct CopyEventParseAnnotation {
    pub rule_name: SmolStr,
    pub target: Option<WplEvaluator>,
}

impl std::fmt::Debug for CopyEventParseAnnotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CopyEventParseAnnotation")
            .field("rule_name", &self.rule_name)
            .field("target_set", &self.target.is_some())
            .finish()
    }
}

impl AnnotationFunc for CopyEventParseAnnotation {
    fn proc(&self, src: &SourceEvent, _data: &mut DataRecord) -> Result<SideRecords, WparseError> {
        let Some(target) = &self.target else {
            return Ok(SideRecords::new());
        };
        // 复制 src.payload 喂给目标 rule 的 parser（proc_ref 避免每条事件 clone payload）
        let (target_rec, _left) = target.proc_ref(src.event_id, &src.payload, 0)?;
        // 产出独立旁路 record：按 rule_name（目标 rule 的 wpl_key）独立路由
        Ok(vec![(self.rule_name.clone(), target_rec)])
    }
}

#[derive(Clone, Debug)]
pub enum AnnotationType {
    Tag(TagAnnotation),
    Copy(RawCopy),
    Null(NoopAnnotation),
    CopyEventParse(CopyEventParseAnnotation),
}

impl AnnotationFunc for AnnotationType {
    fn proc(&self, src: &SourceEvent, data: &mut DataRecord) -> Result<SideRecords, WparseError> {
        match self {
            AnnotationType::Tag(func) => func.proc(src, data),
            AnnotationType::Null(func) => func.proc(src, data),
            AnnotationType::Copy(func) => func.proc(src, data),
            AnnotationType::CopyEventParse(func) => func.proc(src, data),
        }
    }
}

impl AnnotationType {
    pub fn convert(ann: &Option<AnnFun>) -> Vec<Self> {
        let mut vec = vec![];
        if let Some(ann) = ann {
            if !ann.tags.is_empty() {
                vec.push(AnnotationType::Tag(TagAnnotation {
                    args: ann.tags.clone(),
                }));
            }

            if let Some((k, v)) = &ann.copy_raw {
                if k == "name" {
                    vec.push(AnnotationType::Copy(RawCopy { raw_key: v.clone() }));
                } else {
                    vec.push(AnnotationType::Null(NoopAnnotation {}))
                }
            }

            if let Some(rule) = &ann.copy_event_parse {
                // target 留空，构建期由 motor 层按 rule_name 注入目标 rule 的 parser
                vec.push(AnnotationType::CopyEventParse(CopyEventParseAnnotation {
                    rule_name: rule.clone(),
                    target: None,
                }));
            }
        } else {
            vec.push(AnnotationType::Null(NoopAnnotation {}))
        }
        vec
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pkg::DEFAULT_KEY;
    use bytes::Bytes;
    use orion_error::dev::testing::TestAssert;
    use std::collections::BTreeMap;
    use std::sync::Arc;
    use wp_model_core::model::DataRecord;
    use wp_model_core::raw::RawData;
    use wp_source_types::{SourceEvent, Tags};

    #[test]
    fn test_tag_fun() {
        let ann = AnnFun {
            tags: BTreeMap::from([("tag_1".into(), "x".into())]),
            copy_raw: None,
            copy_event_parse: None,
            no_match: false,
        };
        let tag = AnnotationType::convert(&Some(ann));
        let mut data = DataRecord::test_value();
        let src = SourceEvent::new(
            1,
            DEFAULT_KEY.to_string(),
            RawData::String("test".to_string()),
            Tags::new().into(),
        );
        tag.first().unwrap().proc(&src, &mut data).assert();
        let expected = DataField::from_chars("tag_1", "x");
        assert_eq!(data.field("tag_1").map(|s| s.as_field()), Some(&expected));
    }

    #[test]
    fn test_copy_fun() {
        let ann = AnnFun {
            tags: Default::default(),
            copy_raw: Some(("name".into(), "raw".into())),
            copy_event_parse: None,
            no_match: false,
        };
        let tag = AnnotationType::convert(&Some(ann));
        let mut data = DataRecord::test_value();
        let src = SourceEvent::new(
            1,
            DEFAULT_KEY.to_string(),
            RawData::String("test".to_string()),
            Tags::new().into(),
        );
        tag.first().unwrap().proc(&src, &mut data).unwrap();
        let expected = DataField::from_chars("raw", "test");
        assert_eq!(data.field("raw").map(|s| s.as_field()), Some(&expected));
    }

    #[test]
    fn test_copy_fun_handles_invalid_utf8_bytes() {
        let tag = copy_raw_tag("raw");
        let raw = Bytes::from_static(b"hello \xff\xfe\xc0\xaf");
        let expected_raw = String::from_utf8_lossy(&raw).into_owned();
        let mut data = DataRecord::test_value();
        let src = SourceEvent::new(
            1,
            DEFAULT_KEY.to_string(),
            RawData::Bytes(raw),
            Tags::new().into(),
        );

        tag.first().unwrap().proc(&src, &mut data).unwrap();

        let expected = DataField::from_chars("raw", expected_raw);
        assert_eq!(data.field("raw").map(|s| s.as_field()), Some(&expected));
    }

    #[test]
    fn test_copy_fun_handles_invalid_utf8_arc_bytes() {
        let tag = copy_raw_tag("raw");
        let raw = Arc::new(b"hello \xff\xfe\xc0\xaf".to_vec());
        let expected_raw = String::from_utf8_lossy(raw.as_slice()).into_owned();
        let mut data = DataRecord::test_value();
        let src = SourceEvent::new(
            1,
            DEFAULT_KEY.to_string(),
            RawData::ArcBytes(raw),
            Tags::new().into(),
        );

        tag.first().unwrap().proc(&src, &mut data).unwrap();

        let expected = DataField::from_chars("raw", expected_raw);
        assert_eq!(data.field("raw").map(|s| s.as_field()), Some(&expected));
    }

    fn copy_raw_tag(raw_key: &str) -> Vec<AnnotationType> {
        AnnotationType::convert(&Some(AnnFun {
            tags: Default::default(),
            copy_raw: Some(("name".into(), raw_key.into())),
            copy_event_parse: None,
            no_match: false,
        }))
    }

    /// 构造一个注入了 target parser 的 copy_event_parse 注解，
    /// target rule 把 payload 整体捕获为 raw 字段。
    fn copy_event_parse_with_target(rule_code: &str, rule_name: &str) -> Vec<AnnotationType> {
        let target = WplEvaluator::from_code(rule_code).expect("build target evaluator");
        let mut funcs = AnnotationType::convert(&Some(AnnFun {
            tags: Default::default(),
            copy_raw: None,
            copy_event_parse: Some(rule_name.into()),
            no_match: false,
        }));
        for ann in &mut funcs {
            if let AnnotationType::CopyEventParse(c) = ann {
                c.target = Some(target.clone());
            }
        }
        funcs
    }

    #[test]
    fn test_copy_event_parse_emits_side_record() {
        // 目标 rule：解析 JSON {"raw":"..."} 产出名为 raw 的 chars 字段
        let funcs =
            copy_event_parse_with_target(r#"rule raw_event { (json(chars@raw)) }"#, "raw_event");
        let mut data = DataRecord::test_value();
        let src = SourceEvent::new(
            1,
            DEFAULT_KEY.to_string(),
            RawData::String(r#"{ "raw": "payload-content" }"#.to_string()),
            Tags::new().into(),
        );
        let sides = funcs.first().unwrap().proc(&src, &mut data).unwrap();
        // emit：目标解析结果作为独立旁路 record 返回，不并入主 record
        assert!(
            data.field("raw").is_none(),
            "target fields must NOT merge into main record"
        );
        assert_eq!(sides.len(), 1);
        let (key, rec) = &sides[0];
        assert_eq!(key.as_str(), "raw_event");
        let expected = DataField::from_chars("raw", "payload-content");
        assert_eq!(rec.field("raw").map(|s| s.as_field()), Some(&expected));
    }

    #[test]
    fn test_copy_event_parse_noop_without_target() {
        // 未注入 target（editor/station 路径）时应 no-op，不报错也不写字段
        let funcs = AnnotationType::convert(&Some(AnnFun {
            tags: Default::default(),
            copy_raw: None,
            copy_event_parse: Some("raw_event".into()),
            no_match: false,
        }));
        let mut data = DataRecord::test_value();
        let src = SourceEvent::new(
            1,
            DEFAULT_KEY.to_string(),
            RawData::String("payload-content".to_string()),
            Tags::new().into(),
        );
        funcs.first().unwrap().proc(&src, &mut data).unwrap();
        // 未注入 target 时 raw 字段不应存在
        assert!(data.field("raw").is_none());
    }
}
