use std::collections::BTreeMap;
use std::io::Write;

use smol_str::SmolStr;
use winnow::stream::Accumulate;

use crate::ast::WplTag;
use crate::ast::debug::{DebugFormat, DepIndent};
use crate::parser::MergeTags;

pub type TagKvs = BTreeMap<SmolStr, SmolStr>;
pub type CopyRaw = (SmolStr, SmolStr);

#[derive(Debug, PartialEq, Clone)]
pub enum AnnEnum {
    Tags(TagKvs),
    Copy(CopyRaw),
    CopyEventParse(SmolStr),
    NoMatch,
    Late,
}
#[derive(Debug, PartialEq, Default, Clone)]
pub struct AnnFun {
    pub tags: TagKvs,
    pub copy_raw: Option<CopyRaw>,
    pub copy_event_parse: Option<SmolStr>,
    /// 显式声明：该 rule 不参与 parse_event 自动匹配（仅作 copy_event_parse 等显式调用的目标）。
    pub no_match: bool,
    /// 显式声明：该 rule 参与自动匹配，但固定排在普通 rule 之后。
    /// 命中率排序只在层内生效，不会把 late rule 提升到普通 rule 之前。
    pub late: bool,
}

impl MergeTags for AnnFun {
    fn merge_tags(&mut self, other_tags: &Option<AnnFun>) {
        if let Some(atags) = other_tags {
            for (other_k, other_v) in &atags.tags {
                if self.tags.get_mut(other_k).is_none() {
                    self.tags.insert(other_k.clone(), other_v.clone());
                }
            }

            if self.copy_raw.is_none() {
                self.copy_raw = atags.copy_raw.clone()
            }
            if self.copy_event_parse.is_none() {
                self.copy_event_parse = atags.copy_event_parse.clone()
            }
            // no_match / late 取并集：任一层声明即生效
            self.no_match = self.no_match || atags.no_match;
            self.late = self.late || atags.late;
        }
    }
}

impl AnnFun {
    pub fn export_tags(&self) -> Vec<WplTag> {
        let mut tags = Vec::new();
        for (k, v) in &self.tags {
            tags.push(WplTag::new(k.clone(), v.clone()))
        }
        tags
    }
}

impl DebugFormat for AnnFun {
    fn write<W>(&self, w: &mut W) -> std::io::Result<()>
    where
        W: ?Sized + Write + DepIndent,
    {
        write!(w, "#[")?;
        let mut need_sep = false;

        if !self.tags.is_empty() {
            write_sep(w, &mut need_sep)?;
            write!(w, "tag")?;
            self.write_open_parenthesis(w)?;
            for (index, (k, v)) in self.tags.iter().enumerate() {
                if index != 0 {
                    write!(w, ", ")?;
                }
                write!(w, "{}:\"{}\"", k, v)?;
            }
            self.write_close_parenthesis(w)?;
        }

        if let Some((ck, cv)) = &self.copy_raw {
            write_sep(w, &mut need_sep)?;
            write!(w, "copy_raw")?;
            self.write_open_parenthesis(w)?;
            write!(w, "{}:\"{}\"", ck, cv)?;
            self.write_close_parenthesis(w)?;
        }

        if let Some(rule) = &self.copy_event_parse {
            write_sep(w, &mut need_sep)?;
            write!(w, "copy_event_parse")?;
            self.write_open_parenthesis(w)?;
            write!(w, "rule:\"{}\"", rule)?;
            self.write_close_parenthesis(w)?;
        }

        if self.no_match {
            write_sep(w, &mut need_sep)?;
            write!(w, "no_match")?;
        }

        if self.late {
            write_sep(w, &mut need_sep)?;
            write!(w, "late")?;
        }

        write!(w, "]")?;
        self.write_new_line(w)?;
        Ok(())
    }
}

fn write_sep<W>(w: &mut W, need_sep: &mut bool) -> std::io::Result<()>
where
    W: ?Sized + Write,
{
    if *need_sep {
        write!(w, ", ")?;
    }
    *need_sep = true;
    Ok(())
}

impl Accumulate<()> for AnnFun {
    fn initial(_: Option<usize>) -> Self {
        AnnFun::default()
    }

    fn accumulate(&mut self, _acc: ()) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn late_and_no_match_merge_as_union() {
        // rule 层未声明，package 层声明 → 继承
        let mut own = AnnFun::default();
        own.merge_tags(&Some(AnnFun {
            late: true,
            no_match: true,
            ..Default::default()
        }));
        assert!(own.late);
        assert!(own.no_match);

        // 任一已声明即保留
        let mut own = AnnFun {
            late: true,
            ..Default::default()
        };
        own.merge_tags(&Some(AnnFun::default()));
        assert!(
            own.late,
            "own true must survive an empty package annotation"
        );
    }

    #[test]
    fn debug_format_renders_bare_flag_without_tag_prefix() {
        let ann = AnnFun {
            late: true,
            ..Default::default()
        };
        assert_eq!(ann.fmt_string().unwrap().trim(), "#[late]");
    }

    #[test]
    fn debug_format_keeps_all_items_with_single_tag() {
        // 回归：旧实现遇到单个 tag 会提前 return，丢掉后续注解
        let ann = AnnFun {
            tags: BTreeMap::from([("k".into(), "v".into())]),
            copy_raw: Some(("name".into(), "raw".into())),
            copy_event_parse: Some("pkg/rule".into()),
            no_match: true,
            late: true,
        };
        assert_eq!(
            ann.fmt_string().unwrap().trim(),
            r#"#[tag(k:"v"), copy_raw(name:"raw"), copy_event_parse(rule:"pkg/rule"), no_match, late]"#
        );
    }

    #[test]
    fn debug_format_renders_multiple_tags() {
        let ann = AnnFun {
            tags: BTreeMap::from([("a".into(), "1".into()), ("b".into(), "2".into())]),
            late: true,
            ..Default::default()
        };
        assert_eq!(
            ann.fmt_string().unwrap().trim(),
            r#"#[tag(a:"1", b:"2"), late]"#
        );
    }
}
