// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::exc::*;
use crate::ffi::*;
use crate::opt::*;
use crate::util::unlikely;

use crate::io::WriteSlices;
use crate::serialize::Error;
use crate::serialize::{Serialize, Serializer};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString};

use smallvec::SmallVec;

pub struct State {
    fields_str: Py<PyString>,
    dict_str: Py<PyString>,
    pydantic_extra_str: Py<PyString>,
    pydantic_validator_str: Py<PyString>,
}

impl State {
    #[cold]
    pub fn new(py: Python<'_>) -> Self {
        Self {
            dict_str: PyString::intern(py, "__dict__").unbind(),
            fields_str: PyString::intern(py, "__fields__").unbind(),
            pydantic_extra_str: PyString::intern(py, "__pydantic_extra__").unbind(),
            pydantic_validator_str: PyString::intern(py, "__pydantic_validator__").unbind(),
        }
    }
}

pub struct PydanticModel<'a, 'py> {
    obj: Borrowed<'a, 'py, PyAny>,
    state: &'a State,
    opts: Opt,
}

impl<'a, 'py> PydanticModel<'a, 'py> {
    #[inline]
    pub fn try_new(obj: BorrowedWithType<'a, 'py>, state: &'a State, opts: Opt) -> Option<Self> {
        if get_type_dict(obj.get_type()).is_some_and(|v| {
            let fields_str = state.fields_str.bind_borrowed(obj.py());
            let pydantic_validator_str = state.pydantic_validator_str.bind_borrowed(obj.py());
            v.contains(fields_str).unwrap() || v.contains(pydantic_validator_str).unwrap()
        }) {
            Some(Self {
                obj: obj.as_borrowed(),
                state: state,
                opts: opts,
            })
        } else {
            None
        }
    }
}

impl Serialize for PydanticModel<'_, '_> {
    fn serialize_as<const KEY: bool, W>(
        &self,
        serializer: &mut Serializer<'_, '_, W>,
    ) -> Result<(), Error>
    where
        W: WriteSlices,
    {
        let state = self.state;
        let Some(dict) = self
            .obj
            .getattr(state.dict_str.bind_borrowed(self.obj.py()))
            .map(cast_into_exact::<PyDict>)?
        else {
            return Err(Error::custom("__dict__ attribute must be a dict"));
        };

        if let Some(extra_dict) = self
            .obj
            .getattr_opt(state.pydantic_extra_str.bind_borrowed(self.obj.py()))?
            .and_then(cast_into_exact::<PyDict>)
        {
            self.serialize_with_extra(serializer, dict.as_borrowed(), extra_dict.as_borrowed())
        } else {
            self.serialize_with_no_extra(serializer, dict.as_borrowed())
        }
    }
}

impl<'a, 'py> PydanticModel<'a, 'py> {
    fn serialize_with_no_extra<W>(
        &self,
        serializer: &mut Serializer<'_, '_, W>,
        dict: Borrowed<'a, 'py, PyDict>,
    ) -> Result<(), Error>
    where
        W: WriteSlices,
    {
        let len = dict.len();
        if unlikely(len == 0) {
            return serializer.serialize_map(0, |_| Ok(()));
        }
        let mut items: SmallVec<
            [(
                PyCollectionItem<'a, 'py, PyString>,
                PyCollectionItem<'a, 'py, PyAny>,
            ); 8],
        > = SmallVec::with_capacity(len);
        let mut iter = PyDictIter::new(dict);
        for _ in 0..len {
            let Some((key, value)) = iter.next() else {
                return Err(Error::custom(OBJECT_MODIFIED_DURING_ITERATION));
            };
            let Ok(key) = key.cast_exact::<PyString>() else {
                return Err(Error::custom(KEY_MUST_BE_STR));
            };
            let key_as_str = unicode_to_str(key.as_borrowed()).map_err(Error::custom)?;
            if unlikely(key_as_str.as_bytes()[0] == b'_') {
                continue;
            }
            items.push((key, value));
        }

        if self.opts & SORT_KEYS != 0 {
            items.sort_unstable_by(|a, b| {
                let a = unicode_to_str(a.0.as_borrowed()).unwrap();
                let b = unicode_to_str(b.0.as_borrowed()).unwrap();
                a.cmp(b)
            });
        }

        serializer.serialize_map(items.len(), |map| {
            for (key, value) in items.iter() {
                let key_as_str = unicode_to_str(key.as_borrowed()).unwrap();
                map.serialize_entry(key_as_str, &value.as_borrowed())?;
            }
            Ok(())
        })
    }

    fn serialize_with_extra<W>(
        &self,
        serializer: &mut Serializer<'_, '_, W>,
        dict: Borrowed<'a, 'py, PyDict>,
        extra_dict: Borrowed<'a, 'py, PyDict>,
    ) -> Result<(), Error>
    where
        W: WriteSlices,
    {
        let dict_len = dict.len();
        let extra_dict_len = extra_dict.len();
        let len = dict_len + extra_dict_len;
        if unlikely(len == 0) {
            return serializer.serialize_map(0, |_| Ok(()));
        }
        let mut items: SmallVec<
            [(
                PyCollectionItem<'a, 'py, PyString>,
                PyCollectionItem<'a, 'py, PyAny>,
            ); 8],
        > = SmallVec::with_capacity(len);
        for (dict, len) in [(dict, dict_len), (extra_dict, extra_dict_len)] {
            let mut iter = PyDictIter::new(dict);
            for _ in 0..len {
                let Some((key, value)) = iter.next() else {
                    return Err(Error::custom(OBJECT_MODIFIED_DURING_ITERATION));
                };
                let Ok(key) = key.cast_exact::<PyString>() else {
                    return Err(Error::custom(KEY_MUST_BE_STR));
                };
                let key_as_str = unicode_to_str(key.as_borrowed()).map_err(Error::custom)?;
                if unlikely(key_as_str.as_bytes()[0] == b'_') {
                    continue;
                }
                items.push((key, value));
            }
        }

        if self.opts & SORT_KEYS != 0 {
            items.sort_unstable_by(|a, b| {
                let a = unicode_to_str(a.0.as_borrowed()).unwrap();
                let b = unicode_to_str(b.0.as_borrowed()).unwrap();
                a.cmp(b)
            });
        }

        serializer.serialize_map(items.len(), |map| {
            for (key, value) in items.iter() {
                let key_as_str = unicode_to_str(key.as_borrowed()).unwrap();
                map.serialize_entry(key_as_str, &value.as_borrowed())?;
            }
            Ok(())
        })
    }
}
