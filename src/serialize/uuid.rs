// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::PyObjectWithType;
use serde::ser::{Serialize, Serializer};
use std::os::raw::c_uchar;

pub struct State {
    pub type_object: *mut pyo3::ffi::PyTypeObject,
    pub int_str: *mut pyo3::ffi::PyObject,
}

impl State {
    #[cold]
    pub fn new() -> Self {
        unsafe {
            let module = pyo3::ffi::PyImport_ImportModule(c"uuid".as_ptr());
            let type_object = pyo3::ffi::PyObject_GetAttrString(module, c"UUID".as_ptr()).cast();
            pyo3::ffi::Py_DECREF(module);
            Self {
                type_object,
                int_str: pyo3::ffi::PyUnicode_InternFromString(c"int".as_ptr()),
            }
        }
    }
}

pub struct UUID<'a> {
    ptr: *mut pyo3::ffi::PyObject,
    state: &'a State,
}

const HEX: [u8; 16] = *b"0123456789abcdef";

fn write_group<W>(writer: &mut W, group: &[c_uchar]) -> Result<(), std::io::Error>
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

impl<'a> UUID<'a> {
    #[inline]
    pub fn try_new(obj: PyObjectWithType, state: &'a State) -> Option<Self> {
        if obj.get_type_ptr() == state.type_object {
            Some(Self {
                ptr: obj.as_ptr(),
                state,
            })
        } else {
            None
        }
    }
    pub fn write_buf<W>(&self, writer: &mut W) -> Result<(), std::io::Error>
    where
        W: std::io::Write,
    {
        let mut buffer: [c_uchar; 16] = [0; 16];
        unsafe {
            let value = pyo3::ffi::PyObject_GetAttr(self.ptr, self.state.int_str);
            #[cfg(Py_3_13)]
            {
                pyo3::ffi::PyLong_AsNativeBytes(
                    value,
                    buffer.as_mut_ptr().cast(),
                    16,
                    pyo3::ffi::Py_ASNATIVEBYTES_BIG_ENDIAN
                        | pyo3::ffi::Py_ASNATIVEBYTES_UNSIGNED_BUFFER
                        | pyo3::ffi::Py_ASNATIVEBYTES_REJECT_NEGATIVE,
                );
            }
            #[cfg(not(Py_3_13))]
            {
                pyo3::ffi::_PyLong_AsByteArray(
                    value.cast::<pyo3::ffi::PyLongObject>(),
                    buffer.as_mut_ptr(),
                    16,
                    0, // little_endian
                    0, // is_signed
                );
            }
            pyo3::ffi::Py_DECREF(value);
        };

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

impl Serialize for UUID<'_> {
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
