// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::opt::Opt;
use crate::serialize::default::DefaultHook;
use crate::serialize::serializer::{DictKey, PyObject as ObjectSerializer};
use crate::serialize::State as SerializeState;

use pyo3::ffi::*;
use serde::ser::{Serialize, Serializer};

pub struct State {
    pub type_object: *mut PyTypeObject,
    pub value_str: *mut PyObject,
}

impl State {
    #[cold]
    pub fn new() -> Self {
        unsafe {
            let module = PyImport_ImportModule(c"enum".as_ptr());
            let type_object = PyObject_GetAttrString(module, c"EnumMeta".as_ptr()).cast();
            Py_DECREF(module);
            Self {
                type_object,
                value_str: PyUnicode_InternFromString(c"value".as_ptr()),
            }
        }
    }
}

pub struct Enum<'a> {
    ptr: *mut PyObject,
    state: &'a SerializeState,
    opts: Opt,
    default: &'a DefaultHook,
}

impl<'a> Enum<'a> {
    pub fn new(
        ptr: *mut PyObject,
        state: &'a SerializeState,
        opts: Opt,
        default: &'a DefaultHook,
    ) -> Self {
        Self {
            ptr: ptr,
            state: state,
            opts: opts,
            default: default,
        }
    }
}

impl Serialize for Enum<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = unsafe { PyObject_GetAttr(self.ptr, self.state.enum_.value_str) };
        let result =
            ObjectSerializer::new(value, self.state, self.opts, self.default).serialize(serializer);
        unsafe { Py_DECREF(value) };
        result
    }
}

pub struct EnumDictKey<'a> {
    ptr: *mut PyObject,
    state: &'a SerializeState,
    opts: Opt,
}

impl<'a> EnumDictKey<'a> {
    pub fn new(ptr: *mut PyObject, state: &'a SerializeState, opts: Opt) -> Self {
        Self {
            ptr: ptr,
            state: state,
            opts: opts,
        }
    }
}

impl Serialize for EnumDictKey<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = unsafe { PyObject_GetAttr(self.ptr, self.state.enum_.value_str) };
        let result = DictKey::new(value, self.state, self.opts).serialize(serializer);
        unsafe { Py_DECREF(value) };
        result
    }
}
