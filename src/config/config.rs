use derive_builder::Builder;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use std::path::PathBuf;
use ustr::UstrSet;

static MODULE_NAME_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*(?:\.[a-zA-Z_][a-zA-Z0-9_]*)*$").unwrap());

static MACRO_NAME_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$").unwrap());

#[derive(Debug, Default, Builder)]
#[builder(build_fn(validate = "Self::validate"))]
pub struct Config {
    pub name: String,

    #[builder(default = "self.default_output_path()?")]
    pub output_path: PathBuf,

    #[builder(setter(each(name = "library_header")))]
    pub library_headers: Vec<IncludePath>,

    #[builder(setter(each(name = "include_dir")))]
    pub include_dirs: Vec<PathBuf>,

    #[builder(setter(strip_option))]
    pub header_guard_format: Option<Regex>,

    #[builder(setter(each(name = "expand_macro_from_definition")), default)]
    pub expand_macros_from_definition: UstrSet,

    #[builder(setter(each(name = "explicit_macro")))]
    pub explicit_macros: Vec<String>,

    #[builder(setter(each(name = "implementation_macro")), default)]
    pub implementation_macros: UstrSet,

    #[builder(setter(each(name = "exclude_symbol")), default)]
    pub exclude_symbols: UstrSet,

    #[builder(setter(each(name = "include_symbol")), default)]
    pub include_symbols: UstrSet,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum IncludePath {
    Unconditional(PathBuf),
    IfDefinined { path: PathBuf, if_defined: String },
    IfConditioned { path: PathBuf, condition: String },
}

impl IncludePath {
    pub fn path(&self) -> &PathBuf {
        match self {
            Self::Unconditional(path) => path,
            Self::IfDefinined { path, .. } => path,
            Self::IfConditioned { path, .. } => path,
        }
    }
}

impl ConfigBuilder {
    fn default_output_path(&self) -> Result<PathBuf, String> {
        self.name
            .as_ref()
            .map(|name| PathBuf::from(format!("{}.ixx", name)))
            .ok_or("No name configured".to_string())
    }

    fn validate(&self) -> Result<(), String> {
        if let Some(ref name) = self.name {
            if !MODULE_NAME_REGEX.is_match(name) {
                return Err("Module is not a valid C++ module name".to_string());
            }
        }

        for macro_name in self.expand_macros_from_definition.iter().flatten() {
            if !MACRO_NAME_REGEX.is_match(macro_name) {
                return Err(format!(
                    "Macro name `{}` is not a valid C++ macro name",
                    macro_name
                ));
            }
        }

        if let Some(ref excluded_symbols) = self.exclude_symbols {
            for included_symbol in self.include_symbols.iter().flatten() {
                if excluded_symbols.contains(included_symbol) {
                    return Err(format!(
                        "Symbol `{}` is both excluded and included",
                        included_symbol
                    ));
                }
            }
        }

        Ok(())
    }
}
