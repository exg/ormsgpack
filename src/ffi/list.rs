// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::PyCollectionItem;
use pyo3::ffi::PyList_GET_ITEM;
use pyo3::prelude::*;
use pyo3::types::PyList;

#[inline]
pub fn pylist_get_item<'a, 'py>(
    obj: Borrowed<'a, 'py, PyList>,
    pos: usize,
) -> Option<PyCollectionItem<'a, 'py, PyAny>> {
    if pos >= obj.len() {
        return None;
    }

    let item = unsafe {
        let ptr = PyList_GET_ITEM(obj.as_ptr(), pos as isize);
        Borrowed::from_ptr(obj.py(), ptr)
    };
    Some(PyCollectionItem::from_borrowed(item))
}
