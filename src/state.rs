// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::deserialize;
use crate::serialize;
use pyo3::prelude::*;
use pyo3::types::PyString;

#[repr(C)]
pub struct State {
    pub serialize: serialize::State,
    pub deserialize: deserialize::State,
    pub default_str: Py<PyString>,
    pub ext_hook_str: Py<PyString>,
    pub option_str: Py<PyString>,
}

impl State {
    #[cold]
    pub fn new(py: Python<'_>) -> PyResult<Self> {
        Ok(Self {
            serialize: serialize::State::new(py)?,
            deserialize: deserialize::State::new(py),
            default_str: PyString::intern(py, "default").unbind(),
            ext_hook_str: PyString::intern(py, "ext_hook").unbind(),
            option_str: PyString::intern(py, "option").unbind(),
        })
    }
}
