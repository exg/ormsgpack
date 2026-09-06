// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::Buffer;
use crate::ffi::PyObjectWithType;
use serde::ser::{Serialize, Serializer};

#[repr(transparent)]
pub struct MemoryView {
    ptr: *mut pyo3::ffi::PyObject,
}

impl MemoryView {
    #[inline]
    pub fn try_new(obj: PyObjectWithType) -> Option<Self> {
        if obj.get_type_ptr() == &raw mut pyo3::ffi::PyMemoryView_Type {
            Some(Self { ptr: obj.as_ptr() })
        } else {
            None
        }
    }
}

impl Serialize for MemoryView {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(buffer) = unsafe { Buffer::get(self.ptr) } {
            serializer.serialize_bytes(buffer.as_bytes())
        } else {
            Err(serde::ser::Error::custom(
                "Failed to get buffer from memoryview",
            ))
        }
    }
}
