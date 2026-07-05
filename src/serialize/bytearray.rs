// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use pyo3::prelude::*;
use pyo3::sync::critical_section::with_critical_section;
use pyo3::types::PyByteArray;
use serde::ser::{Serialize, Serializer};

#[repr(transparent)]
pub struct ByteArray<'a, 'py> {
    obj: Borrowed<'a, 'py, PyByteArray>,
}

impl<'a, 'py> ByteArray<'a, 'py> {
    pub fn new(obj: Borrowed<'a, 'py, PyByteArray>) -> Self {
        ByteArray { obj: obj }
    }
}

impl Serialize for ByteArray<'_, '_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        with_critical_section(&self.obj, || {
            let contents = unsafe { self.obj.as_bytes() };
            serializer.serialize_bytes(contents)
        })
    }
}
