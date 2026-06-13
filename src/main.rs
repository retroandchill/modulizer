use crate::cli::args::{ApplyCliArgs, CliArgs};
use clap::Parser;
use modulizer::config::{Config, ConfigBuilder};

mod cli;

fn main() -> anyhow::Result<()> {
    let args = CliArgs::try_parse()?;
    let config = ConfigBuilder::default().apply_cli_args(args)?.build()?;
    config.output_module()?;

    Ok(())
}
