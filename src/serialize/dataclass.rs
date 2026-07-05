// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::exc::*;
use crate::ffi::*;
use crate::opt::*;
use crate::serialize::default::DefaultHook;
use crate::serialize::serializer::*;
use crate::serialize::state::State as RootState;
use crate::util::unlikely;

use pyo3::prelude::*;
use pyo3::types::{PyString, PyType};
use serde::ser::{Serialize, SerializeMap, Serializer};

use smallvec::SmallVec;

pub struct State {
    dataclass_field_type: Py<PyAny>,
    dataclass_fields_str: Py<PyString>,
    field_type_str: Py<PyString>,
}

impl State {
    #[cold]
    pub fn new(py: Python<'_>) -> PyResult<Self> {
        Ok(Self {
            dataclass_field_type: py.import("dataclasses")?.getattr("_FIELD")?.unbind(),
            dataclass_fields_str: PyString::intern(py, "__dataclass_fields__").unbind(),
            field_type_str: PyString::intern(py, "_field_type").unbind(),
        })
    }
}

#[inline]
fn has_slots(type_obj: Borrowed<'_, '_, PyType>, state: &RootState) -> bool {
    get_type_dict(type_obj).is_some_and(|v| v.contains(&state.slots_str).unwrap())
}

pub struct Dataclass<'a, 'py> {
    obj: Borrowed<'a, 'py, PyAny>,
    state: &'a RootState,
    opts: Opt,
    default: &'a DefaultHook<'a, 'py>,
}

impl<'a, 'py> Dataclass<'a, 'py> {
    #[inline]
    pub fn matches_type(type_obj: Borrowed<'_, '_, PyType>, state: &State) -> bool {
        get_type_dict(type_obj).is_some_and(|v| v.contains(&state.dataclass_fields_str).unwrap())
    }

    pub fn new(
        obj: Borrowed<'a, 'py, PyAny>,
        state: &'a RootState,
        opts: Opt,
        default: &'a DefaultHook<'a, 'py>,
    ) -> Self {
        Dataclass {
            obj: obj,
            state: state,
            opts: opts,
            default: default,
        }
    }
}

fn is_pseudo_field(field: Borrowed<'_, '_, PyAny>, state: &State) -> PyResult<bool> {
    let field_type = field.getattr(state.field_type_str.bind_borrowed(field.py()))?;
    Ok(!field_type.is(&state.dataclass_field_type))
}

impl Serialize for Dataclass<'_, '_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let state = &self.state.dataclass_state;
        let Some(fields) = self
            .obj
            .getattr(state.dataclass_fields_str.bind_borrowed(self.obj.py()))
            .map(cast_into_dict)
            .map_err(serde::ser::Error::custom)?
        else {
            return Err(serde::ser::Error::custom(
                "__dataclass_fields__ must be a dict",
            ));
        };
        let len = fields.len();
        if unlikely(len == 0) {
            return serializer.serialize_map(Some(0))?.end();
        }

        let maybe_dict = {
            let type_obj = get_type(self.obj);
            if has_slots(type_obj, self.state) {
                None
            } else {
                let Some(dict) = self
                    .obj
                    .getattr(self.state.dict_str.bind_borrowed(self.obj.py()))
                    .map(cast_into_dict)
                    .map_err(serde::ser::Error::custom)?
                else {
                    return Err(serde::ser::Error::custom(
                        "__dict__ attribute must be a dict",
                    ));
                };
                Some(dict)
            }
        };

        let mut items: SmallVec<[(&str, Bound<'_, PyAny>); 8]> = SmallVec::with_capacity(len);
        for (attr, field) in PyDictIter::from_pyobject(fields.as_borrowed()) {
            let Some(attr) = cast_into_str(attr) else {
                return Err(serde::ser::Error::custom(KEY_MUST_BE_STR));
            };
            let key_as_str = unicode_to_str(attr).map_err(serde::ser::Error::custom)?;
            if key_as_str.as_bytes()[0] == b'_' {
                continue;
            }

            if let Some(dict) = &maybe_dict {
                if let Some(item) = dict.get_item(attr).map_err(serde::ser::Error::custom)? {
                    items.push((key_as_str, item));
                } else if !is_pseudo_field(field, state).map_err(serde::ser::Error::custom)? {
                    let value = self.obj.getattr(attr).map_err(serde::ser::Error::custom)?;
                    items.push((key_as_str, value));
                }
            } else {
                if !is_pseudo_field(field, state).map_err(serde::ser::Error::custom)? {
                    let value = self.obj.getattr(attr).map_err(serde::ser::Error::custom)?;
                    items.push((key_as_str, value));
                }
            }
        }

        let mut map = serializer.serialize_map(Some(items.len()))?;
        for (key, value) in items.iter() {
            let pyvalue = PyObject::new(value.as_borrowed(), self.state, self.opts, self.default);
            map.serialize_key(key).unwrap();
            map.serialize_value(&pyvalue)?
        }
        map.end()
    }
}
