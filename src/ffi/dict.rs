// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::PyCollectionItem;
use pyo3::ffi::*;
use pyo3::prelude::*;
use pyo3::types::PyDict;

pub struct PyDictIter<'a, 'py> {
    obj: Borrowed<'a, 'py, PyDict>,
    pos: isize,
}

impl<'a, 'py> PyDictIter<'a, 'py> {
    #[inline]
    pub fn new(obj: Borrowed<'a, 'py, PyDict>) -> Self {
        Self { obj: obj, pos: 0 }
    }
}

impl<'a, 'py> Iterator for PyDictIter<'a, 'py> {
    type Item = (
        PyCollectionItem<'a, 'py, PyAny>,
        PyCollectionItem<'a, 'py, PyAny>,
    );

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let mut key: *mut PyObject = std::ptr::null_mut();
        let mut value: *mut PyObject = std::ptr::null_mut();
        unsafe {
            if PyDict_Next(self.obj.as_ptr(), &mut self.pos, &mut key, &mut value) == 1 {
                Some((
                    PyCollectionItem::from_borrowed(Borrowed::from_ptr(self.obj.py(), key)),
                    PyCollectionItem::from_borrowed(Borrowed::from_ptr(self.obj.py(), value)),
                ))
            } else {
                None
            }
        }
    }
}
