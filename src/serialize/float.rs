// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::PyObjectWithType;
use serde::ser::{Serialize, Serializer};

#[repr(transparent)]
pub struct Float {
    ptr: *mut pyo3::ffi::PyObject,
}

impl Float {
    #[inline]
    pub fn try_new(obj: PyObjectWithType) -> Option<Self> {
        if obj.get_type_ptr() == &raw mut pyo3::ffi::PyFloat_Type {
            Some(Self { ptr: obj.as_ptr() })
        } else {
            None
        }
    }
}

impl Serialize for Float {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = unsafe { pyo3::ffi::PyFloat_AS_DOUBLE(self.ptr) };
        serializer.serialize_f64(value)
    }
}
