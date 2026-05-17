// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::pybytes_as_bytes;
use crate::ffi::{BorrowedPyObject, PyObjectWithType};
use serde::ser::{Serialize, Serializer};

#[repr(transparent)]
pub struct Bytes<'a> {
    obj: BorrowedPyObject<'a>,
}

impl<'a> Bytes<'a> {
    #[inline]
    pub fn try_new(obj: PyObjectWithType<'a>) -> Option<Self> {
        if obj.get_type_ptr() == &raw mut pyo3::ffi::PyBytes_Type {
            Some(Self {
                obj: obj.as_borrowed(),
            })
        } else {
            None
        }
    }
}

impl Serialize for Bytes<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let contents = unsafe { pybytes_as_bytes(self.obj.as_ptr()) };
        serializer.serialize_bytes(contents)
    }
}
