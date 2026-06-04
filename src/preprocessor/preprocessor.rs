use std::fmt::Write;
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct PreprocessorOutput {
    pub source: String,
    
}

pub fn preprocess(source: &str, include_paths: &[PathBuf]) -> anyhow::Result<PreprocessorOutput> {
    let mut result = PreprocessorOutput::default();

    for line in source.lines() {
        result.source.write_str(line)?;
    }

    Ok(result)
}