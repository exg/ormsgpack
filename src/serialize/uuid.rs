// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::BorrowedWithType;
use crate::serialize::pyerr_to_serde;
use pyo3::prelude::*;
use pyo3::types::PyString;
use serde::ser::{Serialize, Serializer};

pub struct State {
    type_object: Py<PyAny>,
    int_str: Py<PyString>,
}

impl State {
    #[cold]
    pub fn new(py: Python<'_>) -> PyResult<Self> {
        Ok(Self {
            type_object: py.import("uuid")?.getattr("UUID")?.unbind(),
            int_str: PyString::intern(py, "int").unbind(),
        })
    }
}

pub struct UUID<'a, 'py> {
    obj: Borrowed<'a, 'py, PyAny>,
    state: &'a State,
}

const HEX: [u8; 16] = *b"0123456789abcdef";

fn write_group<W>(writer: &mut W, group: &[u8]) -> Result<(), std::io::Error>
where
    W: std::io::Write,
{
    for i in 0..group.len() {
        writer.write_all(&[
            HEX[(group[i] >> 4) as usize],
            HEX[(group[i] & 0x0f) as usize],
        ])?;
    }
    Ok(())
}

fn write_uuid<W>(writer: &mut W, value: &[u8; 16]) -> Result<(), std::io::Error>
where
    W: std::io::Write,
{
    write_group(writer, &value[..4])?;
    writer.write_all(b"-")?;
    write_group(writer, &value[4..6])?;
    writer.write_all(b"-")?;
    write_group(writer, &value[6..8])?;
    writer.write_all(b"-")?;
    write_group(writer, &value[8..10])?;
    writer.write_all(b"-")?;
    write_group(writer, &value[10..])?;
    Ok(())
}

impl<'a, 'py> UUID<'a, 'py> {
    #[inline]
    pub fn try_new(obj: BorrowedWithType<'a, 'py>, state: &'a State) -> Option<Self> {
        if obj.get_type_ptr() == state.type_object.as_ptr().cast() {
            Some(Self {
                obj: obj.as_borrowed(),
                state: state,
            })
        } else {
            None
        }
    }

    fn get_value(&self) -> PyResult<[u8; 16]> {
        let value: u128 = self
            .obj
            .getattr(self.state.int_str.bind_borrowed(self.obj.py()))?
            .extract()?;
        Ok(value.to_be_bytes())
    }
}

impl Serialize for UUID<'_, '_> {
    #[inline(never)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = self
            .get_value()
            .map_err(|e| pyerr_to_serde(self.obj.py(), e))?;
        let mut cursor = std::io::Cursor::new([0u8; 64]);
        write_uuid(&mut cursor, &value).unwrap();
        let len = cursor.position() as usize;
        let value = unsafe { std::str::from_utf8_unchecked(&cursor.get_ref()[0..len]) };
        serializer.serialize_str(value)
    }
}
