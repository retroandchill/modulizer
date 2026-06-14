use clap::Parser;
use modulizer::cli::args::CliArgs;
use modulizer::config::OptionsBuilder;

fn main() -> anyhow::Result<()> {
    let args = CliArgs::try_parse()?;
    let config = OptionsBuilder::default().apply_cli_args(args)?.build()?;
    config.output_module()?;

    Ok(())
}
