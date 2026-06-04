use std::path::PathBuf;

use clap::Parser;

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

    /// Add or override a preprocessor definition.
    ///
    /// Examples:
    /// - `-D FOO`
    /// - `-D FOO=1`
    /// - `-D FOO=bar`
    #[arg(short = 'D', long = "define")]
    pub defines: Vec<String>,

    /// Add a header entry point.
    #[arg(long = "header")]
    pub headers: Vec<PathBuf>,

    /// Add a symbol to export.
    #[arg(long = "export-symbol")]
    pub export_symbols: Vec<String>,

    /// Add a namespace to export.
    #[arg(long = "export-namespace")]
    pub export_namespaces: Vec<String>,
}