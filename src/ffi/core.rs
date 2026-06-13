use std::cell::RefCell;
use std::error::Error;
use std::ffi::{CString, c_char};
use std::fmt::{Debug, Display, Formatter};

thread_local! {
    static CURRENT_ERROR: RefCell<Option<FFIError>> = RefCell::new(None);
}

#[derive(Debug)]
pub struct FFIError {
    pub message: CString,
}

impl<E> From<E> for FFIError
where
    E: Error,
{
    fn from(error: E) -> Self {
        Self {
            message: CString::new(error.to_string()).unwrap(),
        }
    }
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

pub fn expect_success(delegate: impl FnOnce() -> Result<(), FFIError>) -> bool {
    match delegate() {
        Ok(_) => true,
        Err(error) => {
            CURRENT_ERROR.replace(Some(error));
            false
        }
    }
}

pub fn expect_success_create<T>(delegate: impl FnOnce() -> Result<Box<T>, FFIError>) -> *mut T {
    match delegate() {
        Ok(value) => Box::into_raw(value),
        Err(error) => {
            CURRENT_ERROR.replace(Some(error));
            std::ptr::null_mut()
        }
    }
}
