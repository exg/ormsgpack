// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::*;
use crate::msgpack;
use crate::opt::*;
use crate::serialize::bytearray::*;
use crate::serialize::bytes::*;
use crate::serialize::dataclass::*;
use crate::serialize::datetime::*;
use crate::serialize::default::*;
use crate::serialize::dict::*;
use crate::serialize::enum_::*;
use crate::serialize::ext::*;
use crate::serialize::fragment::*;
use crate::serialize::list::*;
use crate::serialize::memoryview::*;
use crate::serialize::numpy::*;
use crate::serialize::pydantic::*;
use crate::serialize::state::State;
use crate::serialize::str::*;
use crate::serialize::tuple::*;
use crate::serialize::uuid::*;
use crate::serialize::writer::*;
use pyo3::prelude::*;
use pyo3::types::{
    PyBool, PyByteArray, PyBytes, PyDate, PyDateTime, PyDict, PyFloat, PyInt, PyList, PyMemoryView,
    PyString, PyTime, PyTuple,
};
use pyo3::PyTypeInfo;
use serde::ser::{Serialize, Serializer};

pub fn serialize<'a, 'py>(
    obj: Borrowed<'a, 'py, PyAny>,
    state: &'a State,
    default: Option<Borrowed<'a, 'py, PyAny>>,
    opts: Opt,
) -> PyResult<Bound<'py, PyBytes>> {
    let mut buf = BytesWriter::default();
    let default_hook = DefaultHook::new(default);
    let mut ser = msgpack::Serializer::new(&mut buf);
    let res = PyObject::new(obj, state, opts, &default_hook).serialize(&mut ser);
    match res {
        Ok(_) => Ok(buf.finish(obj.py())),
        Err(err) => Err(state.error(obj.py(), &err.to_string())),
    }
}

pub struct PyObject<'a, 'py> {
    obj: Borrowed<'a, 'py, PyAny>,
    state: &'a State,
    opts: Opt,
    default: &'a DefaultHook<'a, 'py>,
}

impl<'a, 'py> PyObject<'a, 'py> {
    pub fn new(
        obj: Borrowed<'a, 'py, PyAny>,
        state: &'a State,
        opts: Opt,
        default: &'a DefaultHook<'a, 'py>,
    ) -> Self {
        PyObject {
            obj: obj,
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
            .enter_call(self.obj)
            .map_err(serde::ser::Error::custom)?;
        let res = PyObject::new(obj.as_borrowed(), self.state, self.opts, self.default)
            .serialize(serializer);
        self.default.leave_call();
        res
    }

    #[inline(never)]
    fn serialize_unlikely<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let py = self.obj.py();
        let obj = self.obj;
        let type_obj = get_type(obj);
        let type_ptr = type_obj.as_type_ptr();

        if self.opts & PASSTHROUGH_DATETIME == 0 {
            if type_ptr == PyDateTime::type_object_raw(py) {
                let obj = unsafe { obj.cast_unchecked::<PyDateTime>() };
                match DateTime::new(obj, &self.state.datetime_state, self.opts) {
                    Ok(val) => return val.serialize(serializer),
                    Err(err) => return Err(serde::ser::Error::custom(err)),
                }
            }
            if type_ptr == PyDate::type_object_raw(py) {
                let obj = unsafe { obj.cast_unchecked::<PyDate>() };
                return Date::new(obj).serialize(serializer);
            }
            if type_ptr == PyTime::type_object_raw(py) {
                let obj = unsafe { obj.cast_unchecked::<PyTime>() };
                match Time::new(obj, self.opts) {
                    Ok(val) => return val.serialize(serializer),
                    Err(err) => return Err(serde::ser::Error::custom(err)),
                };
            }
        }

        if self.opts & PASSTHROUGH_TUPLE == 0 && type_ptr == PyTuple::type_object_raw(py) {
            let obj = unsafe { obj.cast_unchecked::<PyTuple>() };
            return Tuple::new(obj, self.state, self.opts, self.default).serialize(serializer);
        }

        if self.opts & PASSTHROUGH_UUID == 0
            && UUID::matches_exact_type(type_obj, &self.state.uuid_state)
        {
            return UUID::new(obj, &self.state.uuid_state).serialize(serializer);
        }

        if Enum::matches_exact_type(type_obj, &self.state.enum_state) {
            if self.opts & PASSTHROUGH_ENUM == 0 {
                return Enum::new(obj, self.state, self.opts, self.default).serialize(serializer);
            } else {
                return self.serialize_with_default_hook(serializer);
            }
        }

        if self.opts & PASSTHROUGH_SUBCLASS == 0 {
            if StrSubclass::matches_type(type_obj) {
                let obj = unsafe { obj.cast_unchecked::<PyString>() };
                return StrSubclass::new(obj, self.opts).serialize(serializer);
            }
            if Int::matches_type(type_obj) {
                let obj = unsafe { obj.cast_unchecked::<PyInt>() };
                match Int::new(obj) {
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
            if List::matches_type(type_obj) {
                let obj = unsafe { obj.cast_unchecked::<PyList>() };
                return List::new(obj, self.state, self.opts, self.default).serialize(serializer);
            }
            if Dict::matches_type(type_obj) {
                let obj = unsafe { obj.cast_unchecked::<PyDict>() };
                return Dict::new(obj, self.state, self.opts, self.default).serialize(serializer);
            }
        }

        if type_ptr == self.state.ext_type.as_ptr().cast() {
            return Ext::new(obj).serialize(serializer);
        }

        if self.opts & PASSTHROUGH_DATACLASS == 0
            && Dataclass::matches_type(type_obj, &self.state.dataclass_state)
        {
            return Dataclass::new(obj, self.state, self.opts, self.default).serialize(serializer);
        }

        if self.opts & SERIALIZE_PYDANTIC != 0
            && PydanticModel::matches_type(type_obj, &self.state.pydantic_state)
        {
            return PydanticModel::new(obj, self.state, self.opts, self.default)
                .serialize(serializer);
        }

        if self.opts & SERIALIZE_NUMPY != 0 {
            if let Some(numpy_types_ref) = self
                .state
                .numpy_state
                .get_numpy_types(py)
                .map_err(serde::ser::Error::custom)?
            {
                if type_ptr == numpy_types_ref.bool_.as_ptr().cast() {
                    return NumpyBool::new(obj).serialize(serializer);
                }
                if type_ptr == numpy_types_ref.datetime64.as_ptr().cast() {
                    return NumpyDatetime64::new(obj, &self.state.numpy_state, self.opts)
                        .serialize(serializer);
                }
                if type_ptr == numpy_types_ref.float16.as_ptr().cast() {
                    return NumpyFloat16::new(obj).serialize(serializer);
                }
                if type_ptr == numpy_types_ref.float32.as_ptr().cast() {
                    return NumpyFloat32::new(obj).serialize(serializer);
                }
                if type_ptr == numpy_types_ref.float64.as_ptr().cast() {
                    return NumpyFloat64::new(obj).serialize(serializer);
                }
                if type_ptr == numpy_types_ref.int8.as_ptr().cast() {
                    return NumpyInt8::new(obj).serialize(serializer);
                }
                if type_ptr == numpy_types_ref.int16.as_ptr().cast() {
                    return NumpyInt16::new(obj).serialize(serializer);
                }
                if type_ptr == numpy_types_ref.int32.as_ptr().cast() {
                    return NumpyInt32::new(obj).serialize(serializer);
                }
                if type_ptr == numpy_types_ref.int64.as_ptr().cast() {
                    return NumpyInt64::new(obj).serialize(serializer);
                }
                if type_ptr == numpy_types_ref.uint8.as_ptr().cast() {
                    return NumpyUint8::new(obj).serialize(serializer);
                }
                if type_ptr == numpy_types_ref.uint16.as_ptr().cast() {
                    return NumpyUint16::new(obj).serialize(serializer);
                }
                if type_ptr == numpy_types_ref.uint32.as_ptr().cast() {
                    return NumpyUint32::new(obj).serialize(serializer);
                }
                if type_ptr == numpy_types_ref.uint64.as_ptr().cast() {
                    return NumpyUint64::new(obj).serialize(serializer);
                }
                if type_ptr == numpy_types_ref.array.as_ptr().cast() {
                    match NumpyArray::new(obj, &self.state.numpy_state, self.opts) {
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

        if type_ptr == PyByteArray::type_object_raw(py) {
            let obj = unsafe { obj.cast_unchecked::<PyByteArray>() };
            return ByteArray::new(obj).serialize(serializer);
        }

        if type_ptr == PyMemoryView::type_object_raw(py) {
            let obj = unsafe { obj.cast_unchecked::<PyMemoryView>() };
            return MemoryView::new(obj).serialize(serializer);
        }

        if type_ptr == self.state.fragment_type.as_ptr().cast() {
            return Fragment::new(obj).serialize(serializer);
        }

        self.serialize_with_default_hook(serializer)
    }
}

impl Serialize for PyObject<'_, '_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let py = self.obj.py();
        let obj = self.obj;
        let type_obj = get_type(obj);
        let type_ptr = type_obj.as_type_ptr();

        if type_ptr == PyString::type_object_raw(py) {
            let obj = unsafe { obj.cast_unchecked::<PyString>() };
            Str::new(obj, self.opts).serialize(serializer)
        } else if type_ptr == PyBytes::type_object_raw(py) {
            let obj = unsafe { obj.cast_unchecked::<PyBytes>() };
            Bytes::new(obj).serialize(serializer)
        } else if type_ptr == PyInt::type_object_raw(py) {
            let obj = unsafe { obj.cast_unchecked::<PyInt>() };
            match Int::new(obj) {
                Ok(val) => val.serialize(serializer),
                Err(err) => {
                    if self.opts & PASSTHROUGH_BIG_INT != 0 {
                        self.serialize_with_default_hook(serializer)
                    } else {
                        Err(serde::ser::Error::custom(err))
                    }
                }
            }
        } else if type_ptr == PyBool::type_object_raw(py) {
            let obj = unsafe { obj.cast_unchecked::<PyBool>() };
            serializer.serialize_bool(obj.is_true())
        } else if obj.is_none() {
            serializer.serialize_unit()
        } else if type_ptr == PyFloat::type_object_raw(py) {
            let obj = unsafe { obj.cast_unchecked::<PyFloat>() };
            serializer.serialize_f64(obj.value())
        } else if type_ptr == PyList::type_object_raw(py) {
            let obj = unsafe { obj.cast_unchecked::<PyList>() };
            List::new(obj, self.state, self.opts, self.default).serialize(serializer)
        } else if type_ptr == PyDict::type_object_raw(py) {
            let obj = unsafe { obj.cast_unchecked::<PyDict>() };
            Dict::new(obj, self.state, self.opts, self.default).serialize(serializer)
        } else {
            self.serialize_unlikely(serializer)
        }
    }
}

pub struct DictKey<'a, 'py> {
    obj: Borrowed<'a, 'py, PyAny>,
    state: &'a State,
    opts: Opt,
}

impl<'a, 'py> DictKey<'a, 'py> {
    pub fn new(obj: Borrowed<'a, 'py, PyAny>, state: &'a State, opts: Opt) -> Self {
        DictKey {
            obj: obj,
            state: state,
            opts: opts,
        }
    }

    #[inline(never)]
    fn serialize_unlikely<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let py = self.obj.py();
        let obj = self.obj;
        let type_obj = get_type(obj);
        let type_ptr = type_obj.as_type_ptr();

        if type_ptr == PyDateTime::type_object_raw(py) {
            let obj = unsafe { obj.cast_unchecked::<PyDateTime>() };
            match DateTime::new(obj, &self.state.datetime_state, self.opts) {
                Ok(val) => return val.serialize(serializer),
                Err(err) => return Err(serde::ser::Error::custom(err)),
            }
        }
        if type_ptr == PyDate::type_object_raw(py) {
            let obj = unsafe { obj.cast_unchecked::<PyDate>() };
            return Date::new(obj).serialize(serializer);
        }
        if type_ptr == PyTime::type_object_raw(py) {
            let obj = unsafe { obj.cast_unchecked::<PyTime>() };
            match Time::new(obj, self.opts) {
                Ok(val) => return val.serialize(serializer),
                Err(err) => return Err(serde::ser::Error::custom(err)),
            };
        }

        if type_ptr == PyTuple::type_object_raw(py) {
            let obj = unsafe { obj.cast_unchecked::<PyTuple>() };
            return TupleDictKey::new(obj, self.state, self.opts).serialize(serializer);
        }

        if UUID::matches_exact_type(type_obj, &self.state.uuid_state) {
            return UUID::new(obj, &self.state.uuid_state).serialize(serializer);
        }

        if Enum::matches_exact_type(type_obj, &self.state.enum_state) {
            return EnumDictKey::new(obj, self.state, self.opts).serialize(serializer);
        }

        if StrSubclass::matches_type(type_obj) {
            let obj = unsafe { obj.cast_unchecked::<PyString>() };
            return StrSubclass::new(obj, self.opts).serialize(serializer);
        }
        if Int::matches_type(type_obj) {
            let obj = unsafe { obj.cast_unchecked::<PyInt>() };
            match Int::new(obj) {
                Ok(val) => return val.serialize(serializer),
                Err(err) => return Err(serde::ser::Error::custom(err)),
            }
        }

        if type_ptr == PyMemoryView::type_object_raw(py) {
            let obj = unsafe { obj.cast_unchecked::<PyMemoryView>() };
            return MemoryView::new(obj).serialize(serializer);
        }

        Err(serde::ser::Error::custom(
            "Dict key must a type serializable with OPT_NON_STR_KEYS",
        ))
    }
}

impl Serialize for DictKey<'_, '_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let py = self.obj.py();
        let obj = self.obj;
        let type_obj = get_type(obj);
        let type_ptr = type_obj.as_type_ptr();

        if type_ptr == PyString::type_object_raw(py) {
            let obj = unsafe { obj.cast_unchecked::<PyString>() };
            Str::new(obj, self.opts).serialize(serializer)
        } else if type_ptr == PyBytes::type_object_raw(py) {
            let obj = unsafe { obj.cast_unchecked::<PyBytes>() };
            Bytes::new(obj).serialize(serializer)
        } else if type_ptr == PyInt::type_object_raw(py) {
            let obj = unsafe { obj.cast_unchecked::<PyInt>() };
            match Int::new(obj) {
                Ok(val) => val.serialize(serializer),
                Err(err) => Err(serde::ser::Error::custom(err)),
            }
        } else if type_ptr == PyBool::type_object_raw(py) {
            let obj = unsafe { obj.cast_unchecked::<PyBool>() };
            serializer.serialize_bool(obj.is_true())
        } else if obj.is_none() {
            serializer.serialize_unit()
        } else if type_ptr == PyFloat::type_object_raw(py) {
            let obj = unsafe { obj.cast_unchecked::<PyFloat>() };
            serializer.serialize_f64(obj.value())
        } else {
            self.serialize_unlikely(serializer)
        }
    }
}
