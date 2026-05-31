// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::exc::*;
use crate::ffi::*;
use crate::opt::*;
use crate::serialize::default::DefaultHook;
use crate::serialize::serializer::*;
use crate::state::State;
use crate::util::unlikely;
use serde::ser::{Serialize, SerializeMap, Serializer};
use smallvec::SmallVec;

pub struct Dict<'a> {
    ptr: *mut pyo3::ffi::PyObject,
    state: *mut State,
    opts: Opt,
    default: &'a DefaultHook,
}

impl<'a> Dict<'a> {
    pub fn new(
        ptr: *mut pyo3::ffi::PyObject,
        state: *mut State,
        opts: Opt,
        default: &'a DefaultHook,
    ) -> Self {
        Dict {
            state: state,
            ptr: ptr,
            opts: opts,
            default: default,
        }
    }
}

impl Serialize for Dict<'_> {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut critical_section = CriticalSection::new();
        critical_section.begin(self.ptr);
        if unlikely(unsafe { pydict_size(self.ptr) } == 0) {
            serializer.serialize_map(Some(0))?.end()
        } else if self.opts & (NON_STR_KEYS | SORT_KEYS) == 0 {
            self.serialize_with_str_keys(serializer)
        } else if self.opts & NON_STR_KEYS != 0 {
            if self.opts & SORT_KEYS != 0 {
                return Err(serde::ser::Error::custom(
                    "OPT_NON_STR_KEYS is not compatible with OPT_SORT_KEYS",
                ));
            }
            self.serialize_with_non_str_keys(serializer)
        } else {
            self.serialize_with_sorted_str_keys(serializer)
        }
    }
}

impl Dict<'_> {
    #[inline(always)]
    fn serialize_with_str_keys<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let len = unsafe { pydict_size(self.ptr) } as usize;
        let mut map = serializer.serialize_map(Some(len))?;
        let mut iter = PyDictIter::from_pyobject(self.ptr);
        for _ in 0..len {
            let Some((key, value)) = iter.next() else {
                return Err(serde::ser::Error::custom(
                    "Object modified during iteration",
                ));
            };
            if unlikely(ob_type!(key.as_ptr()) != &raw mut pyo3::ffi::PyUnicode_Type) {
                return Err(serde::ser::Error::custom(KEY_MUST_BE_STR));
            }
            let key_as_str = unicode_to_str(key.as_ptr()).map_err(serde::ser::Error::custom)?;
            let pyvalue = PyObject::new(value.as_ptr(), self.state, self.opts, self.default);
            map.serialize_key(key_as_str).unwrap();
            map.serialize_value(&pyvalue)?;
        }
        map.end()
    }

    #[inline(never)]
    fn serialize_with_sorted_str_keys<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let len = unsafe { pydict_size(self.ptr) } as usize;
        let mut items: SmallVec<[(&str, OwnedPyObject, OwnedPyObject); 8]> =
            SmallVec::with_capacity(len);
        let mut iter = PyDictIter::from_pyobject(self.ptr);
        for _ in 0..len {
            let Some((key, value)) = iter.next() else {
                return Err(serde::ser::Error::custom(
                    "Object modified during iteration",
                ));
            };
            if unlikely(ob_type!(key.as_ptr()) != &raw mut pyo3::ffi::PyUnicode_Type) {
                return Err(serde::ser::Error::custom(KEY_MUST_BE_STR));
            }
            let key_as_str = unicode_to_str(key.as_ptr()).map_err(serde::ser::Error::custom)?;
            items.push((key_as_str, key, value));
        }

        items.sort_unstable_by(|a, b| a.0.cmp(b.0));

        let mut map = serializer.serialize_map(Some(len))?;
        for (key, _, value) in items.iter() {
            let pyvalue = PyObject::new(value.as_ptr(), self.state, self.opts, self.default);
            map.serialize_key(key).unwrap();
            map.serialize_value(&pyvalue)?;
        }
        map.end()
    }

    #[inline(never)]
    fn serialize_with_non_str_keys<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let len = unsafe { pydict_size(self.ptr) } as usize;
        let mut map = serializer.serialize_map(Some(len))?;
        let mut iter = PyDictIter::from_pyobject(self.ptr);
        for _ in 0..len {
            let Some((key, value)) = iter.next() else {
                return Err(serde::ser::Error::custom(
                    "Object modified during iteration",
                ));
            };
            if ob_type!(key.as_ptr()) == &raw mut pyo3::ffi::PyUnicode_Type {
                let key_as_str = unicode_to_str(key.as_ptr()).map_err(serde::ser::Error::custom)?;
                map.serialize_entry(
                    key_as_str,
                    &PyObject::new(value.as_ptr(), self.state, self.opts, self.default),
                )?;
            } else {
                map.serialize_entry(
                    &DictKey::new(key.as_ptr(), self.state, self.opts),
                    &PyObject::new(value.as_ptr(), self.state, self.opts, self.default),
                )?;
            }
        }
        map.end()
    }
}
