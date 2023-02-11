use std::{
    ffi::{CString, NulError},
    os::raw::c_char,
};

pub fn to_c_string_vec(source: Vec<String>) -> Result<Vec<*const c_char>, NulError> {
    source
        .into_iter()
        .map(|name| Ok(CString::new(name)?.as_ptr()))
        .collect()
}
