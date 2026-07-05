// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::opt::*;
use crate::serialize::default::DefaultHook;
use crate::serialize::serializer::*;
use crate::serialize::state::State;

use pyo3::ffi::{PyType_HasFeature, Py_TPFLAGS_LIST_SUBCLASS};
use pyo3::prelude::*;
use pyo3::sync::critical_section::with_critical_section;
use pyo3::types::{PyList, PyType};
use serde::ser::{Serialize, SerializeSeq, Serializer};

pub struct List<'a, 'py> {
    obj: Borrowed<'a, 'py, PyList>,
    state: &'a State,
    opts: Opt,
    default: &'a DefaultHook<'a, 'py>,
}

impl<'a, 'py> List<'a, 'py> {
    #[inline]
    pub fn matches_type(type_obj: Borrowed<'_, '_, PyType>) -> bool {
        unsafe { PyType_HasFeature(type_obj.as_type_ptr(), Py_TPFLAGS_LIST_SUBCLASS) != 0 }
    }

    pub fn new(
        obj: Borrowed<'a, 'py, PyList>,
        state: &'a State,
        opts: Opt,
        default: &'a DefaultHook<'a, 'py>,
    ) -> Self {
        List {
            obj: obj,
            state: state,
            opts: opts,
            default: default,
        }
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
                let item = unsafe {
                    let item = pyo3::ffi::PyList_GET_ITEM(self.obj.as_ptr(), i as isize);
                    Borrowed::from_ptr(self.obj.py(), item)
                };
                let value = PyObject::new(item, self.state, self.opts, self.default);
                seq.serialize_element(&value)?;
            }
            seq.end()
        })
    }
}
