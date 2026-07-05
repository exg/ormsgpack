// SPDX-License-Identifier: (Apache-2.0 OR MIT)

mod bytes;
mod dict;
#[cfg_attr(any(PyPy, GraalPy), path = "base/mod.rs")]
#[cfg_attr(not(any(PyPy, GraalPy)), path = "cpython/mod.rs")]
mod impl_;
mod int;
mod unicode;

pub use bytes::*;
pub use dict::*;
pub use impl_::*;
pub use int::*;
pub use unicode::*;

use pyo3::ffi::*;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyType};
use pyo3::PyTypeInfo;
use std::marker::PhantomData;
use std::ptr::NonNull;

#[inline(always)]
pub fn get_type<'a, 'py, T>(obj: Borrowed<'a, 'py, T>) -> Borrowed<'a, 'py, PyType> {
    unsafe { Borrowed::from_ptr(obj.py(), Py_TYPE(obj.as_ptr()).cast()).cast_unchecked::<PyType>() }
}

#[inline(always)]
pub fn get_type_dict<'a, 'py>(obj: Borrowed<'a, 'py, PyType>) -> Option<Borrowed<'a, 'py, PyDict>> {
    unsafe {
        let tp_dict = (*obj.as_type_ptr()).tp_dict;
        if tp_dict.is_null() {
            None
        } else {
            Some(Borrowed::from_ptr(obj.py(), tp_dict).cast_unchecked::<PyDict>())
        }
    }
}

#[inline(always)]
pub fn cast_exact<'a, 'py, T: PyTypeInfo>(
    value: Borrowed<'a, 'py, PyAny>,
) -> Option<Borrowed<'a, 'py, T>> {
    if get_type(value).as_type_ptr() == T::type_object_raw(value.py()) {
        Some(unsafe { value.cast_unchecked() })
    } else {
        None
    }
}

#[inline(always)]
pub fn cast_into_exact<T: PyTypeInfo>(value: Bound<'_, PyAny>) -> Option<Bound<'_, T>> {
    if get_type(value.as_borrowed()).as_type_ptr() == T::type_object_raw(value.py()) {
        Some(unsafe { value.cast_into_unchecked() })
    } else {
        None
    }
}

#[derive(Clone, Copy)]
pub struct BorrowedWithType<'a, 'py> {
    obj: Borrowed<'a, 'py, PyAny>,
    type_obj: Borrowed<'a, 'py, PyType>,
}

impl<'a, 'py> BorrowedWithType<'a, 'py> {
    #[inline(always)]
    pub fn new(obj: Borrowed<'a, 'py, PyAny>) -> Self {
        Self {
            obj,
            type_obj: get_type(obj),
        }
    }

    pub fn py(self) -> Python<'py> {
        self.obj.py()
    }

    #[inline(always)]
    pub fn as_borrowed(self) -> Borrowed<'a, 'py, PyAny> {
        self.obj
    }

    #[inline(always)]
    pub fn cast_exact<T: PyTypeInfo>(self) -> Option<Borrowed<'a, 'py, T>> {
        if self.get_type_ptr() == T::type_object_raw(self.obj.py()) {
            Some(unsafe { self.obj.cast_unchecked() })
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn get_type(self) -> Borrowed<'a, 'py, PyType> {
        self.type_obj
    }

    #[inline(always)]
    pub fn get_type_ptr(self) -> *mut PyTypeObject {
        self.type_obj.as_type_ptr()
    }
}

/// Minimal `pyo3::Py` replacement for objects stored in per-interpreter
/// module state.
///
/// Owns one reference and decrements it immediately on drop, avoiding PyO3's
/// process-global deferred-reference pool.
#[repr(transparent)]
pub struct Py<T>(NonNull<PyObject>, PhantomData<T>);

impl<T> Py<T> {
    #[inline]
    pub unsafe fn from_bound(obj: Bound<'_, T>) -> Self {
        Self(NonNull::new_unchecked(obj.into_ptr()), PhantomData)
    }

    #[inline]
    pub fn as_ptr(&self) -> *mut PyObject {
        self.0.as_ptr()
    }

    #[inline]
    pub fn bind_borrowed<'a, 'py>(&'a self, py: Python<'py>) -> Borrowed<'a, 'py, T> {
        unsafe { Borrowed::from_ptr(py, self.0.as_ptr()).cast_unchecked() }
    }
}

impl<T> Drop for Py<T> {
    #[inline]
    fn drop(&mut self) {
        unsafe { Py_DECREF(self.0.as_ptr()) }
    }
}
