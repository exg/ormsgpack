// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ext;
use serde::ser::{Serialize, Serializer};
use serde_bytes::Bytes;

#[repr(transparent)]
pub struct Ext {
    ptr: *mut pyo3::ffi::PyObject,
}

impl Ext {
    pub fn new(ptr: *mut pyo3::ffi::PyObject) -> Self {
        Ext { ptr: ptr }
    }
}

impl Serialize for Ext {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        pyo3::Python::attach(|py| {
            let ext = unsafe { pyo3::Borrowed::from_ptr(py, self.ptr) }
                .cast::<ext::Ext>()
                .unwrap()
                .borrow();
            if unlikely!(!(0..=127).contains(&ext.tag)) {
                return Err(serde::ser::Error::custom("Extension type out of range"));
            }
            serializer.serialize_newtype_variant("", ext.tag as u32, "", Bytes::new(&ext.data))
        })
    }
}
