// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::{BorrowedPyObject, CriticalSection, PyObjectWithType};
use crate::opt::*;
use crate::serialize::default::DefaultHook;
use crate::serialize::serializer::*;
use crate::serialize::State;

use serde::ser::{Serialize, SerializeSeq, Serializer};

pub struct List<'a> {
    obj: BorrowedPyObject<'a>,
    state: &'a State,
    opts: Opt,
    default: &'a DefaultHook<'a>,
}

impl<'a> List<'a> {
    #[inline]
    pub fn try_new_exact(
        obj: PyObjectWithType<'a>,
        state: &'a State,
        opts: Opt,
        default: &'a DefaultHook<'a>,
    ) -> Option<Self> {
        if obj.get_type_ptr() == &raw mut pyo3::ffi::PyList_Type {
            Some(Self::new(obj.as_borrowed(), state, opts, default))
        } else {
            None
        }
    }

    #[inline]
    pub fn try_new(
        obj: PyObjectWithType<'a>,
        state: &'a State,
        opts: Opt,
        default: &'a DefaultHook<'a>,
    ) -> Option<Self> {
        if unsafe {
            pyo3::ffi::PyType_HasFeature(obj.get_type_ptr(), pyo3::ffi::Py_TPFLAGS_LIST_SUBCLASS)
                != 0
        } {
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
        List {
            obj,
            state: state,
            opts: opts,
            default: default,
        }
    }
}

impl Serialize for List<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut critical_section = CriticalSection::new();
        critical_section.begin(self.obj.as_ptr());
        let len = unsafe { pyo3::ffi::PyList_GET_SIZE(self.obj.as_ptr()) } as usize;
        let mut seq = serializer.serialize_seq(Some(len))?;
        for i in 0..len {
            let item = unsafe {
                BorrowedPyObject::from_ptr(pyo3::ffi::PyList_GET_ITEM(
                    self.obj.as_ptr(),
                    i as isize,
                ))
                .unwrap_unchecked()
            };
            let value = PyObject::new(item, self.state, self.opts, self.default);
            seq.serialize_element(&value)?;
        }
        seq.end()
    }
}
