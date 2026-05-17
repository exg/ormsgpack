// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::{pybytearray_as_bytes, CriticalSection};
use crate::ffi::{BorrowedPyObject, PyObjectWithType};
use serde::ser::{Serialize, Serializer};

#[repr(transparent)]
pub struct ByteArray<'a> {
    obj: BorrowedPyObject<'a>,
}

impl<'a> ByteArray<'a> {
    #[inline]
    pub fn try_new(obj: PyObjectWithType<'a>) -> Option<Self> {
        if obj.get_type_ptr() == &raw mut pyo3::ffi::PyByteArray_Type {
            Some(Self {
                obj: obj.as_borrowed(),
            })
        } else {
            None
        }
    }
}

impl Serialize for ByteArray<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut critical_section = CriticalSection::new();
        critical_section.begin(self.obj.as_ptr());
        let contents = unsafe { pybytearray_as_bytes(self.obj.as_ptr()) };
        serializer.serialize_bytes(contents)
    }
}
