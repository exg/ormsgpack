// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::exc::OBJECT_MODIFIED_DURING_ITERATION;
use crate::ffi::{pylist_get_item, BorrowedWithType};
use crate::serialize::serializer::*;
use crate::serialize::Context;

use pyo3::prelude::*;
use pyo3::sync::critical_section::with_critical_section;
use pyo3::types::PyList;
use serde::ser::{Serialize, SerializeSeq, Serializer};

pub struct List<'a, 'py> {
    obj: Borrowed<'a, 'py, PyList>,
    context: Context<'a, 'py>,
}

impl<'a, 'py> List<'a, 'py> {
    #[inline]
    pub fn try_new_exact(
        obj: BorrowedWithType<'a, 'py>,
        context: Context<'a, 'py>,
    ) -> Option<Self> {
        Some(Self {
            obj: obj.cast_exact::<PyList>()?,
            context: context,
        })
    }

    #[inline]
    pub fn try_new(obj: BorrowedWithType<'a, 'py>, context: Context<'a, 'py>) -> Option<Self> {
        Some(Self {
            obj: obj.cast::<PyList>()?,
            context: context,
        })
    }
}

impl Serialize for List<'_, '_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        with_critical_section(&self.obj, || {
            let len = self.obj.len();
            let mut seq = serializer.serialize_seq(Some(len))?;
            for i in 0..len {
                let Some(item) = pylist_get_item(self.obj, i) else {
                    return Err(serde::ser::Error::custom(OBJECT_MODIFIED_DURING_ITERATION));
                };
                let value = PyObject::new(item.as_borrowed(), self.context);
                seq.serialize_element(&value)?;
            }
            seq.end()
        })
    }
}
