use std::path::PathBuf;
use clap::Parser;
use crate::config::cli::CliArgs;
use crate::config::file::FileConfig;

#[derive(Debug)]
pub struct Config {
    pub module: ModuleConfig,
    pub headers: HeaderConfig
}

#[derive(Debug)]
pub struct ModuleConfig {
    pub name: String,
    pub output_path: PathBuf,
}

#[derive(Debug)]
pub struct HeaderConfig {
    pub entry: PathBuf,
    pub include_dirs: Vec<PathBuf>,
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
        let output_path = match cli.output {
            Some(path) => path,
            None => {
                std::env::current_dir()?.join(format!("{name}.ixx"))
            }
        };
        Ok(Self {
                module: ModuleConfig {
                    name,
                    output_path
                },
                headers: HeaderConfig {
                    entry: PathBuf::new(),
                    include_dirs: cli.include_dirs
                }
            })
    }
}