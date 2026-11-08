use super::super::prelude::*;
use crate::ast::WplSep;
use crate::eval::runtime::field::FieldEvalUnit;
use quick_xml::Reader;
use quick_xml::escape::resolve_predefined_entity;
use quick_xml::events::{BytesRef, Event};
use std::collections::HashMap;
use wp_model_core::model::FNameStr;

/// WPL `xml(...)` 结构化解析器。
///
/// 与 `json(...)` 对齐：按声明取子字段，未声明的元素自动按 `chars` 输出。
/// 元素通过**路径**匹配（文档根元素透明），因此不依赖元素在文档中的先后顺序，
/// 且缺失元素会被静默跳过。
#[derive(Default)]
pub struct XmlP {}

impl PatternParser for XmlP {
    fn pattern_parse(
        &self,
        e_id: u64,
        fpu: &FieldEvalUnit,
        ups_sep: &WplSep,
        data: &mut &str,
        _name: FNameStr,
        out: &mut Vec<DataField>,
    ) -> ModalResult<()> {
        multispace0.parse_next(data)?;
        let input = *data;
        let Some(doc) = XmlDocument::parse(input) else {
            return fail.context(ctx_desc("xml document")).parse_next(data);
        };
        XmlProc::emit(e_id, fpu, ups_sep, &doc.root, out)?;
        *data = &input[doc.consumed.min(input.len())..];
        Ok(())
    }

    fn patten_gen(
        &self,
        _gen: &mut GenChannel,
        f_conf: &WplField,
        _g_conf: Option<&FieldGenConf>,
    ) -> WplCodeResult<DataField> {
        Ok(DataField::from_chars(
            f_conf.safe_name(),
            "<root></root>".to_string(),
        ))
    }
}

/// 解析后的 XML 元素节点。仅保留元素名、直接文本与子元素；
/// 属性与混合内容不在当前版本支持范围内。
#[derive(Debug, Default, PartialEq)]
struct XmlNode {
    name: String,
    text: String,
    children: Vec<XmlNode>,
}

impl XmlNode {
    fn new(name: String) -> Self {
        Self {
            name,
            text: String::new(),
            children: Vec::new(),
        }
    }
}

#[derive(Debug)]
struct XmlDocument {
    root: XmlNode,
    /// 已消费的字节数（根元素闭合标签之后的位置）
    consumed: usize,
}

impl XmlDocument {
    /// 嵌套深度上限，避免恶意/异常输入导致递归溢出
    const MAX_DEPTH: usize = 256;

    /// 从输入起始位置解析**一个**根元素。
    ///
    /// 返回 `None` 表示不是合法 XML（此时不消费输入，交给上层回退）。
    fn parse(input: &str) -> Option<Self> {
        // 容忍文件 BOM（日志抽取场景常见）
        let (bom_len, body) = match input.strip_prefix('\u{feff}') {
            Some(rest) => (input.len() - rest.len(), rest),
            None => (0, input),
        };
        let mut reader = Reader::from_str(body);
        // 日志类 XML 常见裸 `&`，这里放宽但不放宽标签闭合校验
        reader.config_mut().allow_dangling_amp = true;
        reader.config_mut().check_end_names = true;

        let mut stack: Vec<XmlNode> = Vec::new();
        let closed: (XmlNode, usize) = loop {
            let event = reader.read_event().ok()?;
            match event {
                Event::Start(e) => {
                    if stack.len() >= Self::MAX_DEPTH {
                        return None;
                    }
                    stack.push(XmlNode::new(decode_name(e.name().as_ref())));
                }
                Event::Empty(e) => {
                    let node = XmlNode::new(decode_name(e.name().as_ref()));
                    match stack.last_mut() {
                        Some(parent) => parent.children.push(node),
                        None => break (node, reader.buffer_position() as usize),
                    }
                }
                Event::End(_) => {
                    let node = stack.pop()?;
                    match stack.last_mut() {
                        Some(parent) => parent.children.push(node),
                        None => break (node, reader.buffer_position() as usize),
                    }
                }
                Event::Text(e) => push_text(&mut stack, e.decode().ok()?.as_ref()),
                Event::CData(e) => push_text(&mut stack, e.decode().ok()?.as_ref()),
                Event::GeneralRef(e) => push_text(&mut stack, resolve_ref(&e)?.as_str()),
                Event::Eof => return None,
                // 声明 / 注释 / 处理指令 / DOCTYPE 不产生数据
                _ => {}
            }
        };

        Some(Self {
            root: closed.0,
            consumed: closed.1 + bom_len,
        })
    }
}

struct XmlProc;

impl XmlProc {
    fn emit(
        e_id: u64,
        fpu: &FieldEvalUnit,
        ups_sep: &WplSep,
        root: &XmlNode,
        out: &mut Vec<DataField>,
    ) -> ModalResult<()> {
        // 文档根元素透明：`<root><url>..</url></root>` 的路径是 `url`；
        // 单元素文档（如 `<msg>hi</msg>`）按根元素名输出。
        // 空文档（`<root/>`）既不显式配置也无文本时，不产出字段。
        if root.children.is_empty() {
            let configured = fpu
                .conf()
                .sub_fields()
                .as_ref()
                .is_some_and(|subs| subs.get(root.name.as_str()).is_some());
            if !configured && root.text.trim().is_empty() {
                return Ok(());
            }
            return Self::emit_node(e_id, fpu, ups_sep, &root.name, root, out);
        }
        Self::emit_children(e_id, fpu, ups_sep, "", &root.children, out)
    }

    fn emit_children(
        e_id: u64,
        fpu: &FieldEvalUnit,
        ups_sep: &WplSep,
        parent: &str,
        children: &[XmlNode],
        out: &mut Vec<DataField>,
    ) -> ModalResult<()> {
        // 先统计同名兄弟元素个数，再按出现顺序编号；
        // 重复元素沿用数组命名风格：首个保留原名，后续追加 `/[i]`。
        let mut totals: HashMap<&str, usize> = HashMap::new();
        for child in children {
            *totals.entry(child.name.as_str()).or_insert(0) += 1;
        }
        let mut seen: HashMap<&str, usize> = HashMap::new();
        for node in children {
            let name = node.name.as_str();
            let pos = seen.entry(name).or_insert(0);
            let index = *pos;
            *pos += 1;

            let path = join_path(parent, name);
            let path = if totals.get(name).copied().unwrap_or(1) > 1 && index > 0 {
                format!("{}/[{}]", path, index)
            } else {
                path
            };
            Self::emit_node(e_id, fpu, ups_sep, path.as_str(), node, out)?;
        }
        Ok(())
    }

    fn emit_node(
        e_id: u64,
        fpu: &FieldEvalUnit,
        ups_sep: &WplSep,
        path: &str,
        node: &XmlNode,
        out: &mut Vec<DataField>,
    ) -> ModalResult<()> {
        let text = trim_text(node.text.as_str());
        let sub_conf = fpu
            .conf()
            .sub_fields()
            .as_ref()
            .and_then(|subs| subs.get(path));

        if let (Some(conf), Some(sub_fpu)) = (sub_conf, fpu.get_sub_fpu(path)) {
            // 空元素对非 chars 类型视为缺失（XML 里空元素很常见），避免一次空值拖幂整条记录
            if text.is_empty() && !matches!(conf.meta_type(), DataType::Chars | DataType::Ignore) {
                return Ok(());
            }
            let run_key = conf.run_key_str(path);
            let mut sep = sub_fpu.conf().resolve_sep(ups_sep);
            // 文本已经独立成段，避免 chars 在空格处被截断
            if sep.is_space_sep() {
                sep.set_current("\\0");
            }
            let mut raw = text;
            return sub_fpu.parse(e_id, &sep, &mut raw, Some(run_key), out);
        }

        if node.children.is_empty() {
            out.push(DataField::from_chars(path.to_string(), text.to_string()));
            return Ok(());
        }
        Self::emit_children(e_id, fpu, ups_sep, path, &node.children, out)
    }
}

#[inline]
fn join_path(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_string()
    } else {
        format!("{parent}/{name}")
    }
}

#[inline]
fn trim_text(text: &str) -> &str {
    text.trim()
}

fn push_text(stack: &mut [XmlNode], text: &str) {
    if let Some(current) = stack.last_mut() {
        current.text.push_str(text);
    }
}

/// 元素名解码，并去掉命名空间前缀（`ns:url` → `url`）。
fn decode_name(raw: &[u8]) -> String {
    let name = String::from_utf8_lossy(raw);
    match name.rsplit_once(':') {
        Some((_, local)) if !local.is_empty() => local.to_string(),
        _ => name.into_owned(),
    }
}

/// 解析文本中的实体引用；未知实体保留原始形态。
fn resolve_ref(reference: &BytesRef<'_>) -> Option<String> {
    if let Some(ch) = reference.resolve_char_ref().ok().flatten() {
        return Some(ch.to_string());
    }
    let name = reference.decode().ok()?;
    Some(match resolve_predefined_entity(name.as_ref()) {
        Some(value) => value.to_string(),
        None => format!("&{};", name),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{WplPackage, WplStatementType};
    use crate::eval::runtime::vm_unit::WplEvaluator;
    use orion_error::dev::testing::TestAssert;
    use wp_model_core::model::Value;

    fn chars_of<'a>(record: &'a wp_model_core::model::DataRecord, key: &str) -> Option<&'a str> {
        match record.field(key).map(|f| f.get_value()) {
            Some(Value::Chars(s)) => Some(s.as_str()),
            _ => None,
        }
    }

    fn eval(expr: &str, data: &str) -> wp_model_core::model::DataRecord {
        let rule = format!("rule xml_test {{ {expr} }}");
        let pipe = WplEvaluator::from_code(rule.as_str()).assert();
        let (record, residue) = pipe.proc(0, data, 0).assert();
        assert_eq!(residue, "", "unexpected residue: {residue}");
        record
    }

    #[test]
    fn parse_flat_document_with_entities() {
        let input = "<root>\n <url>a/b</url>\n <desc>&lt;req&gt;&amp;x</desc>\n <empty/>\n</root>";
        let doc = XmlDocument::parse(input).expect("xml should parse");

        assert_eq!(doc.root.name, "root");
        assert_eq!(doc.root.children.len(), 3);
        assert_eq!(doc.root.children[0].name, "url");
        assert_eq!(doc.root.children[0].text, "a/b");
        assert_eq!(doc.root.children[1].text, "<req>&x");
        assert_eq!(doc.root.children[2].name, "empty");
        assert!(doc.root.children[2].text.is_empty());
        assert_eq!(doc.consumed, input.len());
    }

    #[test]
    fn parse_resolves_entities_and_keeps_unknown_ones() {
        let input = concat!(
            "<root>",
            "<lt>&lt;</lt>",
            "<num>&#x4e2d;&#25991;</num>",
            "<unk>&foo;</unk>",
            "<amp>a & b</amp>",
            "</root>",
        );
        let doc = XmlDocument::parse(input).expect("xml should parse");

        assert_eq!(doc.root.children[0].text, "<");
        assert_eq!(doc.root.children[1].text, "中文");
        assert_eq!(doc.root.children[2].text, "&foo;");
        assert_eq!(doc.root.children[3].text, "a & b");
    }

    #[test]
    fn parse_stops_after_root_element() {
        let input = "<root><a>1</a></root> tail";
        let doc = XmlDocument::parse(input).expect("xml should parse");
        assert_eq!(&input[doc.consumed..], " tail");
    }

    #[test]
    fn parse_rejects_non_xml() {
        assert!(XmlDocument::parse("plain text").is_none());
        assert!(XmlDocument::parse("<root><a>1</root>").is_none());
    }

    #[test]
    fn parse_strips_namespace_prefix() {
        let doc = XmlDocument::parse("<ns:root><ns:url>x</ns:url></ns:root>").expect("xml");
        assert_eq!(doc.root.name, "root");
        assert_eq!(doc.root.children[0].name, "url");
    }

    const SAMPLE_RESULT: &str = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<root>\n  <url>172.16.8.248/****/aa.php?anp=************</url>\n  <ts>1747900223</ts>\n  <vxlan_vni_1>0</vxlan_vni_1>\n  <in_dev>replay0</in_dev>\n  <rule_ver>2025.04.10</rule_ver>\n  <devid>2C58F5D0</devid>\n  <src_mac>a0:d3:c1:f1:4a:7d</src_mac>\n  <depict>&lt;sub_desc&gt;ActualScripts Cookie&lt;/sub_desc&gt;</depict>\n  <uri>/****/aa.php?anp=************</uri>\n  <user_agent>Mozilla/4.0 (compatible; MSIE 6.0; Windows NT 5.1)</user_agent>\n  <matched_sid>17170080,11023137</matched_sid>\n  <status_code>302</status_code>\n  <result_id>0</result_id>\n</root>\n";

    #[test]
    fn test_xml_typed_subfields_are_order_independent() {
        let rule = r#"(xml(
            chars@url:res_url,
            time_timestamp@ts,
            digit@status_code,
            chars@user_agent,
            chars@matched_sid,
            chars@depict
        ))"#;
        let record = eval(rule, SAMPLE_RESULT);

        assert_eq!(
            chars_of(&record, "res_url"),
            Some("172.16.8.248/****/aa.php?anp=************")
        );
        assert_eq!(
            chars_of(&record, "user_agent"),
            Some("Mozilla/4.0 (compatible; MSIE 6.0; Windows NT 5.1)")
        );
        assert_eq!(chars_of(&record, "matched_sid"), Some("17170080,11023137"));
        assert_eq!(
            chars_of(&record, "depict"),
            Some("<sub_desc>ActualScripts Cookie</sub_desc>")
        );
        assert!(matches!(
            record.field("ts").map(|f| f.get_value()),
            Some(Value::Time(_))
        ));
        assert!(matches!(
            record.field("status_code").map(|f| f.get_value()),
            Some(Value::Int(302))
        ));
    }

    #[test]
    fn test_xml_missing_element_is_skipped() {
        let rule = r#"(xml(chars@url, chars@not_exists, chars@status_code))"#;
        let record = eval(rule, SAMPLE_RESULT);

        assert_eq!(
            chars_of(&record, "url"),
            Some("172.16.8.248/****/aa.php?anp=************")
        );
        assert_eq!(chars_of(&record, "status_code"), Some("302"));
        assert!(record.field("not_exists").is_none());
    }

    #[test]
    fn test_xml_without_subfields_emits_all_leaves() {
        let record = eval("(xml)", SAMPLE_RESULT);

        assert_eq!(chars_of(&record, "devid"), Some("2C58F5D0"));
        assert_eq!(chars_of(&record, "result_id"), Some("0"));
        assert!(record.field("root").is_none());
    }

    #[test]
    fn xml_field_round_trips_through_wpl_source() {
        let conf =
            WplField::try_parse("xml(chars@url, digit@status_code)").expect("parse xml field");
        assert_eq!(conf.meta_name().as_str(), "xml");
        assert_eq!(*conf.meta_type(), DataType::Chars);
        assert!(
            conf.sub_fields()
                .as_ref()
                .is_some_and(|s| s.get("url").is_some())
        );
        assert!(conf.to_string().starts_with("xml("));
    }

    #[test]
    fn test_xml_single_element_document() {
        let record = eval("(xml(chars@msg))", "<msg>hello</msg>");
        assert_eq!(chars_of(&record, "msg"), Some("hello"));
    }

    #[test]
    fn test_xml_ignore_subfield_and_repeated_elements() {
        let rule = r#"(xml(_@noise, chars@item, chars@keep))"#;
        let data = "<root><noise>a</noise><item>1</item><item>2</item><keep>k</keep></root>";
        let record = eval(rule, data);

        // 与 `json(_@key)` 一致：忽略字段会以 Ignore 值保留，输出层不展示
        assert!(matches!(
            record.field("noise").map(|f| f.get_value()),
            Some(Value::Ignore(_))
        ));
        assert_eq!(chars_of(&record, "item"), Some("1"));
        assert_eq!(chars_of(&record, "item/[1]"), Some("2"));
        assert_eq!(chars_of(&record, "keep"), Some("k"));
    }

    #[test]
    fn test_xml_repeated_siblings_are_indexed_even_when_interleaved() {
        let rule = r#"(xml(chars@a, chars@b))"#;
        let record = eval(rule, "<root><a>1</a><b>2</b><a>3</a></root>");

        assert_eq!(chars_of(&record, "a"), Some("1"));
        assert_eq!(chars_of(&record, "a/[1]"), Some("3"));
        assert_eq!(chars_of(&record, "b"), Some("2"));
    }

    #[test]
    fn test_xml_skips_empty_values_for_typed_subfields() {
        let rule = r#"(xml(digit@a, chars@b, ip@c))"#;
        let record = eval(rule, "<root><a></a><b>abc</b><c/></root>");

        assert!(record.field("a").is_none());
        assert!(record.field("c").is_none());
        assert_eq!(chars_of(&record, "b"), Some("abc"));
    }

    #[test]
    fn test_xml_typed_value_mismatch_fails_rule() {
        // 与 `json(digit@k)` 一致：非空但不符类型会直接判定解析失败
        let pipe = WplEvaluator::from_code("rule xml_test { (xml(digit@port)) }").assert();
        assert!(pipe.proc(0, "<root><port>abc</port></root>", 0).is_err());
    }

    #[test]
    fn test_xml_empty_document_emits_nothing() {
        let record = eval("(xml)", "<root/>");
        assert!(record.field("root").is_none());
        assert_eq!(record.items.len(), 0);
    }

    #[test]
    fn test_xml_handles_decl_doctype_comment_pi_and_cdata() {
        let rule = r#"(xml(chars@a, chars@b))"#;
        let data = concat!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>",
            "<!DOCTYPE root>",
            "<!-- top comment -->",
            "<root>",
            "<a>1<!-- inner -->2</a>",
            "<b><![CDATA[<raw>&x]]></b>",
            "<?pi data?>",
            "</root>",
        );
        let record = eval(rule, data);

        assert_eq!(chars_of(&record, "a"), Some("12"));
        assert_eq!(chars_of(&record, "b"), Some("<raw>&x"));
    }

    #[test]
    fn test_xml_ignores_attributes() {
        let rule = r#"(xml(chars@item))"#;
        let record = eval(rule, r#"<root><item id="7" name="n">v</item></root>"#);
        assert_eq!(chars_of(&record, "item"), Some("v"));
    }

    #[test]
    fn test_xml_trims_surrounding_whitespace() {
        let rule = r#"(xml(chars@a))"#;
        let record = eval(rule, "<root><a>  padded value  </a></root>");
        assert_eq!(chars_of(&record, "a"), Some("padded value"));
    }

    #[test]
    fn test_xml_strips_bom() {
        let rule = r#"(xml(chars@a))"#;
        let record = eval(rule, "\u{feff}<root><a>1</a></root>");
        assert_eq!(chars_of(&record, "a"), Some("1"));
    }

    #[test]
    fn test_xml_subfield_pipes_still_apply() {
        let rule = r#"(xml(chars@ua | chars_replace("Mozilla", "M")))"#;
        let record = eval(rule, "<root><ua>Mozilla/4.0</ua></root>");
        assert_eq!(chars_of(&record, "ua"), Some("M/4.0"));
    }

    #[test]
    fn test_xml_consumes_only_first_root_element() {
        let rule = r#"(xml(chars@a), chars:tail)"#;
        let record = eval(rule, "<a>1</a><b>2</b>");

        assert_eq!(chars_of(&record, "a"), Some("1"));
        assert_eq!(chars_of(&record, "tail"), Some("<b>2</b>"));
    }

    #[test]
    fn test_xml_fragment_exposes_only_first_root() {
        // XML 片段（多个并列根元素）不是合法文档：只解析首个根元素，其余留给后续字段
        let pipe = WplEvaluator::from_code("rule xml_test { (xml(chars@a, chars@b)) }").assert();
        let (record, residue) = pipe.proc(0, "<a>1</a><b>2</b>", 0).assert();

        assert_eq!(chars_of(&record, "a"), Some("1"));
        assert!(record.field("b").is_none());
        assert_eq!(residue, "<b>2</b>");
    }

    #[test]
    fn test_xml_rejects_excessive_nesting_without_panicking() {
        let deep = format!("{}{}", "<a>".repeat(300), "</a>".repeat(300));
        let pipe = WplEvaluator::from_code("rule xml_test { (xml(chars@a)) }").assert();
        assert!(pipe.proc(0, deep.as_str(), 0).is_err());
    }

    #[test]
    fn test_xml_wildcard_ignore_drops_all_elements() {
        let rule = r#"(xml(_@*))"#;
        let record = eval(rule, "<root><a>1</a><b>2</b></root>");

        let visible = record
            .items
            .iter()
            .filter(|item| !matches!(item.as_field().get_value(), Value::Ignore(_)))
            .count();
        assert_eq!(visible, 0);
    }

    #[test]
    fn xml_generate_returns_placeholder_document() {
        let conf = WplField::try_parse("xml").expect("parse xml field");
        let mut gch = GenChannel::new();
        let field = XmlP::default()
            .patten_gen(&mut gch, &conf, None)
            .expect("generate xml field");
        assert_eq!(field.get_value(), &Value::Chars("<root></root>".into()));
    }

    #[test]
    fn test_xml_rule_parses_from_package_source() {
        let mut src = r#"
#[copy_raw(name:"raw_msg")]
package sangfor_probe {
  #[tag(log_desc:"深信服探针-IPS 告警(result XML)", log_type:"ips")]
  rule ips {
    (
      json(
        chars@logtype,
        ip@src_ip,
        chars@result | json_unescape() | (
          xml(chars@url:res_url, digit@status_code:res_status_code)
        )
      ) | f_chars_has(logtype, ips)
    )
  }
}
"#;
        let package = WplPackage::parse(&mut src, "issue-373").expect("package should parse");
        let rule = package.rules.front().expect("rule should exist");
        let WplStatementType::Express(express) = &rule.statement;
        let pipe = WplEvaluator::from(express, None).expect("evaluator should build");

        let data = r#"{"logtype":"ips","src_ip":"1.1.1.1","result":"<?xml version=\"1.0\"?>\n<root><url>/a</url><status_code>200</status_code></root>"}"#;
        let (record, residue) = pipe.proc(0, data, 0).expect("sample should parse");
        assert_eq!(residue, "");
        assert_eq!(chars_of(&record, "res_url"), Some("/a"));
        assert!(matches!(
            record.field("res_status_code").map(|f| f.get_value()),
            Some(Value::Int(200))
        ));
    }

    #[test]
    fn test_xml_nested_path() {
        let rule = r#"(xml(ip@packet/sip, digit@packet/port))"#;
        let data = "<root><packet><sip>10.0.0.1</sip><port>80</port></packet></root>";
        let record = eval(rule, data);

        assert!(matches!(
            record.field("packet/sip").map(|f| f.get_value()),
            Some(Value::IpAddr(_))
        ));
        assert!(matches!(
            record.field("packet/port").map(|f| f.get_value()),
            Some(Value::Int(80))
        ));
    }

    #[test]
    fn test_xml_rejects_non_xml_input() {
        let pipe = WplEvaluator::from_code("rule xml_test { (xml(chars@url)) }").assert();
        assert!(pipe.proc(0, "plain text with spaces", 0).is_err());
    }

    #[test]
    fn test_xml_parses_json_result_field() {
        let sample = r#"{"ts":1747900223,"logtype":"ips","src_ip":"172.16.1.2","dst_ip":"172.16.8.248","result":"<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<root>\n  <url>172.16.8.248/****/aa.php</url>\n  <user_agent>Mozilla/4.0</user_agent>\n  <status_code>302</status_code>\n</root>\n"}"#;
        let rule = r#"(
            json(
                time_timestamp@ts,
                chars@logtype,
                ip@src_ip,
                ip@dst_ip,
                chars@result | (
                    xml(
                        chars@url:res_url,
                        chars@user_agent:res_user_agent,
                        digit@status_code:res_status_code,
                    )
                )
            ) | f_chars_has(logtype, ips)
        )"#;
        let record = eval(rule, sample);

        assert_eq!(
            chars_of(&record, "res_url"),
            Some("172.16.8.248/****/aa.php")
        );
        assert_eq!(chars_of(&record, "res_user_agent"), Some("Mozilla/4.0"));
        assert!(matches!(
            record.field("res_status_code").map(|f| f.get_value()),
            Some(Value::Int(302))
        ));
        assert!(record.field("result").is_none());
    }

    /// issue #373：深信服探针把关键信息放在 JSON 的 `result` 字段（XML）中。
    const SANGFOR_PROBE_IPS: &str =
        include_str!("../../../../../tests/data/sangfor_probe_ips.json");

    #[test]
    fn test_xml_sangfor_probe_ips_result() {
        let rule = r#"(
            json(
                time_timestamp@ts,
                chars@logtype,
                chars@devname,
                ip@src_ip,
                ip@dst_ip,
                port@src_port,
                port@dst_port,
                digit@level,
                chars@result | json_unescape() | (
                    xml(
                        chars@url:res_url,
                        time_timestamp@ts:res_ts,
                        chars@rule_ver:res_rule_ver,
                        chars@devid:res_devid,
                        chars@sta_ver:res_sta_ver,
                        chars@connid:res_connid,
                        chars@dst_mac:res_dst_mac,
                        chars@src_mac:res_src_mac,
                        chars@encode_type:res_encode_type,
                        chars@uri:res_uri,
                        chars@user_agent:res_user_agent,
                        chars@matched_sid:res_matched_sid,
                        chars@sip_type:res_sip_type,
                        chars@status_code:res_status_code,
                        chars@attack_state:res_attack_state,
                        chars@result_id:res_result_id,
                    )
                )
            ) | f_chars_has(logtype, ips)
        )"#;
        let record = eval(rule, SANGFOR_PROBE_IPS.trim_end());

        assert_eq!(
            chars_of(&record, "res_url"),
            Some("172.16.8.248/****/aa.php?anp=************")
        );
        assert_eq!(chars_of(&record, "res_rule_ver"), Some("2025.04.10"));
        assert_eq!(chars_of(&record, "res_devid"), Some("2C58F5D0"));
        assert_eq!(chars_of(&record, "res_connid"), Some("769137190"));
        assert_eq!(chars_of(&record, "res_src_mac"), Some("a0:d3:c1:f1:4a:7d"));
        assert_eq!(chars_of(&record, "res_encode_type"), Some("base64"));
        assert_eq!(
            chars_of(&record, "res_user_agent"),
            Some("Mozilla/4.0 (compatible; MSIE 6.0; Windows NT 5.1)")
        );
        assert_eq!(
            chars_of(&record, "res_matched_sid"),
            Some("17170080,11023137")
        );
        assert_eq!(chars_of(&record, "res_sip_type"), Some("30|1|0"));
        assert_eq!(chars_of(&record, "res_status_code"), Some("302"));
        assert_eq!(chars_of(&record, "res_result_id"), Some("0"));
        assert!(matches!(
            record.field("res_ts").map(|f| f.get_value()),
            Some(Value::Time(_))
        ));
    }
}
