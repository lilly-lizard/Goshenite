use std::{
    ffi::{CStr, CString, NulError},
    os::raw::c_char,
    str::Utf8Error,
};

pub fn string_to_c_string_vec(
    source: impl IntoIterator<Item = String>,
) -> Result<Vec<*const c_char>, NulError> {
    source
        .into_iter()
        .map(|name| Ok(CString::new(name)?.as_ptr()))
        .collect()
}

pub fn c_string_to_string(source: *const c_char) -> Result<String, Utf8Error> {
    let c_str = unsafe { CStr::from_ptr(source) };
    Ok(c_str.to_str()?.to_string())
}
