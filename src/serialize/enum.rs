// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::{pyobject_getattr, PyObjectWithType};
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
    #[inline]
    pub fn try_new(
        obj: PyObjectWithType,
        state: &'a SerializeState,
        opts: Opt,
        default: &'a DefaultHook,
    ) -> Option<Self> {
        if ob_type!(obj.get_type_ptr()) == state.enum_.type_object {
            Some(Self {
                ptr: obj.as_ptr(),
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
        let value = unsafe { pyobject_getattr(self.ptr, self.state.enum_.value_str).unwrap() };
        ObjectSerializer::new(value.as_ptr(), self.state, self.opts, self.default)
            .serialize(serializer)
    }
}

pub struct EnumDictKey<'a> {
    ptr: *mut PyObject,
    state: &'a SerializeState,
    opts: Opt,
}

impl<'a> EnumDictKey<'a> {
    #[inline]
    pub fn try_new(obj: PyObjectWithType, state: &'a SerializeState, opts: Opt) -> Option<Self> {
        if ob_type!(obj.get_type_ptr()) == state.enum_.type_object {
            Some(Self {
                ptr: obj.as_ptr(),
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
        let value = unsafe { pyobject_getattr(self.ptr, self.state.enum_.value_str).unwrap() };
        DictKey::new(value.as_ptr(), self.state, self.opts).serialize(serializer)
    }
}
