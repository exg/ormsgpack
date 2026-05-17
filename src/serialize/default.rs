// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::*;
use crate::msgpack::RECURSION_LIMIT;
use crate::util::unlikely;

use std::cell::Cell;
use std::ffi::CStr;

pub enum Error<'a> {
    InvalidType(BorrowedPyObject<'a>),
    RecursionLimitReached,
}

impl std::fmt::Display for Error<'_> {
    #[cold]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Error::InvalidType(obj) => {
                let name = unsafe {
                    let ob_type = pyo3::ffi::Py_TYPE(obj.as_ptr());
                    CStr::from_ptr((*ob_type).tp_name).to_string_lossy()
                };
                write!(f, "Type is not msgpack serializable: {name}")
            }
            Error::RecursionLimitReached => f.write_str("Recursion limit for default hook reached"),
        }
    }
}

pub struct DefaultHook<'a> {
    pub inner: Option<BorrowedPyObject<'a>>,
    recursion: Cell<u8>,
}

pub struct DefaultHookCall<'a, 'py> {
    hook: &'a DefaultHook<'py>,
    result: Option<OwnedPyObject>,
}

impl DefaultHookCall<'_, '_> {
    pub fn result(&self) -> &OwnedPyObject {
        self.result.as_ref().unwrap()
    }
}

impl Drop for DefaultHookCall<'_, '_> {
    fn drop(&mut self) {
        drop(self.result.take());
        let recursion = self.hook.recursion.get();
        self.hook.recursion.set(recursion - 1);
    }
}

impl<'a> DefaultHook<'a> {
    pub fn new(default: Option<BorrowedPyObject<'a>>) -> Self {
        DefaultHook {
            inner: default,
            recursion: Cell::new(0),
        }
    }

    pub fn call(&self, obj: BorrowedPyObject<'a>) -> Result<DefaultHookCall<'_, 'a>, Error<'a>> {
        match self.inner {
            Some(callable) => {
                let recursion = self.recursion.get();
                if unlikely(recursion == RECURSION_LIMIT) {
                    return Err(Error::RecursionLimitReached);
                }
                if let Some(result) = callable.call_one_arg(obj) {
                    self.recursion.set(recursion + 1);
                    Ok(DefaultHookCall {
                        hook: self,
                        result: Some(result),
                    })
                } else {
                    Err(Error::InvalidType(obj))
                }
            }
            None => Err(Error::InvalidType(obj)),
        }
    }
}
