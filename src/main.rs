use crate::config::Config;

pub mod config;
pub mod writer;
pub mod parser;

fn main() -> anyhow::Result<()> {
    let config = Config::load_from_args()?;
    config.output_module()?;

    Ok(())
}
