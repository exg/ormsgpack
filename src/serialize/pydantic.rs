// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::exc::*;
use crate::ffi::*;
use crate::opt::*;
use crate::serialize::default::DefaultHook;
use crate::serialize::serializer::*;
use crate::serialize::State as SerializeState;
use crate::util::unlikely;

use serde::ser::{Serialize, SerializeMap, Serializer};

use smallvec::SmallVec;

pub struct State {
    pub fields_str: OwnedPyObject,
    pub pydantic_extra_str: OwnedPyObject,
    pub pydantic_validator_str: OwnedPyObject,
}

impl State {
    #[cold]
    pub fn new() -> Option<Self> {
        Some(Self {
            fields_str: OwnedPyObject::try_intern(c"__fields__")?,
            pydantic_extra_str: OwnedPyObject::try_intern(c"__pydantic_extra__")?,
            pydantic_validator_str: OwnedPyObject::try_intern(c"__pydantic_validator__")?,
        })
    }
}

pub struct PydanticModel<'a> {
    obj: BorrowedPyObject<'a>,
    state: &'a SerializeState,
    opts: Opt,
    default: &'a DefaultHook<'a>,
}

impl<'a> PydanticModel<'a> {
    #[inline]
    pub fn try_new(
        obj: PyObjectWithType<'a>,
        state: &'a SerializeState,
        opts: Opt,
        default: &'a DefaultHook<'a>,
    ) -> Option<Self> {
        let tp_dict = unsafe { (*obj.get_type_ptr()).tp_dict };
        if !tp_dict.is_null()
            && unsafe {
                pyo3::ffi::PyDict_Contains(tp_dict, state.pydantic.fields_str.as_ptr()) == 1
                    || pyo3::ffi::PyDict_Contains(
                        tp_dict,
                        state.pydantic.pydantic_validator_str.as_ptr(),
                    ) == 1
            }
        {
            Some(Self {
                obj: obj.as_borrowed(),
                state,
                opts,
                default,
            })
        } else {
            None
        }
    }
}

impl Serialize for PydanticModel<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let maybe_dict = self.obj.getattr(self.state.dict_str.as_borrowed());
        let Some(dict) = maybe_dict else {
            return Err(serde::ser::Error::custom(
                "Pydantic model must have __dict__ attribute",
            ));
        };

        let maybe_extra_dict = self
            .obj
            .getattr(self.state.pydantic.pydantic_extra_str.as_borrowed());
        if let Some(extra_dict) = maybe_extra_dict {
            let ob_type = unsafe { pyo3::ffi::Py_TYPE(extra_dict.as_ptr()) };
            if ob_type == &raw mut pyo3::ffi::PyDict_Type {
                self.serialize_with_extra(serializer, dict.as_borrowed(), extra_dict.as_borrowed())
            } else {
                self.serialize_with_no_extra(serializer, dict.as_borrowed())
            }
        } else {
            self.serialize_with_no_extra(serializer, dict.as_borrowed())
        }
    }
}

impl PydanticModel<'_> {
    fn serialize_with_no_extra<S>(
        &self,
        serializer: S,
        dict: BorrowedPyObject<'_>,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut critical_section = CriticalSection::new();
        critical_section.begin(dict.as_ptr());
        let len = unsafe { pydict_size(dict.as_ptr()) } as usize;
        if unlikely(len == 0) {
            return serializer.serialize_map(Some(0))?.end();
        }
        let mut items: SmallVec<[(&str, OwnedPyObject, OwnedPyObject); 8]> =
            SmallVec::with_capacity(len);
        let mut iter = PyDictIter::from_pyobject(dict);
        for _ in 0..len {
            let Some((key, value)) = iter.next() else {
                return Err(serde::ser::Error::custom(
                    "Object modified during iteration",
                ));
            };

            let ob_type = unsafe { pyo3::ffi::Py_TYPE(key.as_ptr()) };
            if unlikely(ob_type != &raw mut pyo3::ffi::PyUnicode_Type) {
                return Err(serde::ser::Error::custom(KEY_MUST_BE_STR));
            }
            let key_as_str = unicode_to_str(key.as_ptr()).map_err(serde::ser::Error::custom)?;
            if unlikely(key_as_str.as_bytes()[0] == b'_') {
                continue;
            }
            items.push((key_as_str, key, value));
        }

        if self.opts & SORT_KEYS != 0 {
            items.sort_unstable_by(|a, b| a.0.cmp(b.0));
        }

        let mut map = serializer.serialize_map(Some(items.len()))?;
        for (key, _, value) in items.iter() {
            let pyvalue = PyObject::new(value.as_borrowed(), self.state, self.opts, self.default);
            map.serialize_key(key).unwrap();
            map.serialize_value(&pyvalue)?;
        }
        map.end()
    }

    fn serialize_with_extra<S>(
        &self,
        serializer: S,
        dict: BorrowedPyObject<'_>,
        extra_dict: BorrowedPyObject<'_>,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut critical_section = CriticalSection2::new();
        critical_section.begin(dict.as_ptr(), extra_dict.as_ptr());
        let mut iter = PyDictIter::from_pyobject(dict).chain(PyDictIter::from_pyobject(extra_dict));
        let len = iter.size_hint().0;
        if unlikely(len == 0) {
            return serializer.serialize_map(Some(0))?.end();
        }
        let mut items: SmallVec<[(&str, OwnedPyObject, OwnedPyObject); 8]> =
            SmallVec::with_capacity(len);
        for _ in 0..len {
            let Some((key, value)) = iter.next() else {
                return Err(serde::ser::Error::custom(
                    "Object modified during iteration",
                ));
            };

            let ob_type = unsafe { pyo3::ffi::Py_TYPE(key.as_ptr()) };
            if unlikely(ob_type != &raw mut pyo3::ffi::PyUnicode_Type) {
                return Err(serde::ser::Error::custom(KEY_MUST_BE_STR));
            }
            let key_as_str = unicode_to_str(key.as_ptr()).map_err(serde::ser::Error::custom)?;
            if unlikely(key_as_str.as_bytes()[0] == b'_') {
                continue;
            }
            items.push((key_as_str, key, value));
        }

        if self.opts & SORT_KEYS != 0 {
            items.sort_unstable_by(|a, b| a.0.cmp(b.0));
        }

        let mut map = serializer.serialize_map(Some(items.len()))?;
        for (key, _, value) in items.iter() {
            let pyvalue = PyObject::new(value.as_borrowed(), self.state, self.opts, self.default);
            map.serialize_key(key).unwrap();
            map.serialize_value(&pyvalue)?;
        }
        map.end()
    }
}
