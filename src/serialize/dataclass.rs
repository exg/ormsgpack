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
    pub field_type: *mut pyo3::ffi::PyObject,
    pub dataclass_fields_str: *mut pyo3::ffi::PyObject,
    pub field_type_str: *mut pyo3::ffi::PyObject,
}

impl State {
    #[cold]
    pub fn new() -> Self {
        unsafe {
            let module = pyo3::ffi::PyImport_ImportModule(c"dataclasses".as_ptr());
            let field_type = pyo3::ffi::PyObject_GetAttrString(module, c"_FIELD".as_ptr()).cast();
            pyo3::ffi::Py_DECREF(module);
            Self {
                field_type,
                dataclass_fields_str: pyo3::ffi::PyUnicode_InternFromString(
                    c"__dataclass_fields__".as_ptr(),
                ),
                field_type_str: pyo3::ffi::PyUnicode_InternFromString(c"_field_type".as_ptr()),
            }
        }
    }
}

#[inline]
fn has_slots(ob_type: *mut pyo3::ffi::PyTypeObject, state: &SerializeState) -> bool {
    unsafe {
        let tp_dict = (*ob_type).tp_dict;
        pyo3::ffi::PyDict_Contains(tp_dict, state.slots_str) == 1
    }
}

pub struct Dataclass<'a> {
    ptr: *mut pyo3::ffi::PyObject,
    state: &'a SerializeState,
    opts: Opt,
    default: &'a DefaultHook,
}

impl<'a> Dataclass<'a> {
    #[inline]
    pub fn try_new(
        obj: PyObjectWithType,
        state: &'a SerializeState,
        opts: Opt,
        default: &'a DefaultHook,
    ) -> Option<Self> {
        let tp_dict = unsafe { (*obj.get_type_ptr()).tp_dict };
        if !tp_dict.is_null()
            && unsafe {
                pyo3::ffi::PyDict_Contains(tp_dict, state.dataclass.dataclass_fields_str) == 1
            }
        {
            Some(Self {
                ptr: obj.as_ptr(),
                state,
                opts,
                default,
            })
        } else {
            None
        }
    }
}

fn is_pseudo_field(field: *mut pyo3::ffi::PyObject, state: &SerializeState) -> bool {
    let field_type = unsafe { pyobject_getattr(field, state.dataclass.field_type_str).unwrap() };
    field_type.as_ptr() != state.dataclass.field_type
}

impl Serialize for Dataclass<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let fields = unsafe {
            pyobject_getattr(self.ptr, self.state.dataclass.dataclass_fields_str).unwrap()
        };
        let len = unsafe { pydict_size(fields.as_ptr()) } as usize;
        if unlikely(len == 0) {
            return serializer.serialize_map(Some(0))?.end();
        }

        let maybe_dict = {
            let ob_type = ob_type!(self.ptr);
            if has_slots(ob_type, self.state) {
                None
            } else {
                Some(unsafe { pyobject_getattr(self.ptr, self.state.dict_str).unwrap() })
            }
        };

        let mut items: SmallVec<[(&str, OwnedPyObject); 8]> = SmallVec::with_capacity(len);
        for (attr, field) in PyDictIter::from_pyobject(fields.as_ptr()) {
            let key_as_str = unicode_to_str(attr.as_ptr()).map_err(serde::ser::Error::custom)?;
            if key_as_str.as_bytes()[0] == b'_' {
                continue;
            }

            if let Some(dict) = &maybe_dict {
                let mut value = std::ptr::null_mut();
                unsafe {
                    pyo3::ffi::compat::PyDict_GetItemRef(dict.as_ptr(), attr.as_ptr(), &mut value)
                };
                if let Some(ptr) = std::ptr::NonNull::new(value) {
                    items.push((key_as_str, OwnedPyObject::from_non_null(ptr)));
                } else if !is_pseudo_field(field.as_ptr(), self.state) {
                    let value = unsafe { pyobject_getattr(self.ptr, attr.as_ptr()).unwrap() };
                    items.push((key_as_str, value));
                }
            } else {
                if !is_pseudo_field(field.as_ptr(), self.state) {
                    let value = unsafe { pyobject_getattr(self.ptr, attr.as_ptr()).unwrap() };
                    items.push((key_as_str, value));
                }
            }
        }

        let mut map = serializer.serialize_map(Some(items.len()))?;
        for (key, value) in items.iter() {
            let pyvalue = PyObject::new(value.as_ptr(), self.state, self.opts, self.default);
            map.serialize_key(key).unwrap();
            map.serialize_value(&pyvalue)?
        }
        map.end()
    }
}
