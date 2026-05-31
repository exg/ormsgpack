// SPDX-License-Identifier: (Apache-2.0 OR MIT)

mod critical_section;
#[cfg_attr(any(PyPy, GraalPy), path = "base/mod.rs")]
#[cfg_attr(not(any(PyPy, GraalPy)), path = "cpython/mod.rs")]
mod impl_;
mod int;
mod unicode;

pub use critical_section::*;
pub use impl_::*;
pub use int::*;
pub use unicode::*;

use pyo3::ffi::*;
use std::mem::ManuallyDrop;
use std::ptr::NonNull;
#[cfg(Py_GIL_DISABLED)]
use std::sync::atomic::Ordering::Relaxed;

#[inline(always)]
pub unsafe fn pybytes_as_bytes(op: *mut PyObject) -> &'static [u8] {
    let buffer = pybytes_as_mut_u8(op);
    let length = Py_SIZE(op) as usize;
    std::slice::from_raw_parts(buffer, length)
}

#[inline(always)]
pub unsafe fn pybytearray_as_bytes(op: *mut PyObject) -> &'static [u8] {
    let buffer = PyByteArray_AsString(op).cast::<u8>();
    let length = PyByteArray_Size(op) as usize;
    std::slice::from_raw_parts(buffer, length)
}

pub struct PyDictIter {
    op: *mut PyObject,
    pos: isize,
}

impl PyDictIter {
    #[inline]
    pub fn from_pyobject(op: *mut PyObject) -> Self {
        PyDictIter { op: op, pos: 0 }
    }
}

impl Iterator for PyDictIter {
    type Item = (OwnedPyObject, OwnedPyObject);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let mut key: *mut PyObject = std::ptr::null_mut();
        let mut value: *mut PyObject = std::ptr::null_mut();
        unsafe {
            if PyDict_Next(self.op, &mut self.pos, &mut key, &mut value) == 1 {
                Some((
                    OwnedPyObject::from_borrowed_ptr(key),
                    OwnedPyObject::from_borrowed_ptr(value),
                ))
            } else {
                None
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = unsafe { pydict_size(self.op) } as usize;
        (len, Some(len))
    }
}

pub struct Buffer {
    view: Py_buffer,
}

impl Buffer {
    pub unsafe fn get(obj: *mut PyObject) -> Option<Self> {
        let mut view: Py_buffer = std::mem::zeroed();
        if PyObject_GetBuffer(obj, &mut view, PyBUF_CONTIG_RO) == -1 {
            return None;
        }
        Some(Self { view })
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

#[repr(transparent)]
pub struct OwnedPyObject(NonNull<PyObject>);

#[cfg(Py_GIL_DISABLED)]
#[inline(always)]
unsafe fn pyobject_is_immortal(op: *mut PyObject) -> bool {
    (*op).ob_ref_local.load(Relaxed) == u32::MAX
}

#[cfg(not(Py_GIL_DISABLED))]
#[inline(always)]
unsafe fn pyobject_is_immortal(_op: *mut PyObject) -> bool {
    false
}

impl OwnedPyObject {
    #[inline]
    pub fn from_non_null(ptr: NonNull<PyObject>) -> Self {
        Self(ptr)
    }

    #[inline]
    pub unsafe fn from_borrowed_ptr(ptr: *mut PyObject) -> Self {
        if !pyobject_is_immortal(ptr) {
            Py_INCREF(ptr);
        }
        Self(NonNull::new_unchecked(ptr))
    }

    #[inline]
    pub unsafe fn from_ptr(ptr: *mut PyObject) -> Self {
        Self(NonNull::new_unchecked(ptr))
    }

    #[inline]
    pub fn as_ptr(&self) -> *mut PyObject {
        self.0.as_ptr()
    }

    #[inline]
    pub fn into_ptr(self) -> *mut PyObject {
        ManuallyDrop::new(self).0.as_ptr()
    }
}

impl Clone for OwnedPyObject {
    #[inline]
    fn clone(&self) -> Self {
        unsafe {
            if !pyobject_is_immortal(self.0.as_ptr()) {
                Py_INCREF(self.0.as_ptr())
            }
        }
        Self(self.0)
    }
}

impl Drop for OwnedPyObject {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            if !pyobject_is_immortal(self.0.as_ptr()) {
                Py_DECREF(self.0.as_ptr())
            }
        }
    }
}

#[inline]
pub unsafe fn pyobject_getattr(op: *mut PyObject, name: *mut PyObject) -> Option<OwnedPyObject> {
    let ptr = PyObject_GetAttr(op, name);
    if let Some(ptr) = NonNull::new(ptr) {
        Some(OwnedPyObject::from_non_null(ptr))
    } else {
        PyErr_Clear();
        None
    }
}

#[inline]
pub fn pybytes_new(bytes: &[u8]) -> OwnedPyObject {
    let ptr = bytes.as_ptr().cast();
    let len = bytes.len() as pyo3::ffi::Py_ssize_t;
    unsafe {
        let ptr = pyo3::ffi::PyBytes_FromStringAndSize(ptr, len);
        OwnedPyObject::from_ptr(ptr)
    }
}
