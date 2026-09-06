// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use super::cache::KeyMap;
use crate::ffi::OwnedPyObject;
use pyo3::ffi::*;

#[allow(non_snake_case)]
pub struct State {
    pub MsgpackDecodeError: OwnedPyObject,
    pub key_map: KeyMap<512>,
}

impl State {
    #[cold]
    pub fn new() -> Option<Self> {
        unsafe {
            Some(Self {
                MsgpackDecodeError: OwnedPyObject::from_borrowed_ptr(PyExc_ValueError)?,
                key_map: KeyMap::new(),
            })
        }
    }
}
