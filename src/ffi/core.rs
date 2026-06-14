use crate::ffi::strings::StringView;
use std::cell::RefCell;
use std::ffi::{CString, c_char};
use std::fmt::Debug;
use std::str::Utf8Error;
use ustr::{Ustr, UstrSet};

thread_local! {
    static CURRENT_ERROR: RefCell<Option<FFIError>> = RefCell::new(None);
}

#[derive(Debug)]
pub struct FFIError {
    pub message: CString,
}

#[unsafe(no_mangle)]
pub extern "C" fn modulizer_get_last_error() -> *const c_char {
    CURRENT_ERROR.with(|error| {
        error
            .borrow()
            .as_ref()
            .map(|error| error.message.as_ptr())
            .unwrap_or(std::ptr::null())
    })
}

pub fn expect_success(delegate: impl FnOnce() -> anyhow::Result<()>) -> bool {
    match delegate() {
        Ok(_) => true,
        Err(error) => {
            CURRENT_ERROR.replace(Some(FFIError {
                message: CString::new(error.to_string()).unwrap(),
            }));
            false
        }
    }
}

pub fn expect_success_create<T>(delegate: impl FnOnce() -> anyhow::Result<Box<T>>) -> *mut T {
    match delegate() {
        Ok(value) => Box::into_raw(value),
        Err(error) => {
            CURRENT_ERROR.replace(Some(FFIError {
                message: CString::new(error.to_string()).unwrap(),
            }));
            std::ptr::null_mut()
        }
    }
}

pub fn collapse_to_vec<'a, T, E, I: 'a>(
    items: *const I,
    count: usize,
    delegate: impl Fn(&'a I) -> Result<T, E>,
) -> Result<Vec<T>, E> {
    if count == 0 {
        return Ok(vec![]);
    }

    let items = unsafe {
        assert!(!items.is_null());
        std::slice::from_raw_parts(items, count)
    };

    items
        .into_iter()
        .map(delegate)
        .collect::<Result<Vec<_>, _>>()
}

pub fn collapse_to_ustr_set(items: *const StringView, count: usize) -> Result<UstrSet, Utf8Error> {
    if count == 0 {
        return Ok(UstrSet::default());
    }

    let items = unsafe {
        assert!(!items.is_null());
        std::slice::from_raw_parts(items, count)
    };

    items
        .into_iter()
        .map(|item| Ustr::try_from(*item))
        .collect::<Result<UstrSet, _>>()
}
