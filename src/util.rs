use glob::glob;
use std::{fs::File, io::Read, path::PathBuf};

use orion_error::ErrorWith;
use orion_error::compat_traits::ErrorOweBase;
use orion_error::runtime::ContextRecord;
use orion_error::{OperationContext, UvsFrom};
use wp_log::info_ctrl;

use crate::WplCode;
use crate::parser::error::WplCodeResult;
use crate::parser::error::{WplCodeError, WplCodeReason};

pub fn fetch_wpl_data(path: &str, target: &str) -> WplCodeResult<Vec<WplCode>> {
    let mut wpl_vec = Vec::new();
    let mut ctx = OperationContext::doing("load wpl");
    ctx.record("path", path);
    let files = find_conf_files(path, target).with_context(&ctx)?;

    for f_name in &files {
        info_ctrl!("load conf file: {:?}", f_name);
        let mut f = File::open(f_name)
            .owe(WplCodeReason::from_conf())
            .with_context(&ctx)?;
        let mut buffer = Vec::with_capacity(10240);
        f.read_to_end(&mut buffer)
            .owe(WplCodeReason::from_conf())
            .with_context(&ctx)?;
        let file_data = String::from_utf8(buffer)
            .owe(WplCodeReason::from_conf())
            .with_context(&ctx)?;
        wpl_vec.push(WplCode::try_from((PathBuf::from(f_name), file_data))?)
    }
    Ok(wpl_vec)
}
fn find_conf_files(path: &str, target: &str) -> WplCodeResult<Vec<PathBuf>> {
    let mut found = Vec::new();
    info_ctrl!("find conf files in: {}", path);
    let glob_path = format!("{}/**/{}", path, target);
    for entry in glob(glob_path.as_str()).map_err(|e| {
        WplCodeError::from(WplCodeReason::Syntax(format!(
            "read_dir fail: {}, {}",
            path, e
        )))
    })? {
        match entry {
            Ok(path) => {
                found.push(path);
            }
            Err(e) => {
                error!("find_conf files fail: {}", e);
            }
        }
    }
    Ok(found)
}
