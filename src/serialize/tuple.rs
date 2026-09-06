// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::*;
use crate::opt::*;
use crate::serialize::default::DefaultHook;
use crate::serialize::serializer::*;
use crate::serialize::State;

use serde::ser::{Serialize, SerializeSeq, Serializer};

pub struct Tuple<'a> {
    ptr: *mut pyo3::ffi::PyObject,
    state: &'a State,
    opts: Opt,
    default: &'a DefaultHook,
}

impl<'a> Tuple<'a> {
    #[inline]
    pub fn try_new(
        obj: PyObjectWithType,
        state: &'a State,
        opts: Opt,
        default: &'a DefaultHook,
    ) -> Option<Self> {
        if obj.get_type_ptr() == &raw mut pyo3::ffi::PyTuple_Type {
            Some(Self::new(obj.as_ptr(), state, opts, default))
        } else {
            None
        }
    }

    fn new(
        ptr: *mut pyo3::ffi::PyObject,
        state: &'a State,
        opts: Opt,
        default: &'a DefaultHook,
    ) -> Self {
        Tuple {
            ptr: ptr,
            state: state,
            opts: opts,
            default: default,
        }
    }
}

impl Serialize for Tuple<'_> {
    #[inline(never)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let len = unsafe { pyo3::ffi::Py_SIZE(self.ptr) } as usize;
        let mut seq = serializer.serialize_seq(Some(len))?;
        for i in 0..len {
            let item = unsafe { pytuple_get_item(self.ptr, i as isize) };
            let value = PyObject::new(item, self.state, self.opts, self.default);
            seq.serialize_element(&value)?;
        }
        seq.end()
    }
}

pub struct TupleDictKey<'a> {
    ptr: *mut pyo3::ffi::PyObject,
    state: &'a State,
    opts: Opt,
}

impl<'a> TupleDictKey<'a> {
    #[inline]
    pub fn try_new(obj: PyObjectWithType, state: &'a State, opts: Opt) -> Option<Self> {
        if obj.get_type_ptr() == &raw mut pyo3::ffi::PyTuple_Type {
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

impl Serialize for TupleDictKey<'_> {
    #[inline(never)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let len = unsafe { pyo3::ffi::Py_SIZE(self.ptr) } as usize;
        let mut seq = serializer.serialize_seq(Some(len))?;
        for i in 0..len {
            let item = unsafe { pytuple_get_item(self.ptr, i as isize) };
            let value = DictKey::new(item, self.state, self.opts);
            seq.serialize_element(&value)?;
        }
        seq.end()
    }
}
