// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::exc::*;
use crate::ffi::*;
use crate::opt::*;
use crate::serialize::serializer::*;
use crate::serialize::{Context, DictKeyContext};
use crate::util::unlikely;
use pyo3::prelude::*;
use pyo3::sync::critical_section::with_critical_section;
use pyo3::types::{PyDict, PyString};
use serde::ser::{Serialize, SerializeMap, Serializer};
use smallvec::SmallVec;

pub struct Dict<'a, 'py> {
    obj: Borrowed<'a, 'py, PyDict>,
    context: Context<'a, 'py>,
}

impl<'a, 'py> Dict<'a, 'py> {
    #[inline]
    pub fn try_new_exact(
        obj: BorrowedWithType<'a, 'py>,
        context: Context<'a, 'py>,
    ) -> Option<Self> {
        Some(Self {
            obj: obj.cast_exact::<PyDict>()?,
            context: context,
        })
    }

    #[inline]
    pub fn try_new(obj: BorrowedWithType<'a, 'py>, context: Context<'a, 'py>) -> Option<Self> {
        Some(Self {
            obj: obj.cast::<PyDict>()?,
            context: context,
        })
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
            } else if self.context.opts & (NON_STR_KEYS | SORT_KEYS) == 0 {
                self.serialize_with_str_keys(serializer)
            } else if self.context.opts & NON_STR_KEYS != 0 {
                if self.context.opts & SORT_KEYS != 0 {
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
        let mut iter = PyDictIter::new(self.obj);
        for _ in 0..len {
            let Some((key, value)) = iter.next() else {
                return Err(serde::ser::Error::custom(OBJECT_MODIFIED_DURING_ITERATION));
            };
            let Ok(key) = key.cast_exact::<PyString>() else {
                return Err(serde::ser::Error::custom(KEY_MUST_BE_STR));
            };
            let key_as_str =
                unicode_to_str(key.as_borrowed()).map_err(serde::ser::Error::custom)?;
            let pyvalue = PyObject::new(value.as_borrowed(), self.context);
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
        let mut items: SmallVec<
            [(
                PyCollectionItem<'a, 'py, PyString>,
                PyCollectionItem<'a, 'py, PyAny>,
            ); 8],
        > = SmallVec::with_capacity(len);
        let mut iter = PyDictIter::new(self.obj);
        for _ in 0..len {
            let Some((key, value)) = iter.next() else {
                return Err(serde::ser::Error::custom(OBJECT_MODIFIED_DURING_ITERATION));
            };
            let Ok(key) = key.cast_exact::<PyString>() else {
                return Err(serde::ser::Error::custom(KEY_MUST_BE_STR));
            };
            unicode_to_str(key.as_borrowed()).map_err(serde::ser::Error::custom)?;
            items.push((key, value));
        }

        items.sort_unstable_by(|a, b| {
            let a = unicode_to_str(a.0.as_borrowed()).unwrap();
            let b = unicode_to_str(b.0.as_borrowed()).unwrap();
            a.cmp(b)
        });

        let mut map = serializer.serialize_map(Some(len))?;
        for (key, val) in items.iter() {
            let key_as_str = unicode_to_str(key.as_borrowed()).unwrap();
            let pyvalue = PyObject::new(val.as_borrowed(), self.context);
            map.serialize_key(key_as_str).unwrap();
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
        let mut iter = PyDictIter::new(self.obj);
        for _ in 0..len {
            let Some((key, value)) = iter.next() else {
                return Err(serde::ser::Error::custom(OBJECT_MODIFIED_DURING_ITERATION));
            };
            match key.cast_exact::<PyString>() {
                Ok(key) => {
                    let key_as_str =
                        unicode_to_str(key.as_borrowed()).map_err(serde::ser::Error::custom)?;
                    map.serialize_entry(
                        key_as_str,
                        &PyObject::new(value.as_borrowed(), self.context),
                    )?;
                }
                Err(key) => {
                    map.serialize_entry(
                        &DictKey::new(
                            key.as_borrowed(),
                            DictKeyContext {
                                state: self.context.state,
                                opts: self.context.opts,
                            },
                        ),
                        &PyObject::new(value.as_borrowed(), self.context),
                    )?;
                }
            }
        }
        map.end()
    }
}
