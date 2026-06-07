use std::path::PathBuf;
use serde::Deserialize;
use regex::Regex;
use crate::config::config::ConfigIncludePath;

#[derive(Debug, Deserialize, Default)]
pub struct FileConfig {
    pub module: FileModuleConfig,
    #[serde(default)]
    pub headers: FileHeaderConfig,
    #[serde(default)]
    pub macros: FileMacroConfig,
}

#[derive(Debug, Deserialize, Default)]
pub struct FileModuleConfig {
    pub name: Option<String>,
    pub output: Option<PathBuf>
}

#[derive(Debug, Deserialize, Default)]
pub struct FileHeaderConfig {
    #[serde(default)]
    pub library_headers: Vec<ConfigIncludePath>,

    #[serde(default)]
    pub include_dirs: Vec<PathBuf>,

    #[serde(with = "serde_regex")]
    pub header_guard_format: Option<Regex>,
}

#[derive(Debug, Deserialize, Default)]
pub struct FileMacroConfig {
    #[serde(default)]
    pub expand_from_definition: Vec<String>,

    #[serde(default)]
    pub explicit_macros: Vec<String>
}

impl FileConfig {
    pub fn load(target_path: PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(target_path)?;
        Ok(toml::from_str(&content)?)
    }
}