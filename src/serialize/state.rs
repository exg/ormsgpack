// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ext::create_ext_type;
use crate::fragment::create_fragment_type;

use super::{dataclass, datetime, enum_, numpy, pydantic, uuid};

use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyString, PyType};
use pyo3::PyTypeInfo;

#[allow(non_snake_case)]
#[repr(C)]
pub struct State {
    pub ext_type: Py<PyAny>,
    pub fragment_type: Py<PyAny>,
    pub dataclass_state: dataclass::State,
    pub datetime_state: datetime::State,
    pub enum_state: enum_::State,
    pub numpy_state: numpy::State,
    pub pydantic_state: pydantic::State,
    pub uuid_state: uuid::State,
    pub dict_str: Py<PyString>,
    pub slots_str: Py<PyString>,
    pub MsgpackEncodeError: Py<PyType>,
}

impl State {
    #[cold]
    pub fn new(py: Python<'_>) -> PyResult<Self> {
        Ok(Self {
            ext_type: create_ext_type(py)?.unbind(),
            fragment_type: create_fragment_type(py)?.unbind(),
            dataclass_state: dataclass::State::new(py)?,
            datetime_state: datetime::State::new(py),
            enum_state: enum_::State::new(py)?,
            numpy_state: numpy::State::new(py),
            pydantic_state: pydantic::State::new(py),
            uuid_state: uuid::State::new(py)?,
            dict_str: PyString::intern(py, "__dict__").unbind(),
            slots_str: PyString::intern(py, "__slots__").unbind(),
            MsgpackEncodeError: PyTypeError::type_object(py).unbind(),
        })
    }

    #[cold]
    pub fn error(&self, py: Python<'_>, message: &str) -> PyErr {
        let message = PyString::new(py, message).unbind();
        PyErr::from_type(self.MsgpackEncodeError.bind(py).clone(), message)
    }
}
