// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::{BorrowedPyObject, PyObjectWithType};
use serde::ser::{Serialize, Serializer};

#[repr(transparent)]
pub struct Bool<'a> {
    obj: BorrowedPyObject<'a>,
}

impl<'a> Bool<'a> {
    #[inline]
    pub fn try_new(obj: PyObjectWithType<'a>) -> Option<Self> {
        if obj.get_type_ptr() == &raw mut pyo3::ffi::PyBool_Type {
            Some(Self {
                obj: obj.as_borrowed(),
            })
        } else {
            None
        }
    }
}

impl Serialize for Bool<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = unsafe { self.obj.as_ptr() == pyo3::ffi::Py_True() };
        serializer.serialize_bool(value)
    }
}
