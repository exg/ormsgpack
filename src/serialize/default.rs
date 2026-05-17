// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::*;
use crate::msgpack::RECURSION_LIMIT;
use crate::util::unlikely;

use std::cell::Cell;
use std::ffi::CStr;
use std::ptr::NonNull;

pub enum Error {
    InvalidType(*mut pyo3::ffi::PyObject),
    RecursionLimitReached,
}

impl std::fmt::Display for Error {
    #[cold]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Error::InvalidType(ptr) => {
                let name = unsafe { CStr::from_ptr((*ob_type!(ptr)).tp_name).to_string_lossy() };
                write!(f, "Type is not msgpack serializable: {name}")
            }
            Error::RecursionLimitReached => f.write_str("Recursion limit for default hook reached"),
        }
    }
}

pub struct DefaultHook {
    pub inner: Option<NonNull<pyo3::ffi::PyObject>>,
    recursion: Cell<u8>,
}

pub struct DefaultHookCall<'a> {
    hook: &'a DefaultHook,
    pub result: *mut pyo3::ffi::PyObject,
}

impl Drop for DefaultHookCall<'_> {
    fn drop(&mut self) {
        unsafe { pyo3::ffi::Py_DECREF(self.result) };
        let recursion = self.hook.recursion.get();
        self.hook.recursion.set(recursion - 1);
    }
}

impl DefaultHook {
    pub fn new(default: Option<NonNull<pyo3::ffi::PyObject>>) -> Self {
        DefaultHook {
            inner: default,
            recursion: Cell::new(0),
        }
    }

    pub fn call(&self, ptr: *mut pyo3::ffi::PyObject) -> Result<DefaultHookCall<'_>, Error> {
        match self.inner {
            Some(callable) => {
                let recursion = self.recursion.get();
                if unlikely(recursion == RECURSION_LIMIT) {
                    return Err(Error::RecursionLimitReached);
                }
                let default_obj = unsafe { pyobject_call_one_arg(callable.as_ptr(), ptr) };
                if unlikely(default_obj.is_null()) {
                    Err(Error::InvalidType(ptr))
                } else {
                    self.recursion.set(recursion + 1);
                    Ok(DefaultHookCall {
                        hook: self,
                        result: default_obj,
                    })
                }
            }
            None => Err(Error::InvalidType(ptr)),
        }
    }
}
