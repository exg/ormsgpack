// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use super::{dataclass, datetime, enum_, ext, fragment, numpy, pydantic, uuid};
use crate::ffi::OwnedPyObject;
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
    pub dict_str: OwnedPyObject,
    pub slots_str: OwnedPyObject,
    pub MsgpackEncodeError: OwnedPyObject,
}

impl State {
    #[cold]
    pub fn new() -> Option<Self> {
        unsafe {
            Some(Self {
                ext: ext::State::new()?,
                fragment: fragment::State::new()?,
                dataclass: dataclass::State::new()?,
                datetime: datetime::State::new()?,
                enum_: enum_::State::new()?,
                numpy: numpy::State::new()?,
                pydantic: pydantic::State::new()?,
                uuid: uuid::State::new()?,
                dict_str: OwnedPyObject::try_intern(c"__dict__")?,
                slots_str: OwnedPyObject::try_intern(c"__slots__")?,
                MsgpackEncodeError: OwnedPyObject::from_borrowed_ptr(PyExc_TypeError)?,
            })
        }
    }
}
