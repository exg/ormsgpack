// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::PyObjectWithType;
use crate::ffi::*;
use crate::opt::*;
use crate::util::unlikely;

use serde::ser::{Serialize, Serializer};

#[repr(transparent)]
struct StrWithSurrogates<'a> {
    obj: BorrowedPyObject<'a>,
}

impl<'a> StrWithSurrogates<'a> {
    pub fn new(obj: BorrowedPyObject<'a>) -> Self {
        StrWithSurrogates { obj }
    }
}

impl Serialize for StrWithSurrogates<'_> {
    #[inline(never)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        unsafe {
            let ptr = pyo3::ffi::PyUnicode_AsEncodedString(
                self.obj.as_ptr(),
                c"UTF-8".as_ptr(),
                c"replace".as_ptr(),
            );
            if unlikely(ptr.is_null()) {
                return Err(serde::ser::Error::custom("invalid string"));
            }
            let slice = pybytes_as_bytes(ptr);
            let uni = std::str::from_utf8_unchecked(slice);
            let res = serializer.serialize_str(uni);
            pyo3::ffi::Py_DECREF(ptr);
            res
        }
    }
}

pub struct Str<'a> {
    obj: BorrowedPyObject<'a>,
    opts: Opt,
}

impl<'a> Str<'a> {
    #[inline]
    pub fn try_new(obj: PyObjectWithType<'a>, opts: Opt) -> Option<Self> {
        if obj.get_type_ptr() == &raw mut pyo3::ffi::PyUnicode_Type {
            Some(Self {
                obj: obj.as_borrowed(),
                opts,
            })
        } else {
            None
        }
    }
}

impl Serialize for Str<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match unicode_to_str(self.obj.as_ptr()) {
            Ok(val) => serializer.serialize_str(val),
            Err(err) => {
                if self.opts & REPLACE_SURROGATES != 0 {
                    StrWithSurrogates::new(self.obj).serialize(serializer)
                } else {
                    Err(serde::ser::Error::custom(err))
                }
            }
        }
    }
}

pub struct StrSubclass<'a> {
    obj: BorrowedPyObject<'a>,
    opts: Opt,
}

impl<'a> StrSubclass<'a> {
    #[inline]
    pub fn try_new(obj: PyObjectWithType<'a>, opts: Opt) -> Option<Self> {
        if unsafe {
            pyo3::ffi::PyType_HasFeature(obj.get_type_ptr(), pyo3::ffi::Py_TPFLAGS_UNICODE_SUBCLASS)
                != 0
        } {
            Some(Self {
                obj: obj.as_borrowed(),
                opts,
            })
        } else {
            None
        }
    }
}

impl Serialize for StrSubclass<'_> {
    #[inline(never)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match unicode_to_str_via_ffi(self.obj.as_ptr()) {
            Ok(val) => serializer.serialize_str(val),
            Err(err) => {
                if self.opts & REPLACE_SURROGATES != 0 {
                    StrWithSurrogates::new(self.obj).serialize(serializer)
                } else {
                    Err(serde::ser::Error::custom(err))
                }
            }
        }
    }
}
