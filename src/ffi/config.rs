use crate::config::{IncludePath, Options, OptionsBuilder};
use crate::ffi::core::{
    collapse_to_ustr_set, collapse_to_vec, expect_success, expect_success_create,
};
use crate::ffi::strings::StringView;
use regex::Regex;
use std::path::PathBuf;
use std::string::FromUtf8Error;
use ustr::Ustr;

#[repr(C)]
pub struct FFIOptionsBuilder {
    inner: OptionsBuilder,
}

#[repr(C)]
pub struct FFIOptions {
    options: Options,
}

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
pub extern "C" fn modulizer_builder_create() -> *mut FFIOptionsBuilder {
    let builder = OptionsBuilder::default();
    Box::into_raw(Box::new(FFIOptionsBuilder { inner: builder }))
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_destroy(builder: *mut FFIOptionsBuilder) {
    if builder.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(builder));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_set_name(
    builder: *mut FFIOptionsBuilder,
    name: StringView,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
    };

    expect_success(|| {
        builder.name(String::try_from(name)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_set_output_path(
    builder: *mut FFIOptionsBuilder,
    path: StringView,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
    };

    expect_success(|| {
        builder.output_path(PathBuf::try_from(path)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_library_header(
    builder: *mut FFIOptionsBuilder,
    path: *const FFIIncludePath,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
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
    builder: *mut FFIOptionsBuilder,
    paths: *const FFIIncludePath,
    count: usize,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
    };

    expect_success(|| {
        builder.library_headers(collapse_to_vec(paths, count, IncludePath::try_from)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_include_dir(
    builder: *mut FFIOptionsBuilder,
    path: StringView,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
    };

    expect_success(|| {
        builder.include_dir(PathBuf::try_from(path)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_include_dirs(
    builder: *mut FFIOptionsBuilder,
    paths: *const StringView,
    count: usize,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
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
    builder: *mut FFIOptionsBuilder,
    format: StringView,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
    };

    expect_success(|| {
        builder.header_guard_format(Regex::try_from(format)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_expand_macro_from_definition(
    builder: *mut FFIOptionsBuilder,
    macro_name: StringView,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
    };

    expect_success(|| {
        builder.expand_macro_from_definition(Ustr::try_from(macro_name)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_expand_macros_from_definition(
    builder: *mut FFIOptionsBuilder,
    names: *const StringView,
    count: usize,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
    };

    expect_success(|| {
        builder.expand_macros_from_definition(collapse_to_ustr_set(names, count)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_explicit_macro(
    builder: *mut FFIOptionsBuilder,
    definition: StringView,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
    };

    expect_success(|| {
        builder.explicit_macro(String::try_from(definition)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_explicit_macros(
    builder: *mut FFIOptionsBuilder,
    definitions: *const StringView,
    count: usize,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
    };
    expect_success(|| {
        builder.expand_macros_from_definition(collapse_to_ustr_set(definitions, count)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_implementation_macro(
    builder: *mut FFIOptionsBuilder,
    name: StringView,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
    };

    expect_success(|| {
        builder.implementation_macro(Ustr::try_from(name)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_implementation_macros(
    builder: *mut FFIOptionsBuilder,
    names: *const StringView,
    count: usize,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
    };

    expect_success(|| {
        builder.expand_macros_from_definition(collapse_to_ustr_set(names, count)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_exclude_symbol(
    builder: *mut FFIOptionsBuilder,
    name: StringView,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
    };

    expect_success(|| {
        builder.exclude_symbol(Ustr::try_from(name)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_exclude_symbols(
    builder: *mut FFIOptionsBuilder,
    names: *const StringView,
    count: usize,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
    };

    expect_success(|| {
        builder.expand_macros_from_definition(collapse_to_ustr_set(names, count)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_include_symbol(
    builder: *mut FFIOptionsBuilder,
    name: StringView,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
    };

    expect_success(|| {
        builder.include_symbol(Ustr::try_from(name)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_include_symbols(
    builder: *mut FFIOptionsBuilder,
    names: *const StringView,
    count: usize,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
    };
    expect_success(|| {
        builder.expand_macros_from_definition(collapse_to_ustr_set(names, count)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_options_create(builder: *const FFIOptionsBuilder) -> *mut FFIOptions {
    let builder = unsafe {
        assert!(!builder.is_null());
        &(*builder).inner
    };
    expect_success_create(|| {
        Ok(Box::from(FFIOptions {
            options: builder.build()?,
        }))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_options_destroy(options: *mut FFIOptions) {
    if options.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(options));
    }
}
