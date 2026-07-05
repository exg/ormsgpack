// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::fragment::PyFragment;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use serde::ser::{Serialize, Serializer};
use serde_bytes::Bytes;

#[repr(transparent)]
pub struct Fragment<'a, 'py> {
    obj: Borrowed<'a, 'py, PyAny>,
}

impl<'a, 'py> Fragment<'a, 'py> {
    pub fn new(obj: Borrowed<'a, 'py, PyAny>) -> Self {
        Fragment { obj: obj }
    }
}

impl Serialize for Fragment<'_, '_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let fragment = self.obj.as_ptr().cast::<PyFragment>();
        let data = unsafe {
            Borrowed::from_ptr(self.obj.py(), (*fragment).data).cast_unchecked::<PyBytes>()
        };

        serializer.serialize_newtype_struct("", Bytes::new(data.as_bytes()))
    }
}
