use regex::Regex;
use std::ffi::c_char;
use std::path::PathBuf;
use std::slice;
use std::str::Utf8Error;
use ustr::Ustr;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StringView {
    data: *const u8,
    length: usize,
}

impl StringView {
    pub fn new(string: &str) -> Self {
        Self {
            data: string.as_ptr(),
            length: string.len(),
        }
    }

    pub unsafe fn from_raw(ptr: *const c_char, length: usize) -> Self {
        Self {
            data: ptr as *const u8,
            length,
        }
    }

    pub fn as_str(&self) -> Result<&str, Utf8Error> {
        unsafe { str::from_utf8(slice::from_raw_parts(self.data, self.length)) }
    }
}

impl TryFrom<StringView> for Ustr {
    type Error = Utf8Error;

    fn try_from(value: StringView) -> Result<Self, Self::Error> {
        value.as_str().map(Ustr::from)
    }
}

impl TryFrom<StringView> for String {
    type Error = std::string::FromUtf8Error;

    fn try_from(value: StringView) -> Result<Self, Self::Error> {
        unsafe { String::from_utf8(slice::from_raw_parts(value.data, value.length).to_vec()) }
    }
}

impl TryFrom<StringView> for PathBuf {
    type Error = std::string::FromUtf8Error;

    fn try_from(value: StringView) -> Result<Self, Self::Error> {
        String::try_from(value).map(PathBuf::from)
    }
}

impl TryFrom<StringView> for Regex {
    type Error = regex::Error;

    fn try_from(value: StringView) -> Result<Self, Self::Error> {
        value
            .as_str()
            .map_err(|error| regex::Error::Syntax(error.to_string()))
            .and_then(Regex::new)
    }
}
