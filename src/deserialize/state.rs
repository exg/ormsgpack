// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use super::cache::KeyMap;
use pyo3::ffi::*;

#[allow(non_snake_case)]
pub struct State {
    pub MsgpackDecodeError: *mut PyObject,
    pub key_map: KeyMap<512>,
}

impl State {
    #[cold]
    pub fn new() -> Self {
        unsafe {
            Self {
                MsgpackDecodeError: Py_NewRef(PyExc_ValueError),
                key_map: KeyMap::new(),
            }
        }
    }
}
