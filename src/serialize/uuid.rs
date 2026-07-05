// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use pyo3::prelude::*;
use pyo3::types::{PyString, PyType};
use serde::ser::{Serialize, Serializer};

pub struct State {
    uuid_type: Py<PyAny>,
    int_str: Py<PyString>,
}

impl State {
    #[cold]
    pub fn new(py: Python<'_>) -> PyResult<Self> {
        Ok(Self {
            uuid_type: py.import("uuid")?.getattr("UUID")?.unbind(),
            int_str: PyString::intern(py, "int").unbind(),
        })
    }
}

pub struct UUID<'a, 'py> {
    obj: Borrowed<'a, 'py, PyAny>,
    state: &'a State,
}

const HEX: [u8; 16] = [
    b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'a', b'b', b'c', b'd', b'e', b'f',
];

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

impl<'a, 'py> UUID<'a, 'py> {
    #[inline]
    pub fn matches_exact_type(type_obj: Borrowed<'_, '_, PyType>, state: &State) -> bool {
        type_obj.as_type_ptr() == state.uuid_type.as_ptr().cast()
    }

    pub fn new(obj: Borrowed<'a, 'py, PyAny>, state: &'a State) -> Self {
        UUID {
            obj: obj,
            state: state,
        }
    }
    pub fn write_buf<W>(&self, writer: &mut W) -> Result<(), std::io::Error>
    where
        W: std::io::Write,
    {
        let value: u128 = self
            .obj
            .getattr(self.state.int_str.bind_borrowed(self.obj.py()))
            .unwrap()
            .extract()
            .unwrap();
        let buffer = value.to_be_bytes();

        write_group(writer, &buffer[..4])?;
        writer.write_all(b"-")?;
        write_group(writer, &buffer[4..6])?;
        writer.write_all(b"-")?;
        write_group(writer, &buffer[6..8])?;
        writer.write_all(b"-")?;
        write_group(writer, &buffer[8..10])?;
        writer.write_all(b"-")?;
        write_group(writer, &buffer[10..])?;
        Ok(())
    }
}

impl Serialize for UUID<'_, '_> {
    #[inline(never)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut cursor = std::io::Cursor::new([0u8; 64]);
        self.write_buf(&mut cursor).unwrap();
        let len = cursor.position() as usize;
        let value = unsafe { std::str::from_utf8_unchecked(&cursor.get_ref()[0..len]) };
        serializer.serialize_str(value)
    }
}
