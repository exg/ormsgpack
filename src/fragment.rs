// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::msgpack;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::gc::{PyTraverseError, PyVisit};
use pyo3::prelude::*;
use pyo3::types::PyBytes;

#[pyclass(name = "Fragment", module = "ormsgpack", frozen)]
pub struct PyFragment {
    pub data: Py<PyBytes>,
}

#[pymethods]
impl PyFragment {
    #[new]
    #[pyo3(signature = (data, /))]
    fn new(data: &Bound<'_, PyAny>) -> PyResult<Self> {
        if !data.is_instance_of::<PyBytes>() {
            let err = PyTypeError::new_err("Fragment.__new__() first argument must be bytes");
            return Err(err);
        }
        let data = unsafe { data.cast_unchecked::<PyBytes>() };
        let contents = data.as_bytes();
        let mut validator = msgpack::Validator::new(contents);
        validator
            .validate()
            .map_err(|err| PyValueError::new_err(err.to_string()))?;
        Ok(Self {
            data: data.clone().unbind(),
        })
    }

    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        visit.call(&self.data)
    }
}
