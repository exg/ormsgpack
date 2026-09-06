// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::PyObjectWithType;
use serde::ser::{Serialize, Serializer};

#[repr(transparent)]
pub struct Bool {
    ptr: *mut pyo3::ffi::PyObject,
}

impl Bool {
    #[inline]
    pub fn try_new(obj: PyObjectWithType) -> Option<Self> {
        if obj.get_type_ptr() == &raw mut pyo3::ffi::PyBool_Type {
            Some(Self { ptr: obj.as_ptr() })
        } else {
            None
        }
    }
}

impl Serialize for Bool {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = unsafe { self.ptr == pyo3::ffi::Py_True() };
        serializer.serialize_bool(value)
    }
}
