// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ext::PyExt;
use crate::ffi::pybytes_as_bytes;
use crate::ffi::PyObjectWithType;
use crate::util::unlikely;
use serde::ser::{Serialize, Serializer};
use serde_bytes::Bytes;

pub struct State {
    pub type_object: *mut pyo3::ffi::PyTypeObject,
}

impl State {
    #[cold]
    pub fn new() -> Self {
        Self {
            type_object: unsafe { crate::ext::create_ext_type() },
        }
    }
}

#[repr(transparent)]
pub struct Ext {
    ptr: *mut pyo3::ffi::PyObject,
}

impl Ext {
    #[inline]
    pub fn try_new(obj: PyObjectWithType, state: &State) -> Option<Self> {
        if obj.get_type_ptr() == state.type_object {
            Some(Self { ptr: obj.as_ptr() })
        } else {
            None
        }
    }
}

impl Serialize for Ext {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let ext = self.ptr.cast::<PyExt>();
        let tag = unsafe { pyo3::ffi::PyLong_AsLongLong((*ext).tag) };
        if unlikely(!(0..=127).contains(&tag)) {
            return Err(serde::ser::Error::custom("Extension type out of range"));
        }
        let data = unsafe { pybytes_as_bytes((*ext).data) };

        serializer.serialize_newtype_variant("", tag as u32, "", Bytes::new(data))
    }
}
