use std::fmt::Write;
use std::path::PathBuf;

pub fn preprocess(source: &str, include_paths: &[PathBuf]) -> anyhow::Result<String> {
    let mut result = String::new();

    for line in source.lines() {
        result.write_str(line)?;
    }

    Ok(result)
}