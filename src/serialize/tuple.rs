// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::opt::*;
use crate::serialize::default::DefaultHook;
use crate::serialize::serializer::*;
use crate::serialize::state::State;

use pyo3::prelude::*;
use pyo3::types::PyTuple;
use serde::ser::{Serialize, SerializeSeq, Serializer};

pub struct Tuple<'a, 'py> {
    obj: Borrowed<'a, 'py, PyTuple>,
    state: &'a State,
    opts: Opt,
    default: &'a DefaultHook<'a, 'py>,
}

impl<'a, 'py> Tuple<'a, 'py> {
    pub fn new(
        obj: Borrowed<'a, 'py, PyTuple>,
        state: &'a State,
        opts: Opt,
        default: &'a DefaultHook<'a, 'py>,
    ) -> Self {
        Tuple {
            obj: obj,
            state: state,
            opts: opts,
            default: default,
        }
    }
}

impl Serialize for Tuple<'_, '_> {
    #[inline(never)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let len = self.obj.len();
        let mut seq = serializer.serialize_seq(Some(len))?;
        for item in self.obj.iter_borrowed() {
            let value = PyObject::new(item, self.state, self.opts, self.default);
            seq.serialize_element(&value)?;
        }
        seq.end()
    }
}

pub struct TupleDictKey<'a, 'py> {
    obj: Borrowed<'a, 'py, PyTuple>,
    state: &'a State,
    opts: Opt,
}

impl<'a, 'py> TupleDictKey<'a, 'py> {
    pub fn new(obj: Borrowed<'a, 'py, PyTuple>, state: &'a State, opts: Opt) -> Self {
        TupleDictKey {
            obj: obj,
            state: state,
            opts: opts,
        }
    }
}

impl Serialize for TupleDictKey<'_, '_> {
    #[inline(never)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let len = self.obj.len();
        let mut seq = serializer.serialize_seq(Some(len))?;
        for item in self.obj.iter_borrowed() {
            let value = DictKey::new(item, self.state, self.opts);
            seq.serialize_element(&value)?;
        }
        seq.end()
    }
}
