// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::PyObjectWithType;
use crate::ffi::{pybytearray_as_bytes, CriticalSection};
use serde::ser::{Serialize, Serializer};

#[repr(transparent)]
pub struct ByteArray {
    ptr: *mut pyo3::ffi::PyObject,
}

impl ByteArray {
    #[inline]
    pub fn try_new(obj: PyObjectWithType) -> Option<Self> {
        if obj.get_type_ptr() == &raw mut pyo3::ffi::PyByteArray_Type {
            Some(Self { ptr: obj.as_ptr() })
        } else {
            None
        }
    }
}

impl Serialize for ByteArray {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut critical_section = CriticalSection::new();
        critical_section.begin(self.ptr);
        let contents = unsafe { pybytearray_as_bytes(self.ptr) };
        serializer.serialize_bytes(contents)
    }
}
