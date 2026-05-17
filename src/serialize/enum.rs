// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::{BorrowedPyObject, OwnedPyObject, PyObjectWithType};
use crate::opt::Opt;
use crate::serialize::default::DefaultHook;
use crate::serialize::serializer::{DictKey, PyObject as ObjectSerializer};
use crate::serialize::State as SerializeState;

use serde::ser::{Serialize, Serializer};

pub struct State {
    pub type_object: OwnedPyObject,
    pub value_str: OwnedPyObject,
}

impl State {
    #[cold]
    pub fn new() -> Option<Self> {
        let module = OwnedPyObject::try_import(c"enum")?;
        Some(Self {
            type_object: module.getattr_string(c"EnumMeta")?,
            value_str: OwnedPyObject::try_intern(c"value")?,
        })
    }
}

pub struct Enum<'a> {
    obj: BorrowedPyObject<'a>,
    state: &'a SerializeState,
    opts: Opt,
    default: &'a DefaultHook<'a>,
}

impl<'a> Enum<'a> {
    #[inline]
    pub fn try_new(
        obj: PyObjectWithType<'a>,
        state: &'a SerializeState,
        opts: Opt,
        default: &'a DefaultHook<'a>,
    ) -> Option<Self> {
        let ob_type = unsafe { pyo3::ffi::Py_TYPE(obj.get_type_ptr().cast()) };
        if ob_type == state.enum_.type_object.as_ptr().cast() {
            Some(Self {
                obj: obj.as_borrowed(),
                state,
                opts,
                default,
            })
        } else {
            None
        }
    }
}

impl Serialize for Enum<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = self
            .obj
            .getattr(self.state.enum_.value_str.as_borrowed())
            .unwrap();
        ObjectSerializer::new(value.as_borrowed(), self.state, self.opts, self.default)
            .serialize(serializer)
    }
}

pub struct EnumDictKey<'a> {
    obj: BorrowedPyObject<'a>,
    state: &'a SerializeState,
    opts: Opt,
}

impl<'a> EnumDictKey<'a> {
    #[inline]
    pub fn try_new(
        obj: PyObjectWithType<'a>,
        state: &'a SerializeState,
        opts: Opt,
    ) -> Option<Self> {
        let ob_type = unsafe { pyo3::ffi::Py_TYPE(obj.get_type_ptr().cast()) };
        if ob_type == state.enum_.type_object.as_ptr().cast() {
            Some(Self {
                obj: obj.as_borrowed(),
                state,
                opts,
            })
        } else {
            None
        }
    }
}

impl Serialize for EnumDictKey<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = self
            .obj
            .getattr(self.state.enum_.value_str.as_borrowed())
            .unwrap();
        DictKey::new(value.as_borrowed(), self.state, self.opts).serialize(serializer)
    }
}
