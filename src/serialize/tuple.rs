// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::*;
use crate::opt::*;
use crate::serialize::default::DefaultHook;
use crate::serialize::serializer::*;
use crate::serialize::State;

use serde::ser::{Serialize, SerializeSeq, Serializer};

pub struct Tuple<'a> {
    obj: BorrowedPyObject<'a>,
    state: &'a State,
    opts: Opt,
    default: &'a DefaultHook<'a>,
}

impl<'a> Tuple<'a> {
    #[inline]
    pub fn try_new(
        obj: PyObjectWithType<'a>,
        state: &'a State,
        opts: Opt,
        default: &'a DefaultHook<'a>,
    ) -> Option<Self> {
        if obj.get_type_ptr() == &raw mut pyo3::ffi::PyTuple_Type {
            Some(Self::new(obj.as_borrowed(), state, opts, default))
        } else {
            None
        }
    }

    fn new(
        obj: BorrowedPyObject<'a>,
        state: &'a State,
        opts: Opt,
        default: &'a DefaultHook<'a>,
    ) -> Self {
        Tuple {
            obj,
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
        let len = unsafe { pyo3::ffi::Py_SIZE(self.obj.as_ptr()) } as usize;
        let mut seq = serializer.serialize_seq(Some(len))?;
        for i in 0..len {
            let item = unsafe {
                BorrowedPyObject::from_ptr(pytuple_get_item(self.obj.as_ptr(), i as isize))
                    .unwrap_unchecked()
            };
            let value = PyObject::new(item, self.state, self.opts, self.default);
            seq.serialize_element(&value)?;
        }
        seq.end()
    }
}

pub struct TupleDictKey<'a> {
    obj: BorrowedPyObject<'a>,
    state: &'a State,
    opts: Opt,
}

impl<'a> TupleDictKey<'a> {
    #[inline]
    pub fn try_new(obj: PyObjectWithType<'a>, state: &'a State, opts: Opt) -> Option<Self> {
        if obj.get_type_ptr() == &raw mut pyo3::ffi::PyTuple_Type {
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

impl Serialize for TupleDictKey<'_> {
    #[inline(never)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let len = unsafe { pyo3::ffi::Py_SIZE(self.obj.as_ptr()) } as usize;
        let mut seq = serializer.serialize_seq(Some(len))?;
        for i in 0..len {
            let item = unsafe {
                BorrowedPyObject::from_ptr(pytuple_get_item(self.obj.as_ptr(), i as isize))
                    .unwrap_unchecked()
            };
            let value = DictKey::new(item, self.state, self.opts);
            seq.serialize_element(&value)?;
        }
        seq.end()
    }
}
