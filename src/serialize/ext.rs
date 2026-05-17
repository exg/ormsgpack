// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ext::PyExt;
use crate::ffi::{pybytes_as_bytes, BorrowedPyObject, OwnedPyObject, PyObjectWithType};
use crate::util::unlikely;
use serde::ser::{Serialize, Serializer};
use serde_bytes::Bytes;

pub struct State {
    pub type_object: OwnedPyObject,
}

impl State {
    #[cold]
    pub fn new() -> Option<Self> {
        Some(Self {
            type_object: unsafe {
                OwnedPyObject::from_owned_ptr_or_opt(crate::ext::create_ext_type().cast())?
            },
        })
    }
}

#[repr(transparent)]
pub struct Ext<'a> {
    obj: BorrowedPyObject<'a>,
}

impl<'a> Ext<'a> {
    #[inline]
    pub fn try_new(obj: PyObjectWithType<'a>, state: &State) -> Option<Self> {
        if obj.get_type_ptr() == state.type_object.as_ptr().cast() {
            Some(Self {
                obj: obj.as_borrowed(),
            })
        } else {
            None
        }
    }
}

impl Serialize for Ext<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let ext = self.obj.as_ptr().cast::<PyExt>();
        let tag = unsafe { pyo3::ffi::PyLong_AsLongLong((*ext).tag) };
        if unlikely(!(0..=127).contains(&tag)) {
            return Err(serde::ser::Error::custom("Extension type out of range"));
        }
        let data = unsafe { pybytes_as_bytes((*ext).data) };

        serializer.serialize_newtype_variant("", tag as u32, "", Bytes::new(data))
    }
}
