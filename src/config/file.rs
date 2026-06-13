use crate::config::config::{ConfigBuilder, IncludePath};
use regex::Regex;
use serde::Deserialize;
use std::path::PathBuf;
use ustr::Ustr;

#[derive(Debug, Deserialize, Default)]
pub struct FileConfig {
    pub module: FileModuleConfig,
    #[serde(default)]
    pub headers: FileHeaderConfig,
    #[serde(default)]
    pub macros: FileMacroConfig,
    #[serde(default)]
    pub symbols: FileSymbolConfig,
}

#[derive(Debug, Deserialize, Default)]
pub struct FileModuleConfig {
    pub name: Option<String>,
    pub output: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Default)]
pub struct FileHeaderConfig {
    #[serde(default)]
    pub library_headers: Vec<IncludePath>,

    #[serde(default)]
    pub include_dirs: Vec<PathBuf>,

    #[serde(default, with = "serde_regex")]
    pub header_guard_format: Option<Regex>,
}

#[derive(Debug, Deserialize, Default)]
pub struct FileMacroConfig {
    #[serde(default)]
    pub expand_from_definition: Vec<String>,

    #[serde(default)]
    pub explicit_macros: Vec<String>,

    #[serde(default)]
    pub implementation_macros: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct FileSymbolConfig {
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub include: Vec<String>,
}

impl FileConfig {
    pub fn load(target_path: PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(target_path)?;
        Ok(toml::from_str(&content)?)
    }
}

impl ConfigBuilder {
    pub fn apply_file_config(&mut self, config: FileConfig) -> &mut Self {
        if let Some(name) = config.module.name {
            self.name(name);
        }

        if let Some(output) = config.module.output {
            self.output_path(output);
        }

        for header in config.headers.library_headers {
            self.library_header(header);
        }

        for include_dir in config.headers.include_dirs {
            self.include_dir(include_dir);
        }

        if let Some(header_guard_format) = config.headers.header_guard_format {
            self.header_guard_format(header_guard_format);
        }

        for macro_name in &config.macros.expand_from_definition {
            self.expand_macro_from_definition(Ustr::from(&macro_name));
        }

        for explicit_macro in config.macros.explicit_macros {
            self.explicit_macro(explicit_macro);
        }

        for implementation_macro in &config.macros.implementation_macros {
            self.implementation_macro(Ustr::from(&implementation_macro));
        }

        for exclusion in &config.symbols.exclude {
            self.exclude_symbol(Ustr::from(&exclusion));
        }

        for inclusion in &config.symbols.include {
            self.include_symbol(Ustr::from(&inclusion));
        }

        self
    }
}
