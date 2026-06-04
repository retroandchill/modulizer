use crate::config::config::Config;

pub mod config;

fn main() -> anyhow::Result<()> {
    let config = Config::load_from_args()?;

    println!("{config:#?}");

    Ok(())
}
