// SPDX-License-Identifier: (Apache-2.0 OR MIT)

#[cfg_attr(any(PyPy, GraalPy), path = "base/mod.rs")]
#[cfg_attr(not(any(PyPy, GraalPy)), path = "cpython/mod.rs")]
mod impl_;
mod int;
mod unicode;

pub use impl_::*;
pub use int::*;
pub use unicode::*;

use pyo3::ffi::*;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyType};
use pyo3::PyTypeInfo;

#[inline(always)]
pub fn pybytes_as_bytes<'a>(obj: Borrowed<'a, '_, PyBytes>) -> &'a [u8] {
    unsafe {
        let buffer = PyBytes_AS_STRING(obj.as_ptr()).cast::<u8>();
        let length = Py_SIZE(obj.as_ptr()) as usize;
        std::slice::from_raw_parts(buffer, length)
    }
}

pub struct PyDictIter<'a, 'py> {
    obj: Borrowed<'a, 'py, PyDict>,
    pos: isize,
}

impl<'a, 'py> PyDictIter<'a, 'py> {
    #[inline]
    pub fn from_pyobject(obj: Borrowed<'a, 'py, PyDict>) -> Self {
        PyDictIter { obj: obj, pos: 0 }
    }
}

impl<'a, 'py> Iterator for PyDictIter<'a, 'py> {
    type Item = (Borrowed<'a, 'py, PyAny>, Borrowed<'a, 'py, PyAny>);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let mut key: *mut PyObject = std::ptr::null_mut();
        let mut value: *mut PyObject = std::ptr::null_mut();
        unsafe {
            if PyDict_Next(self.obj.as_ptr(), &mut self.pos, &mut key, &mut value) == 1 {
                Some((
                    Borrowed::from_ptr(self.obj.py(), key),
                    Borrowed::from_ptr(self.obj.py(), value),
                ))
            } else {
                None
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.obj.len();
        (len, Some(len))
    }
}

pub struct Buffer {
    view: Py_buffer,
}

impl Buffer {
    pub fn get<T>(obj: Borrowed<'_, '_, T>) -> Option<Self> {
        unsafe {
            let mut view: Py_buffer = std::mem::zeroed();
            if PyObject_GetBuffer(obj.as_ptr(), &mut view, PyBUF_CONTIG_RO) == -1 {
                return None;
            }
            Some(Self { view })
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        let buffer = self.view.buf.cast::<u8>();
        let length = self.view.len as usize;
        unsafe { std::slice::from_raw_parts(buffer, length) }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe { PyBuffer_Release(&mut self.view) }
    }
}

#[inline(always)]
pub fn get_type<'a, 'py, T>(obj: Borrowed<'a, 'py, T>) -> Borrowed<'a, 'py, PyType> {
    unsafe {
        Borrowed::from_ptr(obj.py(), (*obj.as_ptr()).ob_type.cast()).cast_unchecked::<PyType>()
    }
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

    #[inline(always)]
    pub fn as_borrowed(self) -> Borrowed<'a, 'py, PyAny> {
        self.obj
    }

    #[inline(always)]
    pub fn get_type(self) -> Borrowed<'a, 'py, PyType> {
        self.type_obj
    }

    #[inline(always)]
    pub fn get_type_ptr(self) -> *mut PyTypeObject {
        self.type_obj.as_type_ptr()
    }

    #[inline(always)]
    pub fn cast_exact<T: PyTypeInfo>(self) -> Option<Borrowed<'a, 'py, T>> {
        if self.get_type_ptr() == T::type_object_raw(self.obj.py()) {
            Some(unsafe { self.obj.cast_unchecked() })
        } else {
            None
        }
    }
}
