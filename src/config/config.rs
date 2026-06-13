use crate::config::cli::CliArgs;
use crate::config::file::FileConfig;
use clap::Parser;
use itertools::Itertools;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashSet;
use std::path::PathBuf;
use ustr::Ustr;

#[derive(Debug)]
pub struct Config {
    pub name: String,
    pub output_path: PathBuf,

    pub library_headers: Vec<ConfigIncludePath>,
    pub include_dirs: Vec<PathBuf>,
    pub header_guard_format: Option<Regex>,

    pub expand_from_definition: HashSet<Ustr>,
    pub explicit_macros: Vec<String>,
    pub implementation_macros: HashSet<Ustr>,

    pub exclude: HashSet<String>,
    pub include: HashSet<String>,
}

#[derive(Debug)]
pub struct ModuleConfig {}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ConfigIncludePath {
    Unconditional(PathBuf),
    Conditional { path: PathBuf, if_defined: String },
}

impl ConfigIncludePath {
    pub fn path(&self) -> &PathBuf {
        match self {
            Self::Unconditional(path) => path,
            Self::Conditional { path, .. } => path,
        }
    }
}

impl Config {
    pub fn load_from_args() -> anyhow::Result<Self> {
        let cli = CliArgs::try_parse()?;
        Self::load(cli)
    }

    pub fn load(cli: CliArgs) -> anyhow::Result<Self> {
        let source_config = FileConfig::load(cli.config)
            .inspect_err(|e| {
                println!("Failed to load config file: {}", e);
            })
            .unwrap_or_default();
        let name = cli
            .module_name
            .or(source_config.module.name)
            .ok_or_else(|| anyhow::anyhow!("Module name is required"))?;
        let output_path = match cli.output.or(source_config.module.output) {
            Some(path) => path,
            None => std::env::current_dir()?.join(format!("{name}.ixx")),
        };

        let mut explicit_macros = source_config.macros.explicit_macros;
        explicit_macros.extend(cli.defines);

        let expand_from_definition = source_config
            .macros
            .expand_from_definition
            .iter()
            .chain(cli.expand.iter())
            .map(|s| s.as_str())
            .map(Ustr::from)
            .collect();

        let implementation_macros = source_config
            .macros
            .implementation_macros
            .iter()
            .chain(cli.implementation_macros.iter())
            .map(|s| s.as_str())
            .map(Ustr::from)
            .collect();

        Ok(Self {
            name,
            output_path,
            library_headers: cli
                .headers
                .into_iter()
                .map(ConfigIncludePath::Unconditional)
                .chain(source_config.headers.library_headers)
                .unique_by(|h| h.path().clone())
                .sorted_by(|a, b| a.path().cmp(b.path()))
                .collect(),
            include_dirs: cli
                .include_dirs
                .into_iter()
                .chain(source_config.headers.include_dirs)
                .unique()
                .collect(),
            header_guard_format: cli
                .header_guard
                .and_then(|s| Regex::new(&s).ok())
                .or(source_config.headers.header_guard_format),
            explicit_macros,
            expand_from_definition,
            implementation_macros,
            exclude: cli
                .exclude_symbols
                .into_iter()
                .chain(source_config.symbols.exclude)
                .unique()
                .collect(),
            include: cli
                .include_symbols
                .into_iter()
                .chain(source_config.symbols.include)
                .unique()
                .collect(),
        })
    }
}
