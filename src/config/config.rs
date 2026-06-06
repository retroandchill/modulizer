use std::collections::HashSet;
use crate::config::cli::CliArgs;
use crate::config::file::FileConfig;
use clap::Parser;
use itertools::Itertools;
use std::path::PathBuf;

#[derive(Debug)]
pub struct Config {
    pub module: ModuleConfig,
    pub headers: HeaderConfig,
    pub macros: MacroConfig,
}

#[derive(Debug)]
pub struct ModuleConfig {
    pub name: String,
    pub output_path: PathBuf,
}

#[derive(Debug)]
pub struct HeaderConfig {
    pub library_headers: Vec<PathBuf>,
    pub include_dirs: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct MacroConfig {
    pub expand_from_definition: HashSet<String>,
    pub explicit_macros: Vec<String>,
}

impl Config {
    pub fn load_from_args() -> anyhow::Result<Self> {
        let cli = CliArgs::try_parse()?;
        Self::load(cli)
    }

    pub fn load(cli: CliArgs) -> anyhow::Result<Self> {
        let source_config = FileConfig::load(cli.config)
            .unwrap_or_default();
        let name = cli.module_name
            .or(source_config.module.name)
            .ok_or_else(|| anyhow::anyhow!("Module name is required"))?;
        let output_path = match cli.output.or(source_config.module.output) {
            Some(path) => path,
            None => {
                std::env::current_dir()?.join(format!("{name}.ixx"))
            }
        };

        let mut explicit_macros = source_config.macros.explicit_macros;
        explicit_macros.extend(cli.defines);

        let expand_from_definition = source_config.macros.expand_from_definition.into_iter()
            .chain(cli.expand.into_iter())
            .collect();

        Ok(Self {
                module: ModuleConfig {
                    name,
                    output_path
                },
                headers: HeaderConfig {
                    library_headers: cli.headers.into_iter()
                        .chain(source_config.headers.library_headers)
                        .unique()
                        .sorted()
                        .collect(),
                    include_dirs: cli.include_dirs.into_iter()
                        .chain(source_config.headers.include_dirs)
                        .unique()
                        .collect()
                },
                macros: MacroConfig {
                    explicit_macros,
                    expand_from_definition,
                }
            })
    }
}