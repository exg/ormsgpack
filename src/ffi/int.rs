// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::BorrowedWithType;
use crate::util::unlikely;
use pyo3::ffi::*;
use pyo3::prelude::*;
use pyo3::types::PyInt;
use serde::ser::{Serialize, Serializer};
use std::ffi::{c_long, c_ulong};

#[inline(always)]
pub fn pylong_to_i64(obj: Borrowed<'_, '_, PyInt>) -> Option<i64> {
    let op = obj.as_ptr();
    unsafe {
        let value: i64 = if std::mem::size_of::<c_long>() == 8 {
            #[allow(clippy::useless_conversion)]
            PyLong_AsLong(op).into()
        } else {
            PyLong_AsLongLong(op)
        };
        if unlikely(value == -1 && !PyErr_Occurred().is_null()) {
            PyErr_Clear();
            None
        } else {
            Some(value)
        }
    }
}

#[inline(always)]
pub fn pylong_to_u64(obj: Borrowed<'_, '_, PyInt>) -> Option<u64> {
    let op = obj.as_ptr();
    unsafe {
        let value: u64 = if std::mem::size_of::<c_ulong>() == 8 {
            #[allow(clippy::useless_conversion)]
            PyLong_AsUnsignedLong(op).into()
        } else {
            PyLong_AsUnsignedLongLong(op)
        };
        if unlikely(value == u64::MAX && !PyErr_Occurred().is_null()) {
            PyErr_Clear();
            None
        } else {
            Some(value)
        }
    }
}

// https://tools.ietf.org/html/rfc7159#section-6
// "[-(2**53)+1, (2**53)-1]"

pub enum IntError {
    Overflow,
}

impl std::fmt::Display for IntError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Overflow => write!(f, "Integer exceeds 64-bit range"),
        }
    }
}

pub enum Int {
    Signed(i64),
    Unsigned(u64),
}

impl Int {
    #[inline]
    pub fn try_new_exact(obj: BorrowedWithType<'_, '_>) -> Result<Option<Self>, IntError> {
        let Some(obj) = obj.cast_exact::<PyInt>() else {
            return Ok(None);
        };
        Self::new(obj).map(Some)
    }

    #[inline]
    pub fn try_new(obj: BorrowedWithType<'_, '_>) -> Result<Option<Self>, IntError> {
        if unsafe { PyType_HasFeature(obj.get_type_ptr(), Py_TPFLAGS_LONG_SUBCLASS) != 0 } {
            Self::new(unsafe { obj.as_borrowed().cast_unchecked() }).map(Some)
        } else {
            Ok(None)
        }
    }
}

impl Serialize for Int {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Int::Signed(value) => serializer.serialize_i64(*value),
            Int::Unsigned(value) => serializer.serialize_u64(*value),
        }
    }
}
