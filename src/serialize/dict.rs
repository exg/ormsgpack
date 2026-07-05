// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::exc::*;
use crate::ffi::*;
use crate::opt::*;
use crate::serialize::default::DefaultHook;
use crate::serialize::serializer::*;
use crate::serialize::state::State;
use crate::util::unlikely;
use pyo3::ffi::{PyType_HasFeature, Py_TPFLAGS_DICT_SUBCLASS};
use pyo3::prelude::*;
use pyo3::sync::critical_section::with_critical_section;
use pyo3::types::{PyDict, PyType};
use serde::ser::{Serialize, SerializeMap, Serializer};
use smallvec::SmallVec;

pub struct Dict<'a, 'py> {
    obj: Borrowed<'a, 'py, PyDict>,
    state: &'a State,
    opts: Opt,
    default: &'a DefaultHook<'a, 'py>,
}

impl<'a, 'py> Dict<'a, 'py> {
    #[inline]
    pub fn matches_type(type_obj: Borrowed<'_, '_, PyType>) -> bool {
        unsafe { PyType_HasFeature(type_obj.as_type_ptr(), Py_TPFLAGS_DICT_SUBCLASS) != 0 }
    }

    pub fn new(
        obj: Borrowed<'a, 'py, PyDict>,
        state: &'a State,
        opts: Opt,
        default: &'a DefaultHook<'a, 'py>,
    ) -> Self {
        Dict {
            obj: obj,
            state: state,
            opts: opts,
            default: default,
        }
    }
}

impl Serialize for Dict<'_, '_> {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        with_critical_section(&self.obj, || {
            if unlikely(self.obj.len() == 0) {
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
        })
    }
}

impl<'a, 'py> Dict<'a, 'py> {
    #[inline(always)]
    fn serialize_with_str_keys<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let len = self.obj.len();
        let mut map = serializer.serialize_map(Some(len))?;
        for (key, value) in PyDictIter::from_pyobject(self.obj) {
            let Some(key) = cast_into_str(key) else {
                return Err(serde::ser::Error::custom(KEY_MUST_BE_STR));
            };
            let key_as_str = unicode_to_str(key).map_err(serde::ser::Error::custom)?;
            let pyvalue = PyObject::new(value, self.state, self.opts, self.default);
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
        let len = self.obj.len();
        let mut items: SmallVec<[(&str, Borrowed<'a, 'py, PyAny>); 8]> =
            SmallVec::with_capacity(len);
        for (key, value) in PyDictIter::from_pyobject(self.obj) {
            let Some(key) = cast_into_str(key) else {
                return Err(serde::ser::Error::custom(KEY_MUST_BE_STR));
            };
            let key_as_str = unicode_to_str(key).map_err(serde::ser::Error::custom)?;
            items.push((key_as_str, value));
        }

        items.sort_unstable_by(|a, b| a.0.cmp(b.0));

        let mut map = serializer.serialize_map(Some(len))?;
        for (key, val) in items.iter() {
            let pyvalue = PyObject::new(*val, self.state, self.opts, self.default);
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
        let len = self.obj.len();
        let mut map = serializer.serialize_map(Some(len))?;
        for (key, value) in PyDictIter::from_pyobject(self.obj) {
            if let Some(key) = cast_into_str(key) {
                let key_as_str = unicode_to_str(key).map_err(serde::ser::Error::custom)?;
                map.serialize_entry(
                    key_as_str,
                    &PyObject::new(value, self.state, self.opts, self.default),
                )?;
            } else {
                map.serialize_entry(
                    &DictKey::new(key, self.state, self.opts),
                    &PyObject::new(value, self.state, self.opts, self.default),
                )?;
            }
        }
        map.end()
    }
}
