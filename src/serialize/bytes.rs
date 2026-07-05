// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use pyo3::prelude::*;
use pyo3::types::PyBytes;
use serde::ser::{Serialize, Serializer};

#[repr(transparent)]
pub struct Bytes<'a, 'py> {
    obj: Borrowed<'a, 'py, PyBytes>,
}

impl<'a, 'py> Bytes<'a, 'py> {
    pub fn new(obj: Borrowed<'a, 'py, PyBytes>) -> Self {
        Bytes { obj: obj }
    }
}

impl Serialize for Bytes<'_, '_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(self.obj.as_bytes())
    }
}
