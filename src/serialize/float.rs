// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::{BorrowedPyObject, PyObjectWithType};
use serde::ser::{Serialize, Serializer};

#[repr(transparent)]
pub struct Float<'a> {
    obj: BorrowedPyObject<'a>,
}

impl<'a> Float<'a> {
    #[inline]
    pub fn try_new(obj: PyObjectWithType<'a>) -> Option<Self> {
        if obj.get_type_ptr() == &raw mut pyo3::ffi::PyFloat_Type {
            Some(Self {
                obj: obj.as_borrowed(),
            })
        } else {
            None
        }
    }
}

impl Serialize for Float<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = unsafe { pyo3::ffi::PyFloat_AS_DOUBLE(self.obj.as_ptr()) };
        serializer.serialize_f64(value)
    }
}
