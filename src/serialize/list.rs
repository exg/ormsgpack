// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::{CriticalSection, OwnedPyObject};
use crate::opt::*;
use crate::serialize::default::DefaultHook;
use crate::serialize::serializer::*;
use crate::state::State;

use serde::ser::{Serialize, SerializeSeq, Serializer};

pub struct List<'a> {
    ptr: *mut pyo3::ffi::PyObject,
    state: *mut State,
    opts: Opt,
    default: &'a DefaultHook,
}

impl<'a> List<'a> {
    pub fn new(
        ptr: *mut pyo3::ffi::PyObject,
        state: *mut State,
        opts: Opt,
        default: &'a DefaultHook,
    ) -> Self {
        List {
            ptr: ptr,
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
        critical_section.begin(self.ptr);
        let len = unsafe { pyo3::ffi::PyList_GET_SIZE(self.ptr) } as usize;
        let mut seq = serializer.serialize_seq(Some(len))?;
        for i in 0..len {
            let item = unsafe {
                let item_ptr = pyo3::ffi::PyList_GetItem(self.ptr, i as isize);
                if item_ptr.is_null() {
                    return Err(serde::ser::Error::custom(
                        "Object modified during iteration",
                    ));
                }
                OwnedPyObject::from_borrowed_ptr(item_ptr)
            };
            let value = PyObject::new(item.as_ptr(), self.state, self.opts, self.default);
            seq.serialize_element(&value)?;
        }
        seq.end()
    }
}
