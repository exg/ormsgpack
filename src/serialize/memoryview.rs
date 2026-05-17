// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::{BorrowedPyObject, Buffer, PyObjectWithType};
use serde::ser::{Serialize, Serializer};

#[repr(transparent)]
pub struct MemoryView<'a> {
    obj: BorrowedPyObject<'a>,
}

impl<'a> MemoryView<'a> {
    #[inline]
    pub fn try_new(obj: PyObjectWithType<'a>) -> Option<Self> {
        if obj.get_type_ptr() == &raw mut pyo3::ffi::PyMemoryView_Type {
            Some(Self {
                obj: obj.as_borrowed(),
            })
        } else {
            None
        }
    }
}

impl Serialize for MemoryView<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(buffer) = unsafe { Buffer::get(self.obj.as_ptr()) } {
            serializer.serialize_bytes(buffer.as_bytes())
        } else {
            Err(serde::ser::Error::custom(
                "Failed to get buffer from memoryview",
            ))
        }
    }
}
