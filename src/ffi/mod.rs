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
use std::ffi::CStr;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::ptr::NonNull;

#[cold]
pub unsafe fn set_python_error(exception: *mut PyObject, msg: &str) {
    let err_msg = PyUnicode_FromStringAndSize(msg.as_ptr().cast(), msg.len() as isize);
    if !err_msg.is_null() {
        PyErr_SetObject(exception, err_msg);
        Py_DECREF(err_msg);
    }
}

#[repr(transparent)]
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct BorrowedPyObject<'a>(NonNull<PyObject>, PhantomData<&'a PyObject>);

#[allow(dead_code)]
impl<'a> BorrowedPyObject<'a> {
    #[inline]
    pub unsafe fn from_ptr(ptr: *mut PyObject) -> Option<Self> {
        NonNull::new(ptr).map(|ptr| Self(ptr, PhantomData))
    }

    #[inline]
    pub fn as_ptr(self) -> *mut PyObject {
        self.0.as_ptr()
    }

    #[inline]
    pub fn getattr(self, name: BorrowedPyObject<'_>) -> Option<OwnedPyObject> {
        unsafe {
            let result = OwnedPyObject::from_owned_ptr_or_opt(PyObject_GetAttr(
                self.as_ptr(),
                name.as_ptr(),
            ));
            if result.is_none() {
                PyErr_Clear();
            }
            result
        }
    }

    #[inline]
    pub fn hasattr(self, name: BorrowedPyObject<'_>) -> bool {
        unsafe { PyObject_HasAttr(self.as_ptr(), name.as_ptr()) == 1 }
    }

    #[inline]
    pub fn call_one_arg(self, arg: BorrowedPyObject<'_>) -> Option<OwnedPyObject> {
        let result = unsafe {
            #[cfg(any(PyPy, GraalPy))]
            {
                PyObject_CallFunctionObjArgs(
                    self.as_ptr(),
                    arg.as_ptr(),
                    std::ptr::null_mut::<PyObject>(),
                )
            }
            #[cfg(not(any(PyPy, GraalPy)))]
            {
                PyObject_CallOneArg(self.as_ptr(), arg.as_ptr())
            }
        };
        unsafe { OwnedPyObject::from_owned_ptr_or_opt(result) }
    }

    #[inline]
    pub fn call_two_args(
        self,
        arg1: BorrowedPyObject<'_>,
        arg2: BorrowedPyObject<'_>,
    ) -> Option<OwnedPyObject> {
        unsafe {
            OwnedPyObject::from_owned_ptr_or_opt(PyObject_CallFunctionObjArgs(
                self.as_ptr(),
                arg1.as_ptr(),
                arg2.as_ptr(),
                std::ptr::null_mut::<PyObject>(),
            ))
        }
    }

    #[inline]
    pub fn call_method_no_args(self, name: BorrowedPyObject<'_>) -> Option<OwnedPyObject> {
        let result = unsafe {
            #[cfg(any(PyPy, GraalPy))]
            {
                PyObject_CallMethodObjArgs(
                    self.as_ptr(),
                    name.as_ptr(),
                    std::ptr::null_mut::<PyObject>(),
                )
            }
            #[cfg(not(any(PyPy, GraalPy)))]
            {
                PyObject_CallMethodNoArgs(self.as_ptr(), name.as_ptr())
            }
        };
        unsafe { OwnedPyObject::from_owned_ptr_or_opt(result) }
    }

    #[inline]
    pub fn call_method_one_arg(
        self,
        name: BorrowedPyObject<'_>,
        arg: BorrowedPyObject<'_>,
    ) -> Option<OwnedPyObject> {
        let result = unsafe {
            #[cfg(any(PyPy, GraalPy))]
            {
                PyObject_CallMethodObjArgs(
                    self.as_ptr(),
                    name.as_ptr(),
                    arg.as_ptr(),
                    std::ptr::null_mut::<PyObject>(),
                )
            }
            #[cfg(not(any(PyPy, GraalPy)))]
            {
                PyObject_CallMethodOneArg(self.as_ptr(), name.as_ptr(), arg.as_ptr())
            }
        };
        unsafe { OwnedPyObject::from_owned_ptr_or_opt(result) }
    }
}

#[repr(transparent)]
#[allow(dead_code)]
pub struct OwnedPyObject(NonNull<PyObject>);

#[allow(dead_code)]
impl OwnedPyObject {
    #[inline]
    #[track_caller]
    pub unsafe fn from_owned_ptr(ptr: *mut PyObject) -> Self {
        Self(NonNull::new(ptr).expect("PyObject pointer is null"))
    }

    #[inline]
    pub unsafe fn from_owned_ptr_or_opt(ptr: *mut PyObject) -> Option<Self> {
        NonNull::new(ptr).map(Self)
    }

    #[inline]
    pub unsafe fn from_borrowed_ptr(ptr: *mut PyObject) -> Option<Self> {
        let ptr = NonNull::new(ptr)?;
        Py_INCREF(ptr.as_ptr());
        Some(Self(ptr))
    }

    #[inline]
    pub fn getattr(&self, name: BorrowedPyObject<'_>) -> Option<Self> {
        self.as_borrowed().getattr(name)
    }

    #[inline]
    pub fn hasattr(&self, name: BorrowedPyObject<'_>) -> bool {
        self.as_borrowed().hasattr(name)
    }

    #[inline]
    pub fn call_one_arg(&self, arg: BorrowedPyObject<'_>) -> Option<Self> {
        self.as_borrowed().call_one_arg(arg)
    }

    #[inline]
    pub fn call_two_args(
        &self,
        arg1: BorrowedPyObject<'_>,
        arg2: BorrowedPyObject<'_>,
    ) -> Option<Self> {
        self.as_borrowed().call_two_args(arg1, arg2)
    }

    #[inline]
    pub fn call_method_no_args(&self, name: BorrowedPyObject<'_>) -> Option<Self> {
        self.as_borrowed().call_method_no_args(name)
    }

    #[inline]
    pub fn call_method_one_arg(
        &self,
        name: BorrowedPyObject<'_>,
        arg: BorrowedPyObject<'_>,
    ) -> Option<Self> {
        self.as_borrowed().call_method_one_arg(name, arg)
    }

    #[inline]
    pub fn getattr_string(&self, name: &CStr) -> Option<Self> {
        unsafe { Self::from_owned_ptr_or_opt(PyObject_GetAttrString(self.as_ptr(), name.as_ptr())) }
    }

    #[inline]
    pub fn try_import(name: &CStr) -> Option<Self> {
        unsafe { Self::from_owned_ptr_or_opt(PyImport_ImportModule(name.as_ptr())) }
    }

    #[inline]
    pub fn try_intern(value: &CStr) -> Option<Self> {
        unsafe { Self::from_owned_ptr_or_opt(PyUnicode_InternFromString(value.as_ptr())) }
    }

    #[inline]
    pub fn as_borrowed(&self) -> BorrowedPyObject<'_> {
        BorrowedPyObject(self.0, PhantomData)
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
        unsafe { Py_INCREF(self.0.as_ptr()) }
        Self(self.0)
    }
}

impl Drop for OwnedPyObject {
    #[inline]
    fn drop(&mut self) {
        unsafe { Py_DECREF(self.0.as_ptr()) }
    }
}

#[derive(Clone, Copy)]
pub struct PyObjectWithType<'a> {
    obj: BorrowedPyObject<'a>,
    type_ptr: *mut PyTypeObject,
}

impl<'a> PyObjectWithType<'a> {
    #[inline(always)]
    pub fn new(obj: BorrowedPyObject<'a>) -> Self {
        let ob_type = unsafe { Py_TYPE(obj.as_ptr()) };
        Self {
            obj,
            type_ptr: ob_type,
        }
    }

    #[inline(always)]
    pub fn as_borrowed(self) -> BorrowedPyObject<'a> {
        self.obj
    }

    #[inline(always)]
    pub fn as_ptr(self) -> *mut PyObject {
        self.obj.as_ptr()
    }

    #[inline(always)]
    pub fn get_type_ptr(self) -> *mut PyTypeObject {
        self.type_ptr
    }
}

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

pub struct PyDictIter<'a> {
    op: BorrowedPyObject<'a>,
    pos: isize,
}

impl<'a> PyDictIter<'a> {
    #[inline]
    pub fn from_pyobject(op: BorrowedPyObject<'a>) -> Self {
        PyDictIter { op: op, pos: 0 }
    }
}

impl<'a> Iterator for PyDictIter<'a> {
    type Item = (BorrowedPyObject<'a>, BorrowedPyObject<'a>);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let mut key: *mut PyObject = std::ptr::null_mut();
        let mut value: *mut PyObject = std::ptr::null_mut();
        unsafe {
            if PyDict_Next(self.op.as_ptr(), &mut self.pos, &mut key, &mut value) == 1 {
                Some((
                    BorrowedPyObject(NonNull::new_unchecked(key), PhantomData),
                    BorrowedPyObject(NonNull::new_unchecked(value), PhantomData),
                ))
            } else {
                None
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = unsafe { pydict_size(self.op.as_ptr()) } as usize;
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

#[inline]
pub fn pybytes_new(bytes: &[u8]) -> OwnedPyObject {
    let ptr = bytes.as_ptr().cast();
    let len = bytes.len() as pyo3::ffi::Py_ssize_t;
    unsafe {
        let ptr = pyo3::ffi::PyBytes_FromStringAndSize(ptr, len);
        OwnedPyObject::from_owned_ptr(ptr)
    }
}
