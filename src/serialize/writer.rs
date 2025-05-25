// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::*;
use crate::io::WriteSlices;
use pyo3::ffi::*;
use std::ptr::NonNull;

const BUFFER_LENGTH: usize = 1024;

pub struct BytesWriter {
    cap: usize,
    pos: usize,
    len: usize,
    bytes: *mut PyObject,
}

impl BytesWriter {
    pub fn default() -> Self {
        BytesWriter {
            cap: BUFFER_LENGTH,
            pos: 0,
            len: 0,
            bytes: unsafe {
                PyBytes_FromStringAndSize(std::ptr::null_mut(), BUFFER_LENGTH as isize)
            },
        }
    }

    pub fn finish(&mut self) -> NonNull<PyObject> {
        unsafe {
            let ptr = pybytes_as_mut_u8(self.bytes).add(self.len);
            std::ptr::write(ptr, 0);
            self.resize(self.len);
            NonNull::new_unchecked(self.bytes)
        }
    }

    #[inline]
    fn resize(&mut self, len: usize) {
        self.cap = len;
        unsafe {
            _PyBytes_Resize(&raw mut self.bytes, len as isize);
        }
    }

    #[cold]
    #[inline(never)]
    fn grow(&mut self, len: usize) {
        let mut cap = self.cap;
        while len >= cap {
            if len < 262144 {
                cap *= 4;
            } else {
                cap *= 2;
            }
        }
        self.resize(cap);
    }

    fn insert_slices<const N: usize>(&mut self, bufs: [&[u8]; N]) {
        let len: usize = bufs.iter().map(|b| b.len()).sum();
        let new_pos = self.pos + len;
        if new_pos > self.cap {
            self.grow(new_pos);
        }
        unsafe {
            let mut ptr = pybytes_as_mut_u8(self.bytes).add(self.pos);
            for buf in bufs {
                std::ptr::copy_nonoverlapping(buf.as_ptr(), ptr, buf.len());
                ptr = ptr.add(buf.len());
            }
        };
        self.pos = new_pos;
        if new_pos > self.len {
            self.len = new_pos;
        }
    }
}

impl std::io::Write for BytesWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.insert_slices([buf]);
        Ok(buf.len())
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.insert_slices([buf]);
        Ok(())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl WriteSlices for BytesWriter {
    fn write_slices<const N: usize>(&mut self, bufs: [&[u8]; N]) -> Result<(), std::io::Error> {
        self.insert_slices(bufs);
        Ok(())
    }
}

impl std::io::Seek for BytesWriter {
    fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
        debug_assert!(size_of::<u64>() >= size_of::<usize>());
        let (base_pos, offset) = match pos {
            std::io::SeekFrom::Start(n) => {
                self.pos = std::cmp::min(n, self.len as u64) as usize;
                return Ok(self.pos as u64);
            }
            std::io::SeekFrom::End(n) => (self.len as u64, n),
            std::io::SeekFrom::Current(n) => (self.pos as u64, n),
        };
        match base_pos.checked_add_signed(offset) {
            Some(n) => {
                self.pos = std::cmp::min(n, self.len as u64) as usize;
                Ok(self.pos as u64)
            }
            None => Err(std::io::Error::from(std::io::ErrorKind::InvalidInput)),
        }
    }

    fn stream_position(&mut self) -> std::io::Result<u64> {
        Ok(self.pos as u64)
    }
}
