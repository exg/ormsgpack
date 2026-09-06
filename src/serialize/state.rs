// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use super::{dataclass, datetime, enum_, ext, fragment, numpy, pydantic, uuid};
use pyo3::ffi::*;

#[allow(non_snake_case)]
pub struct State {
    pub ext: ext::State,
    pub fragment: fragment::State,
    pub dataclass: dataclass::State,
    pub datetime: datetime::State,
    pub enum_: enum_::State,
    pub numpy: numpy::State,
    pub pydantic: pydantic::State,
    pub uuid: uuid::State,
    pub dict_str: *mut PyObject,
    pub slots_str: *mut PyObject,
    pub MsgpackEncodeError: *mut PyObject,
}

impl State {
    #[cold]
    pub fn new() -> Self {
        unsafe {
            Self {
                ext: ext::State::new(),
                fragment: fragment::State::new(),
                dataclass: dataclass::State::new(),
                datetime: datetime::State::new(),
                enum_: enum_::State::new(),
                numpy: numpy::State::new(),
                pydantic: pydantic::State::new(),
                uuid: uuid::State::new(),
                dict_str: PyUnicode_InternFromString(c"__dict__".as_ptr()),
                slots_str: PyUnicode_InternFromString(c"__slots__".as_ptr()),
                MsgpackEncodeError: Py_NewRef(PyExc_TypeError),
            }
        }
    }
}
