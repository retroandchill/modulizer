use clap::Parser;
use modulizer::config::file::FileConfig;
use modulizer::config::{IncludePath, OptionsBuilder};
use regex::Regex;
use std::path::PathBuf;
use std::str::FromStr;
use ustr::Ustr;

#[derive(Debug, Parser)]
#[command(name = "modulizer")]
#[command(about = "Generate C++20 module interface files from C++ headers")]
pub struct CliArgs {
    /// Path to the TOML configuration file.
    #[arg(short, long, default_value = "modulizer.toml")]
    pub config: PathBuf,

    /// Override the generated module name.
    #[arg(long)]
    pub module_name: Option<String>,

    /// Override the generated module interface output path.
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Add an include directory.
    #[arg(short = 'I', long = "include")]
    pub include_dirs: Vec<PathBuf>,

    /// Add an include directory.
    #[arg(short = 'G', long = "guard")]
    pub header_guard: Option<String>,

    /// Add or override a preprocessor definition.
    ///
    /// Examples:
    /// - `-D FOO`
    /// - `-D FOO=1`
    /// - `-D FOO=bar`
    #[arg(short = 'D', long = "define")]
    pub defines: Vec<String>,

    #[arg(short = 'E', long = "expand")]
    pub expand: Vec<String>,

    #[arg(short = 'i', long = "implementation-macro")]
    pub implementation_macros: Vec<String>,

    /// Add a header entry point.
    #[arg(long = "header")]
    pub headers: Vec<PathBuf>,

    /// Add a symbol to export.
    #[arg(long = "exclude-symbol")]
    pub exclude_symbols: Vec<String>,

    /// Add a symbol to export.
    #[arg(long = "include-symbol")]
    pub include_symbols: Vec<String>,
}

pub trait ApplyCliArgs {
    fn apply_cli_args(&mut self, cli: CliArgs) -> anyhow::Result<&mut Self>;
}

impl ApplyCliArgs for OptionsBuilder {
    fn apply_cli_args(&mut self, cli: CliArgs) -> anyhow::Result<&mut Self> {
        if cli.config.exists() {
            self.apply_file_config(FileConfig::load(cli.config)?);
        }

        if let Some(name) = cli.module_name {
            self.name(name);
        }

        if let Some(output) = cli.output {
            self.output_path(output);
        }

        for include_dir in cli.include_dirs {
            self.include_dir(include_dir);
        }

        if let Some(header_guard) = cli.header_guard {
            self.header_guard_format(Regex::from_str(&header_guard)?);
        }

        for define in &cli.defines {
            self.expand_macro_from_definition(Ustr::from(&define));
        }

        for macro_name in &cli.expand {
            self.expand_macro_from_definition(Ustr::from(&macro_name));
        }

        for impl_macro in &cli.implementation_macros {
            self.implementation_macro(Ustr::from(&impl_macro));
        }

        for header in cli.headers {
            self.library_header(IncludePath::Unconditional(header));
        }

        for exclusion in &cli.exclude_symbols {
            self.exclude_symbol(Ustr::from(&exclusion));
        }

        for inclusion in &cli.include_symbols {
            self.include_symbol(Ustr::from(&inclusion));
        }

        Ok(self)
    }
}
