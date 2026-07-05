// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ext::PyExt;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyInt};
use serde::ser::{Serialize, Serializer};
use serde_bytes::Bytes;

#[repr(transparent)]
pub struct Ext<'a, 'py> {
    obj: Borrowed<'a, 'py, PyAny>,
}

impl<'a, 'py> Ext<'a, 'py> {
    pub fn new(obj: Borrowed<'a, 'py, PyAny>) -> Self {
        Ext { obj: obj }
    }
}

impl Serialize for Ext<'_, '_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let ext = self.obj.as_ptr().cast::<PyExt>();
        let tag = match unsafe {
            Borrowed::from_ptr(self.obj.py(), (*ext).tag).cast_unchecked::<PyInt>()
        }
        .extract::<u32>()
        {
            Ok(tag @ 0..=127) => tag,
            _ => return Err(serde::ser::Error::custom("Extension type out of range")),
        };
        let data =
            unsafe { Borrowed::from_ptr(self.obj.py(), (*ext).data).cast_unchecked::<PyBytes>() };

        serializer.serialize_newtype_variant("", tag, "", Bytes::new(data.as_bytes()))
    }
}
