// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::OwnedPyObject;
use crate::{deserialize, serialize};

pub struct State {
    pub serialize: serialize::State,
    pub deserialize: deserialize::State,
    pub default_str: OwnedPyObject,
    pub ext_hook_str: OwnedPyObject,
    pub option_str: OwnedPyObject,
}

impl State {
    #[cold]
    pub fn new() -> Option<Self> {
        Some(Self {
            serialize: serialize::State::new()?,
            deserialize: deserialize::State::new()?,
            default_str: OwnedPyObject::try_intern(c"default")?,
            ext_hook_str: OwnedPyObject::try_intern(c"ext_hook")?,
            option_str: OwnedPyObject::try_intern(c"option")?,
        })
    }
}
