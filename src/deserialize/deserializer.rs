// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::deserialize::cache::*;
use crate::deserialize::DeserializeError;
use crate::ffi::*;
use crate::opt::*;
use crate::typeref::*;
use crate::unicode::*;
use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_bytes::ByteBuf;
use std::borrow::Cow;
use std::fmt;
use std::os::raw::c_char;
use std::ptr::NonNull;

pub fn deserialize(
    ptr: *mut pyo3::ffi::PyObject,
    ext_hook: Option<NonNull<pyo3::ffi::PyObject>>,
    opts: Opt,
) -> std::result::Result<NonNull<pyo3::ffi::PyObject>, DeserializeError<'static>> {
    let obj_type_ptr = ob_type!(ptr);
    let buffer: *const u8;
    let length: usize;

    if is_type!(obj_type_ptr, BYTES_TYPE) {
        buffer = unsafe { PyBytes_AS_STRING(ptr) as *const u8 };
        length = unsafe { PyBytes_GET_SIZE(ptr) as usize };
    } else if is_type!(obj_type_ptr, MEMORYVIEW_TYPE) {
        let membuf = unsafe { PyMemoryView_GET_BUFFER(ptr) };
        if unsafe { pyo3::ffi::PyBuffer_IsContiguous(membuf, b'C' as c_char) == 0 } {
            return Err(DeserializeError::new(Cow::Borrowed(
                "Input type memoryview must be a C contiguous buffer",
            )));
        }
        buffer = unsafe { (*membuf).buf as *const u8 };
        length = unsafe { (*membuf).len as usize };
    } else if is_type!(obj_type_ptr, BYTEARRAY_TYPE) {
        buffer = ffi!(PyByteArray_AsString(ptr)) as *const u8;
        length = ffi!(PyByteArray_Size(ptr)) as usize;
    } else {
        return Err(DeserializeError::new(Cow::Borrowed(
            "Input must be bytes, bytearray, memoryview",
        )));
    }
    let contents: &[u8] = unsafe { std::slice::from_raw_parts(buffer, length) };

    let mut deserializer = rmp_serde::Deserializer::new(contents);
    if (opts & NON_STR_KEYS) != 0 {
        let seed = MsgpackNonStrDictValue { ext_hook };
        seed.deserialize(&mut deserializer)
            .map_err(|e| DeserializeError::new(Cow::Owned(e.to_string())))
    } else {
        let seed = MsgpackValue { ext_hook };
        seed.deserialize(&mut deserializer)
            .map_err(|e| DeserializeError::new(Cow::Owned(e.to_string())))
    }
}

fn unicode_from_map_key(key: &str) -> *mut pyo3::ffi::PyObject {
    if unlikely!(key.len() > 64) {
        let pykey = unicode_from_str(key);
        hash_str(pykey);
        pykey
    } else {
        let hash = cache_hash(key.as_bytes());
        let map = unsafe { KEY_MAP.get_mut().unwrap_or_else(|| unreachable!()) };
        let entry = map.entry(&hash).or_insert_with(
            || hash,
            || {
                let pykey = unicode_from_str(key);
                hash_str(pykey);
                CachedKey::new(pykey)
            },
        );
        entry.get()
    }
}

#[derive(Clone, Copy)]
struct MsgpackExtValue {
    ext_hook: Option<NonNull<pyo3::ffi::PyObject>>,
}

impl<'de> Visitor<'de> for MsgpackExtValue {
    type Value = NonNull<pyo3::ffi::PyObject>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("msgpack extension type")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let tag: i8 = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(0, &self))?;

        let data: ByteBuf = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(1, &self))?;

        match self.ext_hook {
            Some(callable) => {
                let tag_obj = ffi!(PyLong_FromLongLong(tag as i64));
                let data_ptr = data.as_ptr() as *const c_char;
                let data_len = data.len() as pyo3::ffi::Py_ssize_t;
                let data_obj = ffi!(PyBytes_FromStringAndSize(data_ptr, data_len));
                #[allow(clippy::unnecessary_cast)]
                let obj = ffi!(PyObject_CallFunctionObjArgs(
                    callable.as_ptr(),
                    tag_obj,
                    data_obj,
                    std::ptr::null_mut() as *mut pyo3::ffi::PyObject
                ));
                ffi!(Py_DECREF(tag_obj));
                ffi!(Py_DECREF(data_obj));
                if unlikely!(obj.is_null()) {
                    Err(de::Error::custom("ext_hook failed"))
                } else {
                    Ok(nonnull!(obj))
                }
            }
            None => Err(de::Error::custom("ext_hook missing")),
        }
    }
}

#[derive(Clone, Copy)]
struct MsgpackValue {
    ext_hook: Option<NonNull<pyo3::ffi::PyObject>>,
}

impl<'de> DeserializeSeed<'de> for MsgpackValue {
    type Value = NonNull<pyo3::ffi::PyObject>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for MsgpackValue {
    type Value = NonNull<pyo3::ffi::PyObject>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("msgpack")
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_tuple(
            2,
            MsgpackExtValue {
                ext_hook: self.ext_hook,
            },
        )
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        ffi!(Py_INCREF(NONE));
        Ok(nonnull!(NONE))
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if value {
            ffi!(Py_INCREF(TRUE));
            Ok(nonnull!(TRUE))
        } else {
            ffi!(Py_INCREF(FALSE));
            Ok(nonnull!(FALSE))
        }
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(nonnull!(ffi!(PyLong_FromLongLong(value))))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(nonnull!(ffi!(PyLong_FromUnsignedLongLong(value))))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(nonnull!(ffi!(PyFloat_FromDouble(value))))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(nonnull!(unicode_from_str(value.as_str())))
    }

    fn visit_borrowed_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(nonnull!(unicode_from_str(value)))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(nonnull!(unicode_from_str(value)))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let ptr = v.as_ptr() as *const c_char;
        let len = v.len() as pyo3::ffi::Py_ssize_t;
        Ok(nonnull!(ffi!(PyBytes_FromStringAndSize(ptr, len))))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let size = seq.size_hint().unwrap() as pyo3::ffi::Py_ssize_t;
        let ptr = ffi!(PyList_New(size));
        let mut i = 0;
        while let Some(elem) = seq.next_element_seed(self)? {
            ffi!(PyList_SET_ITEM(
                ptr,
                i as pyo3::ffi::Py_ssize_t,
                elem.as_ptr()
            ));
            i += 1;
        }
        Ok(nonnull!(ptr))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let size = map.size_hint().unwrap() as pyo3::ffi::Py_ssize_t;
        let dict_ptr = ffi!(_PyDict_NewPresized(size));
        while let Some(key) = map.next_key::<Cow<str>>()? {
            let value = map.next_value_seed(self)?;
            let pykey = unicode_from_map_key(&key);
            let pyhash = unsafe { (*pykey.cast::<pyo3::ffi::PyASCIIObject>()).hash };
            let _ = ffi!(_PyDict_SetItem_KnownHash(
                dict_ptr,
                pykey,
                value.as_ptr(),
                pyhash
            ));
            // counter Py_INCREF in insertdict
            ffi!(Py_DECREF(pykey));
            ffi!(Py_DECREF(value.as_ptr()));
        }
        Ok(nonnull!(dict_ptr))
    }
}

// Implemntation of MsgpackValue that can deserialize non-str keys also.
#[derive(Clone, Copy)]
struct MsgpackNonStrDictValue {
    ext_hook: Option<NonNull<pyo3::ffi::PyObject>>,
}

impl<'de> DeserializeSeed<'de> for MsgpackNonStrDictValue {
    type Value = NonNull<pyo3::ffi::PyObject>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for MsgpackNonStrDictValue {
    type Value = NonNull<pyo3::ffi::PyObject>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("msgpack")
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_tuple(
            2,
            MsgpackExtValue {
                ext_hook: self.ext_hook,
            },
        )
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        ffi!(Py_INCREF(NONE));
        Ok(nonnull!(NONE))
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if value {
            ffi!(Py_INCREF(TRUE));
            Ok(nonnull!(TRUE))
        } else {
            ffi!(Py_INCREF(FALSE));
            Ok(nonnull!(FALSE))
        }
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(nonnull!(ffi!(PyLong_FromLongLong(value))))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(nonnull!(ffi!(PyLong_FromUnsignedLongLong(value))))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(nonnull!(ffi!(PyFloat_FromDouble(value))))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(nonnull!(unicode_from_str(value.as_str())))
    }

    fn visit_borrowed_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(nonnull!(unicode_from_str(value)))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(nonnull!(unicode_from_str(value)))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let ptr = v.as_ptr() as *const c_char;
        let len = v.len() as pyo3::ffi::Py_ssize_t;
        Ok(nonnull!(ffi!(PyBytes_FromStringAndSize(ptr, len))))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let size = seq.size_hint().unwrap() as pyo3::ffi::Py_ssize_t;
        let ptr = ffi!(PyList_New(size));
        let mut i = 0;
        while let Some(elem) = seq.next_element_seed(self)? {
            ffi!(PyList_SET_ITEM(
                ptr,
                i as pyo3::ffi::Py_ssize_t,
                elem.as_ptr()
            ));
            i += 1;
        }
        Ok(nonnull!(ptr))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let size = map.size_hint().unwrap() as pyo3::ffi::Py_ssize_t;
        let dict_ptr = ffi!(_PyDict_NewPresized(size));
        while let Some((key, value)) = map.next_entry_seed(MsgpackKey {}, self)? {
            let ret = ffi!(PyDict_SetItem(dict_ptr, key.as_ptr(), value.as_ptr()));
            if unlikely!(ret == -1) {
                return Err(de::Error::custom("PyDict_SetItem failed"));
            }
            ffi!(Py_DECREF(key.as_ptr()));
            ffi!(Py_DECREF(value.as_ptr()));
        }
        Ok(nonnull!(dict_ptr))
    }
}

#[derive(Clone, Copy)]
struct MsgpackKey;

impl<'de> DeserializeSeed<'de> for MsgpackKey {
    type Value = NonNull<pyo3::ffi::PyObject>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for MsgpackKey {
    type Value = NonNull<pyo3::ffi::PyObject>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("msgpack")
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        ffi!(Py_INCREF(NONE));
        Ok(nonnull!(NONE))
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if value {
            ffi!(Py_INCREF(TRUE));
            Ok(nonnull!(TRUE))
        } else {
            ffi!(Py_INCREF(FALSE));
            Ok(nonnull!(FALSE))
        }
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(nonnull!(ffi!(PyLong_FromLongLong(value))))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(nonnull!(ffi!(PyLong_FromUnsignedLongLong(value))))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(nonnull!(ffi!(PyFloat_FromDouble(value))))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(nonnull!(unicode_from_map_key(value.as_str())))
    }

    fn visit_borrowed_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(nonnull!(unicode_from_map_key(value)))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(nonnull!(unicode_from_map_key(value)))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let ptr = v.as_ptr() as *const c_char;
        let len = v.len() as pyo3::ffi::Py_ssize_t;
        Ok(nonnull!(ffi!(PyBytes_FromStringAndSize(ptr, len))))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let size = seq.size_hint().unwrap() as pyo3::ffi::Py_ssize_t;
        let ptr = ffi!(PyTuple_New(size));
        let mut i = 0;
        while let Some(elem) = seq.next_element_seed(self)? {
            ffi!(PyTuple_SET_ITEM(
                ptr,
                i as pyo3::ffi::Py_ssize_t,
                elem.as_ptr()
            ));
            i += 1;
        }
        Ok(nonnull!(ptr))
    }
}
