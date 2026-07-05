// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use pyo3::ffi::*;
use pyo3::prelude::*;
use pyo3::types::PyBytes;

#[inline(always)]
pub fn pybytes_as_bytes<'a>(obj: Borrowed<'a, '_, PyBytes>) -> &'a [u8] {
    unsafe {
        let buffer = PyBytes_AS_STRING(obj.as_ptr()).cast::<u8>();
        let length = Py_SIZE(obj.as_ptr()) as usize;
        std::slice::from_raw_parts(buffer, length)
    }
}

#[repr(transparent)]
pub struct Buffer(Py_buffer);

impl Buffer {
    pub fn get<T>(obj: Borrowed<'_, '_, T>) -> Option<Self> {
        unsafe {
            let mut view: Py_buffer = std::mem::zeroed();
            if PyObject_GetBuffer(obj.as_ptr(), &mut view, PyBUF_CONTIG_RO) == -1 {
                None
            } else {
                Some(Self(view))
            }
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        let buffer = self.0.buf.cast::<u8>();
        let length = self.0.len as usize;
        unsafe { std::slice::from_raw_parts(buffer, length) }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe { PyBuffer_Release(&mut self.0) }
    }
}
