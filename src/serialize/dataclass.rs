// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::*;
use crate::opt::*;
use crate::serialize::default::DefaultHook;
use crate::serialize::serializer::*;
use crate::serialize::State as SerializeState;
use crate::util::unlikely;

use serde::ser::{Serialize, SerializeMap, Serializer};

use smallvec::SmallVec;

pub struct State {
    pub field_type: OwnedPyObject,
    pub dataclass_fields_str: OwnedPyObject,
    pub field_type_str: OwnedPyObject,
}

impl State {
    #[cold]
    pub fn new() -> Option<Self> {
        let module = OwnedPyObject::try_import(c"dataclasses")?;
        Some(Self {
            field_type: module.getattr_string(c"_FIELD")?,
            dataclass_fields_str: OwnedPyObject::try_intern(c"__dataclass_fields__")?,
            field_type_str: OwnedPyObject::try_intern(c"_field_type")?,
        })
    }
}

#[inline]
fn has_slots(ob_type: *mut pyo3::ffi::PyTypeObject, state: &SerializeState) -> bool {
    unsafe {
        let tp_dict = (*ob_type).tp_dict;
        pyo3::ffi::PyDict_Contains(tp_dict, state.slots_str.as_ptr()) == 1
    }
}

pub struct Dataclass<'a> {
    obj: BorrowedPyObject<'a>,
    state: &'a SerializeState,
    opts: Opt,
    default: &'a DefaultHook<'a>,
}

impl<'a> Dataclass<'a> {
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
                pyo3::ffi::PyDict_Contains(tp_dict, state.dataclass.dataclass_fields_str.as_ptr())
                    == 1
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

fn is_pseudo_field(field: BorrowedPyObject<'_>, state: &SerializeState) -> bool {
    let field_type = field
        .getattr(state.dataclass.field_type_str.as_borrowed())
        .unwrap();
    field_type.as_ptr() != state.dataclass.field_type.as_ptr()
}

impl Serialize for Dataclass<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let fields = self
            .obj
            .getattr(self.state.dataclass.dataclass_fields_str.as_borrowed())
            .unwrap();
        let mut critical_section = CriticalSection::new();
        critical_section.begin(fields.as_ptr());
        let len = unsafe { pydict_size(fields.as_ptr()) } as usize;
        if unlikely(len == 0) {
            return serializer.serialize_map(Some(0))?.end();
        }

        let maybe_dict = {
            let ob_type = unsafe { pyo3::ffi::Py_TYPE(self.obj.as_ptr()) };
            if has_slots(ob_type, self.state) {
                None
            } else {
                Some(self.obj.getattr(self.state.dict_str.as_borrowed()).unwrap())
            }
        };

        let mut items: SmallVec<[(&str, OwnedPyObject, OwnedPyObject); 8]> =
            SmallVec::with_capacity(len);
        let mut iter = PyDictIter::from_pyobject(fields.as_borrowed());
        for _ in 0..len {
            let Some((attr, field)) = iter.next() else {
                return Err(serde::ser::Error::custom(
                    "Object modified during iteration",
                ));
            };

            let key_as_str = unicode_to_str(attr.as_ptr()).map_err(serde::ser::Error::custom)?;
            if key_as_str.as_bytes()[0] == b'_' {
                continue;
            }

            if let Some(dict) = &maybe_dict {
                let mut value = std::ptr::null_mut();
                unsafe {
                    pyo3::ffi::compat::PyDict_GetItemRef(dict.as_ptr(), attr.as_ptr(), &mut value)
                };
                if let Some(value) = unsafe { OwnedPyObject::from_owned_ptr_or_opt(value) } {
                    items.push((key_as_str, attr, value));
                } else if !is_pseudo_field(field.as_borrowed(), self.state) {
                    let value = self.obj.getattr(attr.as_borrowed()).unwrap();
                    items.push((key_as_str, attr, value));
                }
            } else {
                if !is_pseudo_field(field.as_borrowed(), self.state) {
                    let value = self.obj.getattr(attr.as_borrowed()).unwrap();
                    items.push((key_as_str, attr, value));
                }
            }
        }

        let mut map = serializer.serialize_map(Some(items.len()))?;
        for (key, _, value) in items.iter() {
            let pyvalue = PyObject::new(value.as_borrowed(), self.state, self.opts, self.default);
            map.serialize_key(key).unwrap();
            map.serialize_value(&pyvalue)?
        }
        map.end()
    }
}
