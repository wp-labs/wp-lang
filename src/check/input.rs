use std::io::{self, Read};
use std::path::{Path, PathBuf};

use super::{DEFAULT_RULE_FILE, DEFAULT_SAMPLE_FILE, SampleInput};

pub(crate) fn resolve_source_path(path: Option<&Path>) -> Option<PathBuf> {
    match path {
        Some(path) if path == Path::new("-") => Some(path.to_path_buf()),
        Some(path) if path.is_dir() => Some(path.join(DEFAULT_RULE_FILE)),
        Some(path) => Some(path.to_path_buf()),
        None => None,
    }
}

pub(crate) fn resolve_sample_input(
    sample: &SampleInput,
    source_path: Option<&Path>,
) -> SampleInput {
    match sample {
        SampleInput::Inline(data) => SampleInput::Inline(data.clone()),
        SampleInput::DefaultFile => {
            let path = if let Some(source_path) = source_path {
                if source_path != Path::new("-") {
                    source_path
                        .parent()
                        .map(|base| base.join(DEFAULT_SAMPLE_FILE))
                        .unwrap_or_else(|| PathBuf::from(DEFAULT_SAMPLE_FILE))
                } else {
                    PathBuf::from(DEFAULT_SAMPLE_FILE)
                }
            } else {
                PathBuf::from(DEFAULT_SAMPLE_FILE)
            };
            SampleInput::File(path)
        }
        SampleInput::File(path) if path.is_absolute() => SampleInput::File(path.clone()),
        SampleInput::File(path) => {
            if path.is_dir() {
                SampleInput::File(path.join(DEFAULT_SAMPLE_FILE))
            } else {
                SampleInput::File(path.clone())
            }
        }
    }
}

pub(crate) fn load_input(path: Option<&Path>) -> Result<(String, Option<String>), String> {
    match path {
        Some(path) if path != Path::new("-") => {
            let source = std::fs::read_to_string(path)
                .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
            Ok((source, Some(path.display().to_string())))
        }
        _ => {
            let mut source = String::new();
            io::stdin()
                .read_to_string(&mut source)
                .map_err(|err| format!("failed to read stdin: {err}"))?;
            Ok((source, None))
        }
    }
}

pub(crate) fn load_sample_data(sample: &SampleInput) -> Result<String, String> {
    match sample {
        SampleInput::Inline(data) => Ok(data.clone()),
        SampleInput::DefaultFile => {
            Err("internal error: default sample path was not resolved".to_string())
        }
        SampleInput::File(path) => std::fs::read_to_string(path)
            .map_err(|err| format!("failed to read sample data {}: {err}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    #[test]
    fn test_resolve_directory_defaults() {
        let source = resolve_source_path(Some(Path::new("examples/wpl-check/csv_demo"))).unwrap();
        let sample = resolve_sample_input(&SampleInput::DefaultFile, Some(source.as_path()));

        assert_eq!(
            source,
            PathBuf::from("examples/wpl-check/csv_demo/rule.wpl")
        );
        assert_eq!(
            sample,
            SampleInput::File(PathBuf::from("examples/wpl-check/csv_demo/sample.txt"))
        );
    }

    #[test]
    fn test_explicit_relative_sample_path_is_not_rebased() {
        let source = PathBuf::from("examples/wpl-check/package_demo/rule.wpl");
        let sample = resolve_sample_input(
            &SampleInput::File(PathBuf::from("custom.txt")),
            Some(source.as_path()),
        );

        assert_eq!(sample, SampleInput::File(PathBuf::from("custom.txt")));
    }

    #[test]
    fn test_load_sample_data_preserves_trailing_newline() {
        let path = PathBuf::from(format!(
            "/tmp/wpl_check_sample_{}_{}.txt",
            std::process::id(),
            "newline"
        ));
        std::fs::write(&path, "42,alice,\n").unwrap();

        let loaded = load_sample_data(&SampleInput::File(path.clone())).unwrap();
        assert_eq!(loaded, "42,alice,\n");

        let _ = std::fs::remove_file(path);
    }
}
