// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::OwnedPyObject;
use pyo3::ffi::*;
use std::ptr::NonNull;

mod int;
mod unicode;
#[cfg(unicode_state)]
mod unicode_state;

pub use unicode::*;

#[inline(always)]
pub unsafe fn pybytes_as_mut_u8(op: *mut PyObject) -> *mut u8 {
    (*op.cast::<PyBytesObject>())
        .ob_sval
        .as_mut_ptr()
        .cast::<u8>()
}

#[inline(always)]
pub unsafe fn pydict_size(mp: *mut PyObject) -> Py_ssize_t {
    Py_SIZE(mp)
}

#[inline(always)]
pub unsafe fn pyobject_call_one_arg(
    func: *mut PyObject,
    arg: *mut PyObject,
) -> Option<OwnedPyObject> {
    NonNull::new(PyObject_CallOneArg(func, arg)).map(OwnedPyObject::from_non_null)
}

#[inline(always)]
pub unsafe fn pyobject_call_method_no_args(
    self_: *mut PyObject,
    name: *mut PyObject,
) -> Option<OwnedPyObject> {
    NonNull::new(PyObject_CallMethodNoArgs(self_, name)).map(OwnedPyObject::from_non_null)
}

#[inline(always)]
pub unsafe fn pyobject_call_method_one_arg(
    self_: *mut PyObject,
    name: *mut PyObject,
    arg: *mut PyObject,
) -> Option<OwnedPyObject> {
    NonNull::new(PyObject_CallMethodOneArg(self_, name, arg)).map(OwnedPyObject::from_non_null)
}

#[inline(always)]
pub unsafe fn pytuple_get_item(op: *mut PyObject, i: Py_ssize_t) -> *mut PyObject {
    PyTuple_GET_ITEM(op, i)
}

#[inline(always)]
pub unsafe fn pytuple_set_item(op: *mut PyObject, i: Py_ssize_t, v: *mut PyObject) {
    PyTuple_SET_ITEM(op, i, v)
}
