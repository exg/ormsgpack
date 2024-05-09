use pyo3::prelude::*;
use pyo3::types::PyBytes;

#[pyclass]
pub struct Ext {
    #[pyo3(get)]
    pub tag: i8,
    pub data: Vec<u8>,
}

#[pymethods]
impl Ext {
    #[new]
    fn new(tag: i8, data: Vec<u8>) -> Self {
        Ext { tag, data }
    }

    #[getter]
    fn data<'a>(&self, py: Python<'a>) -> Bound<'a, PyBytes> {
        PyBytes::new(py, self.data.as_ref())
    }
}
