use crate::cli::args::CliArgs;
use crate::config::file::FileConfig;
use crate::config::{IncludePath, OptionsBuilder};
use crate::ffi::core::{
    collapse_to_ustr_set, collapse_to_vec, expect_success, expect_success_create,
};
use crate::ffi::strings::StringView;
use crate::writer::cpp_output::GenerationResult;
use clap::Parser;
use regex::Regex;
use std::path::PathBuf;
use std::string::FromUtf8Error;
use ustr::Ustr;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FFIIfdefInclude {
    path: StringView,
    if_defined: StringView,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FFIConditionalInclude {
    path: StringView,
    condition: StringView,
}

#[repr(C)]
pub union FFIIncludePathData {
    unconditional: StringView,
    if_defined: FFIIfdefInclude,
    conditional: FFIConditionalInclude,
}

#[repr(C)]
pub enum FFIIncludePathKind {
    Unconditional = 0,
    IfDefined = 1,
    Conditional = 2,
}

#[repr(C)]
pub struct FFIIncludePath {
    kind: FFIIncludePathKind,
    data: FFIIncludePathData,
}

impl TryFrom<&FFIIncludePath> for IncludePath {
    type Error = FromUtf8Error;
    fn try_from(value: &FFIIncludePath) -> Result<Self, Self::Error> {
        Ok(match value.kind {
            FFIIncludePathKind::Unconditional => unsafe {
                IncludePath::Unconditional(PathBuf::try_from(value.data.unconditional)?)
            },
            FFIIncludePathKind::IfDefined => unsafe {
                IncludePath::IfDefined {
                    path: PathBuf::try_from(value.data.if_defined.path)?,
                    if_defined: String::try_from(value.data.if_defined.if_defined)?,
                }
            },
            FFIIncludePathKind::Conditional => unsafe {
                IncludePath::Conditioned {
                    path: PathBuf::try_from(value.data.conditional.path)?,
                    condition: String::try_from(value.data.conditional.condition)?,
                }
            },
        })
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_create() -> *mut OptionsBuilder {
    let builder = OptionsBuilder::default();
    Box::into_raw(Box::new(builder))
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_destroy(builder: *mut OptionsBuilder) {
    if builder.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(builder));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_set_name(
    builder: *mut OptionsBuilder,
    name: StringView,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder)
    };

    expect_success(|| {
        builder.name(String::try_from(name)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_set_output_path(
    builder: *mut OptionsBuilder,
    path: StringView,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder)
    };

    expect_success(|| {
        builder.output_path(PathBuf::try_from(path)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_library_header(
    builder: *mut OptionsBuilder,
    path: *const FFIIncludePath,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder)
    };
    let path = unsafe {
        assert!(!path.is_null());
        &(*path)
    };

    expect_success(|| {
        builder.library_header(IncludePath::try_from(path)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_library_headers(
    builder: *mut OptionsBuilder,
    paths: *const FFIIncludePath,
    count: usize,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder)
    };

    expect_success(|| {
        builder.library_headers(collapse_to_vec(paths, count, IncludePath::try_from)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_include_dir(
    builder: *mut OptionsBuilder,
    path: StringView,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder)
    };

    expect_success(|| {
        builder.include_dir(PathBuf::try_from(path)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_include_dirs(
    builder: *mut OptionsBuilder,
    paths: *const StringView,
    count: usize,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder)
    };

    expect_success(|| {
        builder.include_dirs(collapse_to_vec(paths, count, |path| {
            PathBuf::try_from(*path)
        })?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_set_header_guard_format(
    builder: *mut OptionsBuilder,
    format: StringView,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder)
    };

    expect_success(|| {
        builder.header_guard_format(Regex::try_from(format)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_expand_macro_from_definition(
    builder: *mut OptionsBuilder,
    macro_name: StringView,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder)
    };

    expect_success(|| {
        builder.expand_macro_from_definition(Ustr::try_from(macro_name)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_expand_macros_from_definition(
    builder: *mut OptionsBuilder,
    names: *const StringView,
    count: usize,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder)
    };

    expect_success(|| {
        builder.expand_macros_from_definition(collapse_to_ustr_set(names, count)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_explicit_macro(
    builder: *mut OptionsBuilder,
    definition: StringView,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder)
    };

    expect_success(|| {
        builder.explicit_macro(String::try_from(definition)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_explicit_macros(
    builder: *mut OptionsBuilder,
    definitions: *const StringView,
    count: usize,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder)
    };
    expect_success(|| {
        builder.expand_macros_from_definition(collapse_to_ustr_set(definitions, count)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_implementation_macro(
    builder: *mut OptionsBuilder,
    name: StringView,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder)
    };

    expect_success(|| {
        builder.implementation_macro(Ustr::try_from(name)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_implementation_macros(
    builder: *mut OptionsBuilder,
    names: *const StringView,
    count: usize,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder)
    };

    expect_success(|| {
        builder.expand_macros_from_definition(collapse_to_ustr_set(names, count)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_exclude_symbol(
    builder: *mut OptionsBuilder,
    name: StringView,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder)
    };

    expect_success(|| {
        builder.exclude_symbol(Ustr::try_from(name)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_exclude_symbols(
    builder: *mut OptionsBuilder,
    names: *const StringView,
    count: usize,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder)
    };

    expect_success(|| {
        builder.expand_macros_from_definition(collapse_to_ustr_set(names, count)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_include_symbol(
    builder: *mut OptionsBuilder,
    name: StringView,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder)
    };

    expect_success(|| {
        builder.include_symbol(Ustr::try_from(name)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_include_symbols(
    builder: *mut OptionsBuilder,
    names: *const StringView,
    count: usize,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder)
    };
    expect_success(|| {
        builder.expand_macros_from_definition(collapse_to_ustr_set(names, count)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_from_config_file(
    builder: *mut OptionsBuilder,
    path: StringView,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder)
    };
    expect_success(|| {
        builder.apply_file_config(FileConfig::load(PathBuf::try_from(path)?)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_from_cli_args(
    builder: *mut OptionsBuilder,
    argv: *const StringView,
    argc: usize,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder)
    };

    expect_success(|| {
        let argv = unsafe { std::slice::from_raw_parts(argv, argc) };
        let args = argv
            .into_iter()
            .map(|arg| String::try_from(*arg))
            .collect::<Result<Vec<_>, _>>()?;
        builder.apply_cli_args(CliArgs::try_parse_from(args)?)?;
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_generate(builder: *const OptionsBuilder) -> *mut GenerationResult {
    let builder = unsafe {
        assert!(!builder.is_null());
        &(*builder)
    };
    expect_success_create(|| {
        let options = builder.build()?;
        Ok(Box::new(options.output_module()?))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_generation_result_destroy(result: *mut GenerationResult) {
    if result.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(result));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_generation_result_get_output_path(
    result: *const GenerationResult,
) -> StringView {
    let result = unsafe {
        assert!(!result.is_null());
        &(*result)
    };
    result.output_path.to_str().unwrap_or("").into()
}
