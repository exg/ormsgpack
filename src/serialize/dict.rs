// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::exc::*;
use crate::ffi::*;
use crate::io::WriteSlices;
use crate::opt::*;
use crate::serialize::Error;
use crate::serialize::{Serialize, Serializer};
use crate::util::unlikely;
use pyo3::prelude::*;
use pyo3::sync::critical_section::with_critical_section;
use pyo3::types::{PyDict, PyString};
use smallvec::SmallVec;

pub struct Dict<'a, 'py> {
    obj: Borrowed<'a, 'py, PyDict>,
    opts: Opt,
}

impl<'a, 'py> Dict<'a, 'py> {
    #[inline]
    pub fn try_new_exact(obj: BorrowedWithType<'a, 'py>, opts: Opt) -> Option<Self> {
        Some(Self {
            obj: obj.cast_exact::<PyDict>()?,
            opts: opts,
        })
    }

    #[inline]
    pub fn try_new(obj: BorrowedWithType<'a, 'py>, opts: Opt) -> Option<Self> {
        Some(Self {
            obj: obj.cast::<PyDict>()?,
            opts: opts,
        })
    }
}

impl Serialize for Dict<'_, '_> {
    #[inline]
    fn serialize_as<const KEY: bool, W>(
        &self,
        serializer: &mut Serializer<'_, '_, W>,
    ) -> Result<(), Error>
    where
        W: WriteSlices,
    {
        with_critical_section(&self.obj, || {
            if unlikely(self.obj.len() == 0) {
                serializer.serialize_map(0, |_| Ok(()))
            } else if self.opts & (NON_STR_KEYS | SORT_KEYS) == 0 {
                self.serialize_with_str_keys(serializer)
            } else if self.opts & NON_STR_KEYS != 0 {
                if self.opts & SORT_KEYS != 0 {
                    return Err(Error::custom(
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
    fn serialize_with_str_keys<W>(
        &self,
        serializer: &mut Serializer<'_, '_, W>,
    ) -> Result<(), Error>
    where
        W: WriteSlices,
    {
        let len = self.obj.len();
        serializer.serialize_map(len, |map| {
            let mut iter = PyDictIter::new(self.obj);
            for _ in 0..len {
                let Some((key, value)) = iter.next() else {
                    return Err(Error::custom(OBJECT_MODIFIED_DURING_ITERATION));
                };
                let Ok(key) = key.cast_exact::<PyString>() else {
                    return Err(Error::custom(KEY_MUST_BE_STR));
                };
                let key_as_str = unicode_to_str(key.as_borrowed()).map_err(Error::custom)?;
                map.serialize_entry(key_as_str, &value.as_borrowed())?;
            }
            Ok(())
        })
    }

    #[inline(never)]
    fn serialize_with_sorted_str_keys<W>(
        &self,
        serializer: &mut Serializer<'_, '_, W>,
    ) -> Result<(), Error>
    where
        W: WriteSlices,
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
                return Err(Error::custom(OBJECT_MODIFIED_DURING_ITERATION));
            };
            let Ok(key) = key.cast_exact::<PyString>() else {
                return Err(Error::custom(KEY_MUST_BE_STR));
            };
            unicode_to_str(key.as_borrowed()).map_err(Error::custom)?;
            items.push((key, value));
        }

        items.sort_unstable_by(|a, b| {
            let a = unicode_to_str(a.0.as_borrowed()).unwrap();
            let b = unicode_to_str(b.0.as_borrowed()).unwrap();
            a.cmp(b)
        });

        serializer.serialize_map(len, |map| {
            for (key, val) in items.iter() {
                let key_as_str = unicode_to_str(key.as_borrowed()).unwrap();
                map.serialize_entry(key_as_str, &val.as_borrowed())?;
            }
            Ok(())
        })
    }

    #[inline(never)]
    fn serialize_with_non_str_keys<W>(
        &self,
        serializer: &mut Serializer<'_, '_, W>,
    ) -> Result<(), Error>
    where
        W: WriteSlices,
    {
        let len = self.obj.len();
        serializer.serialize_map(len, |map| {
            let mut iter = PyDictIter::new(self.obj);
            for _ in 0..len {
                let Some((key, value)) = iter.next() else {
                    return Err(Error::custom(OBJECT_MODIFIED_DURING_ITERATION));
                };
                match key.cast_exact::<PyString>() {
                    Ok(key) => {
                        let key_as_str =
                            unicode_to_str(key.as_borrowed()).map_err(Error::custom)?;
                        map.serialize_entry(key_as_str, &value.as_borrowed())?;
                    }
                    Err(key) => {
                        map.serialize_entry(&key.as_borrowed(), &value.as_borrowed())?;
                    }
                }
            }
            Ok(())
        })
    }
}
