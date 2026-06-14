use crate::parser::grammar::preprocessor::DefineDirective;
use derive_builder::Builder;
use regex::Regex;
use std::collections::HashMap;
use std::path::PathBuf;
use ustr::{UstrMap, UstrSet};

#[derive(Debug, Default, Builder)]
pub struct PreprocessorConfig {
    #[builder(setter(each(name = "include_dir")))]
    include_dirs: Vec<PathBuf>,

    #[builder(setter(strip_option))]
    pub header_guard_format: Option<Regex>,

    #[builder(setter(each(name = "expand_macro")), default)]
    expand_macros: UstrSet,

    #[builder(setter(each(name = "macro_definition")), default)]
    macro_definitions: UstrMap<DefineDirective>,
}
