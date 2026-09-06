// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::pybytes_as_bytes;
use crate::ffi::PyObjectWithType;
use serde::ser::{Serialize, Serializer};

#[repr(transparent)]
pub struct Bytes {
    ptr: *mut pyo3::ffi::PyObject,
}

impl Bytes {
    #[inline]
    pub fn try_new(obj: PyObjectWithType) -> Option<Self> {
        if obj.get_type_ptr() == &raw mut pyo3::ffi::PyBytes_Type {
            Some(Self { ptr: obj.as_ptr() })
        } else {
            None
        }
    }
}

impl Serialize for Bytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let contents = unsafe { pybytes_as_bytes(self.ptr) };
        serializer.serialize_bytes(contents)
    }
}
