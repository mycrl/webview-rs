use std::{
    ffi::{self, c_char},
    ptr::null,
};

pub(crate) struct CStrPtr {
    pub ptr: *const c_char,
}

unsafe impl Send for CStrPtr {}
unsafe impl Sync for CStrPtr {}

impl TryFrom<&str> for CStrPtr {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(Self {
            ptr: ffi::CString::new(value)?.into_raw(),
        })
    }
}

impl Drop for CStrPtr {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            drop(unsafe { ffi::CString::from_raw(self.ptr as *mut c_char) })
        }
    }
}

pub(crate) trait AsCStr {
    fn as_c_str(&self) -> CStrPtr;
}

impl AsCStr for String {
    fn as_c_str(&self) -> CStrPtr {
        CStrPtr::try_from(self.as_str()).unwrap()
    }
}

impl<'a> AsCStr for &'a str {
    fn as_c_str(&self) -> CStrPtr {
        CStrPtr::try_from(*self).unwrap()
    }
}

impl<'a> AsCStr for Option<&'a str> {
    fn as_c_str(&self) -> CStrPtr {
        self.map(|str| CStrPtr::try_from(str).unwrap())
            .unwrap_or(CStrPtr { ptr: null() })
    }
}

pub(crate) fn to_c_str(str: &str) -> *const c_char {
    ffi::CString::new(str).unwrap().into_raw()
}

pub(crate) fn opt_to_c_str(str: Option<&str>) -> *const c_char {
    str.map(|str| ffi::CString::new(str).unwrap().into_raw() as *const _)
        .unwrap_or(null())
}

pub(crate) fn release_c_str(str: *const c_char) {
    if !str.is_null() {
        drop(unsafe { ffi::CString::from_raw(str as *mut c_char) })
    }
}

pub(crate) fn from_c_str(str: *const c_char) -> Option<String> {
    if !str.is_null() {
        unsafe { ffi::CStr::from_ptr(str) }
            .to_str()
            .map(|s| s.to_string())
            .ok()
    } else {
        None
    }
}
