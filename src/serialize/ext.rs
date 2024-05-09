// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::pybytes_as_bytes;
use crate::state::State;
use serde::ser::{Serialize, Serializer};
use serde_bytes::Bytes;

pub struct Ext {
    ptr: *mut pyo3::ffi::PyObject,
    state: *mut State,
}

impl Ext {
    pub fn new(ptr: *mut pyo3::ffi::PyObject, state: *mut State) -> Self {
        Ext {
            ptr: ptr,
            state: state,
        }
    }
}

impl Serialize for Ext {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let tag = unsafe {
            let value = pyo3::ffi::PyObject_GetAttr(self.ptr, (*self.state).tag_str);
            pyo3::ffi::Py_DECREF(value);
            pyo3::ffi::PyLong_AsLongLong(value)
        };
        if unlikely!(!(0..=127).contains(&tag)) {
            return Err(serde::ser::Error::custom("Extension type out of range"));
        }
        let data = unsafe {
            let value = pyo3::ffi::PyObject_GetAttr(self.ptr, (*self.state).data_str);
            pyo3::ffi::Py_DECREF(value);
            pybytes_as_bytes(value)
        };

        serializer.serialize_newtype_variant("", tag as u32, "", Bytes::new(data))
    }
}
