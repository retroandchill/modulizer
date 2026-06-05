use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::path::PathBuf;
use crate::preprocessor::lex;

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
    let lexemes = lex(source);
    
    for lexeme in lexemes {
        result.source.push_str(&lexeme.as_source());
    }

    Ok(PreprocessorOutput {
        source: result.source,
    })
}