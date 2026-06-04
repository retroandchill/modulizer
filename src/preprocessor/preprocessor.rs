use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::path::PathBuf;

pub struct PreprocessorOutput {
    pub source: String,
    
}

#[derive(Default)]
struct PreprocessorState {
    source: String,
    included: HashSet<PathBuf>,
    definitions: HashMap<String, String>,
}

pub fn preprocess(source: &str, include_paths: &[PathBuf]) -> anyhow::Result<PreprocessorOutput> {
    let mut result = PreprocessorState::default();

    for line in source.lines() {
        result.source.write_str(line)?;
    }

    Ok(PreprocessorOutput {
        source: result.source,
    })
}