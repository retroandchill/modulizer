use crate::ffi::core::{FFIError, expect_success_create};
use crate::ffi::strings::StringView;
use crate::parser::grammar::preprocessor::DefineDirective;
use std::ffi::CString;

#[repr(C)]
pub struct FFIPreprocessorConfig {
    pub include_dirs: *const StringView,
    pub include_dir_count: usize,
    pub header_guard_format: StringView,
    pub expand_macros: *const StringView,
    pub expand_macro_count: usize,
    pub macro_definitions: *const *const DefineDirective,
    pub macro_definition_count: usize,
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_create_define_directive(
    definition: StringView,
) -> *const DefineDirective {
    expect_success_create(|| {
        if definition.is_empty() {
            return Err(FFIError {
                message: CString::new("The definition cannot be empty")?,
            });
        }

        Ok(Box::new(DefineDirective::from_str(definition.as_str()?)))
    }) as *const DefineDirective
}

#[unsafe(no_mangle)]
pub extern "C" fn modulzier_destroy_define_directive(directive: *const DefineDirective) {
    if directive.is_null() {
        return;
    }

    unsafe {
        drop(Box::from_raw(directive as *mut DefineDirective));
    }
}
