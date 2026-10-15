// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use pyo3::exceptions::PyTypeError;
use pyo3::gc::{PyTraverseError, PyVisit};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyInt};

#[pyclass(name = "Ext", module = "ormsgpack", frozen)]
pub struct PyExt {
    pub tag: Py<PyInt>,
    pub data: Py<PyBytes>,
}

#[pymethods]
impl PyExt {
    #[new]
    #[pyo3(signature = (tag, data, /))]
    fn new(tag: &Bound<'_, PyAny>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        if !tag.is_instance_of::<PyInt>() {
            let err = PyTypeError::new_err("Ext.__new__() first argument must be int");
            return Err(err);
        }
        let tag = unsafe { tag.cast_unchecked::<PyInt>() };
        if !data.is_instance_of::<PyBytes>() {
            let err = PyTypeError::new_err("Ext.__new__() second argument must be bytes");
            return Err(err);
        }
        let data = unsafe { data.cast_unchecked::<PyBytes>() };
        Ok(Self {
            tag: tag.clone().unbind(),
            data: data.clone().unbind(),
        })
    }

    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        visit.call(&self.tag)?;
        visit.call(&self.data)
    }
}
