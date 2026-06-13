use modulizer::config::Config;

fn main() -> anyhow::Result<()> {
    let config = Config::load_from_args()?;
    config.output_module()?;

    Ok(())
}
