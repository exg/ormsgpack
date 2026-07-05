// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::get_type;
use crate::opt::Opt;
use crate::serialize::default::DefaultHook;
use crate::serialize::serializer::{DictKey, PyObject};
use crate::serialize::state::State as RootState;

use pyo3::prelude::*;
use pyo3::types::{PyString, PyType};
use serde::ser::{Serialize, Serializer};

pub struct State {
    enum_type: Py<PyAny>,
    value_str: Py<PyString>,
}

impl State {
    #[cold]
    pub fn new(py: Python<'_>) -> PyResult<Self> {
        Ok(Self {
            enum_type: py.import("enum")?.getattr("EnumMeta")?.unbind(),
            value_str: PyString::intern(py, "value").unbind(),
        })
    }
}

pub struct Enum<'a, 'py> {
    obj: Borrowed<'a, 'py, PyAny>,
    state: &'a RootState,
    opts: Opt,
    default: &'a DefaultHook<'a, 'py>,
}

impl<'a, 'py> Enum<'a, 'py> {
    #[inline]
    pub fn matches_exact_type(type_obj: Borrowed<'_, '_, PyType>, state: &State) -> bool {
        get_type(type_obj).as_type_ptr() == state.enum_type.as_ptr().cast()
    }

    pub fn new(
        obj: Borrowed<'a, 'py, PyAny>,
        state: &'a RootState,
        opts: Opt,
        default: &'a DefaultHook<'a, 'py>,
    ) -> Self {
        Self {
            obj,
            state,
            opts,
            default,
        }
    }
}

impl Serialize for Enum<'_, '_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = self
            .obj
            .getattr(self.state.enum_state.value_str.bind_borrowed(self.obj.py()))
            .map_err(serde::ser::Error::custom)?;
        PyObject::new(value.as_borrowed(), self.state, self.opts, self.default)
            .serialize(serializer)
    }
}

pub struct EnumDictKey<'a, 'py> {
    obj: Borrowed<'a, 'py, PyAny>,
    state: &'a RootState,
    opts: Opt,
}

impl<'a, 'py> EnumDictKey<'a, 'py> {
    pub fn new(obj: Borrowed<'a, 'py, PyAny>, state: &'a RootState, opts: Opt) -> Self {
        Self { obj, state, opts }
    }
}

impl Serialize for EnumDictKey<'_, '_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = self
            .obj
            .getattr(self.state.enum_state.value_str.bind_borrowed(self.obj.py()))
            .map_err(serde::ser::Error::custom)?;
        DictKey::new(value.as_borrowed(), self.state, self.opts).serialize(serializer)
    }
}
