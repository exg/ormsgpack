// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::{pybytes_as_bytes, BorrowedPyObject, OwnedPyObject, PyObjectWithType};
use crate::fragment::PyFragment;
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
                OwnedPyObject::from_owned_ptr_or_opt(
                    crate::fragment::create_fragment_type().cast(),
                )?
            },
        })
    }
}

#[repr(transparent)]
pub struct Fragment<'a> {
    obj: BorrowedPyObject<'a>,
}

impl<'a> Fragment<'a> {
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

impl Serialize for Fragment<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let fragment = self.obj.as_ptr().cast::<PyFragment>();
        let data = unsafe { pybytes_as_bytes((*fragment).data) };

        serializer.serialize_newtype_struct("", Bytes::new(data))
    }
}
