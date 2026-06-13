use crate::config::{IncludePath, Options, OptionsBuilder};
use crate::ffi::core::{expect_success, expect_success_create};
use crate::ffi::strings::StringView;
use regex::Regex;
use std::path::PathBuf;
use ustr::{Ustr, UstrSet};

#[repr(C)]
pub struct FFIOptionsBuilder {
    inner: OptionsBuilder,
}

#[repr(C)]
pub struct FFIOptions {
    options: Options,
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
pub extern "C" fn modulizer_builder_add_library_header(
    builder: *mut FFIOptionsBuilder,
    path: StringView,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
    };

    expect_success(|| {
        builder.library_header(IncludePath::Unconditional(PathBuf::try_from(path)?));
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_add_library_header_if_defined(
    builder: *mut FFIOptionsBuilder,
    path: StringView,
    macro_name: StringView,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
    };

    expect_success(|| {
        builder.library_header(IncludePath::IfDefined {
            path: PathBuf::try_from(path)?,
            if_defined: String::try_from(macro_name)?,
        });
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_add_library_header_conditioned(
    builder: *mut FFIOptionsBuilder,
    path: StringView,
    condition: StringView,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
    };

    expect_success(|| {
        builder.library_header(IncludePath::Conditioned {
            path: PathBuf::try_from(path)?,
            condition: String::try_from(condition)?,
        });
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_clear_library_headers(builder: *mut FFIOptionsBuilder) {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
    };

    builder.library_headers(Vec::new());
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_add_include_dir(
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
pub extern "C" fn modulizer_builder_clear_include_dirs(builder: *mut FFIOptionsBuilder) {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
    };
    builder.include_dirs(Vec::new());
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
pub extern "C" fn modulizer_builder_add_expand_macro_from_definition(
    builder: *mut FFIOptionsBuilder,
    path: StringView,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
    };

    expect_success(|| {
        builder.expand_macro_from_definition(Ustr::try_from(path)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_clear_expand_macros_from_definition(
    builder: *mut FFIOptionsBuilder,
) {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
    };
    builder.expand_macros_from_definition(UstrSet::default());
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_add_explicit_macro(
    builder: *mut FFIOptionsBuilder,
    path: StringView,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
    };

    expect_success(|| {
        builder.explicit_macro(String::try_from(path)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_clear_explicit_macros(builder: *mut FFIOptionsBuilder) {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
    };
    builder.explicit_macros(vec![]);
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_add_implementation_macro(
    builder: *mut FFIOptionsBuilder,
    path: StringView,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
    };

    expect_success(|| {
        builder.implementation_macro(Ustr::try_from(path)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_clear_implementation_macros(builder: *mut FFIOptionsBuilder) {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
    };
    builder.implementation_macros(UstrSet::default());
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_exclude_symbol(
    builder: *mut FFIOptionsBuilder,
    path: StringView,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
    };

    expect_success(|| {
        builder.exclude_symbol(Ustr::try_from(path)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_clear_excluded_symbols(builder: *mut FFIOptionsBuilder) {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
    };
    builder.exclude_symbols(UstrSet::default());
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_include_symbol(
    builder: *mut FFIOptionsBuilder,
    path: StringView,
) -> bool {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
    };

    expect_success(|| {
        builder.include_symbol(Ustr::try_from(path)?);
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_builder_clear_included_symbols(builder: *mut FFIOptionsBuilder) {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
    };
    builder.include_symbols(UstrSet::default());
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_options_create(builder: *mut FFIOptionsBuilder) -> *mut FFIOptions {
    let builder = unsafe {
        assert!(!builder.is_null());
        &mut (*builder).inner
    };
    expect_success_create(|| {
        Ok(Box::from(FFIOptions {
            options: builder.build()?,
        }))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_options_destroy(builder: *mut FFIOptions) {
    if builder.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(builder));
    }
}
