// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::*;
use crate::typeref::*;

use serde::ser::{Serialize, Serializer};
use serde_bytes::ByteBuf;

#[repr(transparent)]
pub struct Ext {
    ptr: *mut pyo3::ffi::PyObject,
}

impl Ext {
    pub fn new(ptr: *mut pyo3::ffi::PyObject) -> Self {
        Ext { ptr: ptr }
    }
}

impl Serialize for Ext {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = ffi!(PyObject_GetAttr(self.ptr, TAG_STR));
        let tag = ffi!(PyLong_AsLongLong(value));
        ffi!(Py_DECREF(value));
        if unlikely!(!(0..=127).contains(&tag)) {
            err!("Extension type out of range")
        }
        let value = ffi!(PyObject_GetAttr(self.ptr, DATA_STR));
        let buffer = unsafe { PyBytes_AS_STRING(value) as *const u8 };
        let length = unsafe { PyBytes_GET_SIZE(value) as usize };
        let data = unsafe { std::slice::from_raw_parts(buffer, length) };
        ffi!(Py_DECREF(value));

        serializer.serialize_newtype_variant("", tag as u32, "", &ByteBuf::from(data))
    }
}
