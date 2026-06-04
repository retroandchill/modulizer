use std::path::PathBuf;
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct FileConfig {
    pub module: FileModuleConfig,
    pub headers: FileHeaderConfig,
}

#[derive(Debug, Deserialize, Default)]
pub struct FileModuleConfig {
    pub name: Option<String>,
    pub output: Option<PathBuf>
}

#[derive(Debug, Deserialize, Default)]
pub struct FileHeaderConfig {
    pub library_headers: Vec<PathBuf>,
    pub include_dirs: Vec<PathBuf>
}

impl FileConfig {
    pub fn load(target_path: PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(target_path)?;
        Ok(toml::from_str(&content)?)
    }
}