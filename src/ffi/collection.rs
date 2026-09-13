// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::get_type;
use pyo3::prelude::*;
use pyo3::PyTypeInfo;
#[cfg(feature = "hardened")]
use std::marker::PhantomData;

#[repr(transparent)]
#[cfg(not(feature = "hardened"))]
pub struct PyCollectionItem<'a, 'py, T>(Borrowed<'a, 'py, T>);

#[repr(transparent)]
#[cfg(feature = "hardened")]
pub struct PyCollectionItem<'a, 'py, T>(Bound<'py, T>, PhantomData<&'a ()>);

impl<'a, 'py, T> PyCollectionItem<'a, 'py, T> {
    #[inline(always)]
    pub fn from_borrowed(obj: Borrowed<'a, 'py, T>) -> Self {
        #[cfg(not(feature = "hardened"))]
        {
            Self(obj)
        }
        #[cfg(feature = "hardened")]
        {
            Self(obj.to_owned(), PhantomData)
        }
    }

    #[inline(always)]
    pub fn as_borrowed(&self) -> Borrowed<'_, 'py, T> {
        self.0.as_borrowed()
    }
}

impl<'a, 'py> PyCollectionItem<'a, 'py, PyAny> {
    #[inline(always)]
    pub fn cast_exact<T: PyTypeInfo>(
        self,
    ) -> Result<PyCollectionItem<'a, 'py, T>, PyCollectionItem<'a, 'py, PyAny>> {
        let obj = self.as_borrowed();
        if get_type(obj).as_type_ptr() != T::type_object_raw(obj.py()) {
            return Err(self);
        }

        #[cfg(not(feature = "hardened"))]
        {
            Ok(PyCollectionItem(unsafe { self.0.cast_unchecked() }))
        }
        #[cfg(feature = "hardened")]
        {
            Ok(PyCollectionItem(
                unsafe { self.0.cast_into_unchecked() },
                PhantomData,
            ))
        }
    }
}
