// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::pybytes_as_bytes;
use crate::ffi::PyObjectWithType;
use crate::fragment::PyFragment;
use serde::ser::{Serialize, Serializer};
use serde_bytes::Bytes;

pub struct State {
    pub type_object: *mut pyo3::ffi::PyTypeObject,
}

impl State {
    #[cold]
    pub fn new() -> Self {
        Self {
            type_object: unsafe { crate::fragment::create_fragment_type() },
        }
    }
}

#[repr(transparent)]
pub struct Fragment {
    ptr: *mut pyo3::ffi::PyObject,
}

impl Fragment {
    #[inline]
    pub fn try_new(obj: PyObjectWithType, state: &State) -> Option<Self> {
        if obj.get_type_ptr() == state.type_object {
            Some(Self { ptr: obj.as_ptr() })
        } else {
            None
        }
    }
}

impl Serialize for Fragment {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let fragment = self.ptr.cast::<PyFragment>();
        let data = unsafe { pybytes_as_bytes((*fragment).data) };

        serializer.serialize_newtype_struct("", Bytes::new(data))
    }
}
