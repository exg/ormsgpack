// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::{deserialize, serialize};
use pyo3::ffi::*;

pub struct State {
    pub serialize: serialize::State,
    pub deserialize: deserialize::State,
    pub default_str: *mut PyObject,
    pub ext_hook_str: *mut PyObject,
    pub option_str: *mut PyObject,
}

impl State {
    #[cold]
    pub fn new() -> Self {
        unsafe {
            Self {
                serialize: serialize::State::new(),
                deserialize: deserialize::State::new(),
                default_str: PyUnicode_InternFromString(c"default".as_ptr()),
                ext_hook_str: PyUnicode_InternFromString(c"ext_hook".as_ptr()),
                option_str: PyUnicode_InternFromString(c"option".as_ptr()),
            }
        }
    }
}
