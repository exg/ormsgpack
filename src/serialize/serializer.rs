// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::*;
use crate::msgpack;
use crate::opt::*;
use crate::serialize::bool_::*;
use crate::serialize::bytearray::*;
use crate::serialize::bytes::*;
use crate::serialize::dataclass::*;
use crate::serialize::datetime::*;
use crate::serialize::default::*;
use crate::serialize::dict::*;
use crate::serialize::enum_::*;
use crate::serialize::ext::*;
use crate::serialize::float::*;
use crate::serialize::fragment::*;
use crate::serialize::list::*;
use crate::serialize::memoryview::*;
use crate::serialize::numpy::*;
use crate::serialize::pydantic::*;
use crate::serialize::str::*;
use crate::serialize::tuple::*;
use crate::serialize::uuid::*;
use crate::serialize::writer::*;
use crate::serialize::State;
use serde::ser::{Serialize, Serializer};
use std::os::raw::c_ulong;
use std::ptr::NonNull;

pub fn serialize(
    ptr: *mut pyo3::ffi::PyObject,
    state: &State,
    default: Option<NonNull<pyo3::ffi::PyObject>>,
    opts: Opt,
) -> Result<NonNull<pyo3::ffi::PyObject>, String> {
    let mut buf = BytesWriter::default();
    let default_hook = DefaultHook::new(default);
    let obj = PyObject::new(ptr, state, opts, &default_hook);
    let mut ser = msgpack::Serializer::new(&mut buf);
    let res = obj.serialize(&mut ser);
    match res {
        Ok(_) => Ok(buf.finish()),
        Err(err) => {
            unsafe { pyo3::ffi::Py_DECREF(buf.finish().as_ptr()) };
            Err(err.to_string())
        }
    }
}

#[inline(always)]
fn is_subclass(op: *mut pyo3::ffi::PyTypeObject, feature: c_ulong) -> bool {
    unsafe { pyo3::ffi::PyType_HasFeature(op, feature) != 0 }
}

pub struct PyObject<'a> {
    ptr: *mut pyo3::ffi::PyObject,
    state: &'a State,
    opts: Opt,
    default: &'a DefaultHook,
}

impl<'a> PyObject<'a> {
    pub fn new(
        ptr: *mut pyo3::ffi::PyObject,
        state: &'a State,
        opts: Opt,
        default: &'a DefaultHook,
    ) -> Self {
        PyObject {
            ptr: ptr,
            state: state,
            opts: opts,
            default: default,
        }
    }

    fn serialize_with_default_hook<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let obj = self
            .default
            .enter_call(self.ptr)
            .map_err(serde::ser::Error::custom)?;
        let res = PyObject::new(obj, self.state, self.opts, self.default).serialize(serializer);
        self.default.leave_call();
        unsafe { pyo3::ffi::Py_DECREF(obj) };
        res
    }

    #[inline(never)]
    fn serialize_unlikely<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let ob_type = unsafe { pyo3::ffi::Py_TYPE(self.ptr) };

        if self.opts & PASSTHROUGH_DATETIME == 0 {
            let datetime_api = unsafe { *pyo3::ffi::PyDateTimeAPI() };
            if ob_type == datetime_api.DateTimeType {
                match DateTime::new(self.ptr, &self.state.datetime, self.opts) {
                    Ok(val) => return val.serialize(serializer),
                    Err(err) => return Err(serde::ser::Error::custom(err)),
                }
            }
            if ob_type == datetime_api.DateType {
                return Date::new(self.ptr).serialize(serializer);
            }
            if ob_type == datetime_api.TimeType {
                match Time::new(self.ptr, self.opts) {
                    Ok(val) => return val.serialize(serializer),
                    Err(err) => return Err(serde::ser::Error::custom(err)),
                };
            }
        }

        if self.opts & PASSTHROUGH_TUPLE == 0 && ob_type == &raw mut pyo3::ffi::PyTuple_Type {
            return Tuple::new(self.ptr, self.state, self.opts, self.default).serialize(serializer);
        }

        if self.opts & PASSTHROUGH_UUID == 0
            && ob_type == self.state.uuid.type_object.as_ptr().cast()
        {
            return UUID::new(self.ptr, &self.state.uuid).serialize(serializer);
        }

        let is_enum = unsafe {
            let ob_type = pyo3::ffi::Py_TYPE(ob_type.cast());
            ob_type == self.state.enum_.type_object.as_ptr().cast()
        };
        if is_enum {
            if self.opts & PASSTHROUGH_ENUM == 0 {
                return Enum::new(self.ptr, self.state, self.opts, self.default)
                    .serialize(serializer);
            } else {
                return self.serialize_with_default_hook(serializer);
            }
        }

        if self.opts & PASSTHROUGH_SUBCLASS == 0 {
            if is_subclass(ob_type, pyo3::ffi::Py_TPFLAGS_UNICODE_SUBCLASS) {
                return StrSubclass::new(self.ptr, self.opts).serialize(serializer);
            }
            if is_subclass(ob_type, pyo3::ffi::Py_TPFLAGS_LONG_SUBCLASS) {
                match Int::new(self.ptr) {
                    Ok(val) => return val.serialize(serializer),
                    Err(err) => {
                        if self.opts & PASSTHROUGH_BIG_INT != 0 {
                            return self.serialize_with_default_hook(serializer);
                        } else {
                            return Err(serde::ser::Error::custom(err));
                        }
                    }
                }
            }
            if is_subclass(ob_type, pyo3::ffi::Py_TPFLAGS_LIST_SUBCLASS) {
                return List::new(self.ptr, self.state, self.opts, self.default)
                    .serialize(serializer);
            }
            if is_subclass(ob_type, pyo3::ffi::Py_TPFLAGS_DICT_SUBCLASS) {
                return Dict::new(self.ptr, self.state, self.opts, self.default)
                    .serialize(serializer);
            }
        }

        if ob_type == self.state.ext.type_object.as_ptr().cast() {
            return Ext::new(self.ptr).serialize(serializer);
        }

        if self.opts & PASSTHROUGH_DATACLASS == 0 && is_dataclass(ob_type, self.state) {
            return Dataclass::new(self.ptr, self.state, self.opts, self.default)
                .serialize(serializer);
        }

        if self.opts & SERIALIZE_PYDANTIC != 0 && is_pydantic_model(ob_type, self.state) {
            return PydanticModel::new(self.ptr, self.state, self.opts, self.default)
                .serialize(serializer);
        }

        if self.opts & SERIALIZE_NUMPY != 0 {
            if let Some(numpy_types_ref) = self
                .state
                .numpy
                .get_types()
                .map_err(|()| serde::ser::Error::custom("numpy initialization failed"))?
            {
                if ob_type == numpy_types_ref.bool_.as_ptr().cast() {
                    return NumpyBool::new(self.ptr).serialize(serializer);
                }
                if ob_type == numpy_types_ref.datetime64.as_ptr().cast() {
                    return NumpyDatetime64::new(self.ptr, &self.state.numpy, self.opts)
                        .serialize(serializer);
                }
                if ob_type == numpy_types_ref.float16.as_ptr().cast() {
                    return NumpyFloat16::new(self.ptr).serialize(serializer);
                }
                if ob_type == numpy_types_ref.float32.as_ptr().cast() {
                    return NumpyFloat32::new(self.ptr).serialize(serializer);
                }
                if ob_type == numpy_types_ref.float64.as_ptr().cast() {
                    return NumpyFloat64::new(self.ptr).serialize(serializer);
                }
                if ob_type == numpy_types_ref.int8.as_ptr().cast() {
                    return NumpyInt8::new(self.ptr).serialize(serializer);
                }
                if ob_type == numpy_types_ref.int16.as_ptr().cast() {
                    return NumpyInt16::new(self.ptr).serialize(serializer);
                }
                if ob_type == numpy_types_ref.int32.as_ptr().cast() {
                    return NumpyInt32::new(self.ptr).serialize(serializer);
                }
                if ob_type == numpy_types_ref.int64.as_ptr().cast() {
                    return NumpyInt64::new(self.ptr).serialize(serializer);
                }
                if ob_type == numpy_types_ref.uint8.as_ptr().cast() {
                    return NumpyUint8::new(self.ptr).serialize(serializer);
                }
                if ob_type == numpy_types_ref.uint16.as_ptr().cast() {
                    return NumpyUint16::new(self.ptr).serialize(serializer);
                }
                if ob_type == numpy_types_ref.uint32.as_ptr().cast() {
                    return NumpyUint32::new(self.ptr).serialize(serializer);
                }
                if ob_type == numpy_types_ref.uint64.as_ptr().cast() {
                    return NumpyUint64::new(self.ptr).serialize(serializer);
                }
                if ob_type == numpy_types_ref.array.as_ptr().cast() {
                    match NumpyArray::new(self.ptr, &self.state.numpy, self.opts) {
                        Ok(val) => return val.serialize(serializer),
                        Err(PyArrayError::Malformed) => {
                            return Err(serde::ser::Error::custom("numpy array is malformed"))
                        }
                        Err(PyArrayError::NotContiguous)
                        | Err(PyArrayError::UnsupportedDataType) => {
                            if self.default.inner.is_none() {
                                return Err(serde::ser::Error::custom("numpy array is not C contiguous; use ndarray.tolist() in default"));
                            }
                        }
                    }
                }
            }
        }

        if ob_type == &raw mut pyo3::ffi::PyByteArray_Type {
            return ByteArray::new(self.ptr).serialize(serializer);
        }

        if ob_type == &raw mut pyo3::ffi::PyMemoryView_Type {
            return MemoryView::new(self.ptr).serialize(serializer);
        }

        if ob_type == self.state.fragment.type_object.as_ptr().cast() {
            return Fragment::new(self.ptr).serialize(serializer);
        }

        self.serialize_with_default_hook(serializer)
    }
}

impl Serialize for PyObject<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let ob_type = unsafe { pyo3::ffi::Py_TYPE(self.ptr) };
        if ob_type == &raw mut pyo3::ffi::PyUnicode_Type {
            Str::new(self.ptr, self.opts).serialize(serializer)
        } else if ob_type == &raw mut pyo3::ffi::PyBytes_Type {
            Bytes::new(self.ptr).serialize(serializer)
        } else if ob_type == &raw mut pyo3::ffi::PyLong_Type {
            match Int::new(self.ptr) {
                Ok(val) => val.serialize(serializer),
                Err(err) => {
                    if self.opts & PASSTHROUGH_BIG_INT != 0 {
                        self.serialize_with_default_hook(serializer)
                    } else {
                        Err(serde::ser::Error::custom(err))
                    }
                }
            }
        } else if ob_type == &raw mut pyo3::ffi::PyBool_Type {
            Bool::new(self.ptr).serialize(serializer)
        } else if self.ptr == unsafe { pyo3::ffi::Py_None() } {
            serializer.serialize_unit()
        } else if ob_type == &raw mut pyo3::ffi::PyFloat_Type {
            Float::new(self.ptr).serialize(serializer)
        } else if ob_type == &raw mut pyo3::ffi::PyList_Type {
            List::new(self.ptr, self.state, self.opts, self.default).serialize(serializer)
        } else if ob_type == &raw mut pyo3::ffi::PyDict_Type {
            Dict::new(self.ptr, self.state, self.opts, self.default).serialize(serializer)
        } else {
            self.serialize_unlikely(serializer)
        }
    }
}

pub struct DictKey<'a> {
    ptr: *mut pyo3::ffi::PyObject,
    state: &'a State,
    opts: Opt,
}

impl<'a> DictKey<'a> {
    pub fn new(ptr: *mut pyo3::ffi::PyObject, state: &'a State, opts: Opt) -> Self {
        DictKey {
            ptr: ptr,
            state: state,
            opts: opts,
        }
    }

    #[inline(never)]
    fn serialize_unlikely<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let ob_type = unsafe { pyo3::ffi::Py_TYPE(self.ptr) };

        let datetime_api = unsafe { *pyo3::ffi::PyDateTimeAPI() };
        if ob_type == datetime_api.DateTimeType {
            match DateTime::new(self.ptr, &self.state.datetime, self.opts) {
                Ok(val) => return val.serialize(serializer),
                Err(err) => return Err(serde::ser::Error::custom(err)),
            }
        }
        if ob_type == datetime_api.DateType {
            return Date::new(self.ptr).serialize(serializer);
        }
        if ob_type == datetime_api.TimeType {
            match Time::new(self.ptr, self.opts) {
                Ok(val) => return val.serialize(serializer),
                Err(err) => return Err(serde::ser::Error::custom(err)),
            };
        }

        if ob_type == &raw mut pyo3::ffi::PyTuple_Type {
            return TupleDictKey::new(self.ptr, self.state, self.opts).serialize(serializer);
        }

        if ob_type == self.state.uuid.type_object.as_ptr().cast() {
            return UUID::new(self.ptr, &self.state.uuid).serialize(serializer);
        }

        let is_enum = unsafe {
            let ob_type = pyo3::ffi::Py_TYPE(ob_type.cast());
            ob_type == self.state.enum_.type_object.as_ptr().cast()
        };
        if is_enum {
            return EnumDictKey::new(self.ptr, self.state, self.opts).serialize(serializer);
        }

        if is_subclass(ob_type, pyo3::ffi::Py_TPFLAGS_UNICODE_SUBCLASS) {
            return StrSubclass::new(self.ptr, self.opts).serialize(serializer);
        }
        if is_subclass(ob_type, pyo3::ffi::Py_TPFLAGS_LONG_SUBCLASS) {
            match Int::new(self.ptr) {
                Ok(val) => return val.serialize(serializer),
                Err(err) => return Err(serde::ser::Error::custom(err)),
            }
        }

        if ob_type == &raw mut pyo3::ffi::PyMemoryView_Type {
            return MemoryView::new(self.ptr).serialize(serializer);
        }

        Err(serde::ser::Error::custom(
            "Dict key must a type serializable with OPT_NON_STR_KEYS",
        ))
    }
}

impl Serialize for DictKey<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let ob_type = unsafe { pyo3::ffi::Py_TYPE(self.ptr) };
        if ob_type == &raw mut pyo3::ffi::PyUnicode_Type {
            Str::new(self.ptr, self.opts).serialize(serializer)
        } else if ob_type == &raw mut pyo3::ffi::PyBytes_Type {
            Bytes::new(self.ptr).serialize(serializer)
        } else if ob_type == &raw mut pyo3::ffi::PyLong_Type {
            match Int::new(self.ptr) {
                Ok(val) => val.serialize(serializer),
                Err(err) => Err(serde::ser::Error::custom(err)),
            }
        } else if ob_type == &raw mut pyo3::ffi::PyBool_Type {
            Bool::new(self.ptr).serialize(serializer)
        } else if self.ptr == unsafe { pyo3::ffi::Py_None() } {
            serializer.serialize_unit()
        } else if ob_type == &raw mut pyo3::ffi::PyFloat_Type {
            Float::new(self.ptr).serialize(serializer)
        } else {
            self.serialize_unlikely(serializer)
        }
    }
}
