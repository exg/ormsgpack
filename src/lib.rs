// SPDX-License-Identifier: (Apache-2.0 OR MIT)
#![allow(unused_unsafe)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::ptr_eq)]
#![allow(clippy::redundant_field_names)]
#![allow(clippy::unusual_byte_groupings)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::zero_prefixed_literal)]
#![deny(clippy::ptr_as_ptr)]

#[macro_use]
mod util;

mod deserialize;
mod exc;
mod ext;
mod ffi;
mod fragment;
mod io;
mod msgpack;
mod opt;
mod serialize;
mod state;
mod str;

use pyo3::ffi::*;
use pyo3::prelude::*;
use pyo3::types::{PyInt, PyTuple};
use std::ffi::CStr;
use std::os::raw::c_int;
use std::os::raw::c_void;

const PACKB_DOC: &CStr =
    c"packb(obj, /, default=None, option=None)\n--\n\nSerialize Python objects to msgpack.";
const UNPACKB_DOC: &CStr =
    c"unpackb(obj, /, *, ext_hook=None, option=None)\n--\n\nDeserialize msgpack to Python objects.";

#[allow(non_snake_case)]
#[no_mangle]
#[cold]
pub unsafe extern "C" fn PyInit_ormsgpack() -> *mut PyModuleDef {
    let methods: Box<[PyMethodDef; 3]> = Box::new([
        PyMethodDef {
            ml_name: c"packb".as_ptr(),
            ml_meth: PyMethodDefPointer {
                PyCFunctionFastWithKeywords: packb,
            },
            ml_flags: METH_FASTCALL | METH_KEYWORDS,
            ml_doc: PACKB_DOC.as_ptr(),
        },
        PyMethodDef {
            ml_name: c"unpackb".as_ptr(),
            ml_meth: PyMethodDefPointer {
                PyCFunctionFastWithKeywords: unpackb,
            },
            ml_flags: METH_FASTCALL | METH_KEYWORDS,
            ml_doc: UNPACKB_DOC.as_ptr(),
        },
        PyMethodDef::zeroed(),
    ]);

    let slots: Box<[PyModuleDef_Slot]> = Box::new([
        PyModuleDef_Slot {
            slot: Py_mod_exec,
            value: ormsgpack_exec as *mut c_void,
        },
        #[cfg(Py_3_12)]
        PyModuleDef_Slot {
            slot: Py_mod_multiple_interpreters,
            value: Py_MOD_PER_INTERPRETER_GIL_SUPPORTED,
        },
        #[cfg(Py_3_13)]
        PyModuleDef_Slot {
            slot: Py_mod_gil,
            value: Py_MOD_GIL_NOT_USED,
        },
        PyModuleDef_Slot {
            slot: 0,
            value: std::ptr::null_mut(),
        },
    ]);

    let init = Box::new(PyModuleDef {
        m_base: PyModuleDef_HEAD_INIT,
        m_name: c"ormsgpack".as_ptr(),
        m_doc: std::ptr::null(),
        m_size: std::mem::size_of::<state::State>() as Py_ssize_t,
        m_methods: Box::into_raw(methods).cast::<PyMethodDef>(),
        m_slots: Box::into_raw(slots).cast::<PyModuleDef_Slot>(),
        m_traverse: None,
        m_clear: None,
        m_free: None,
    });
    let init_ptr = Box::into_raw(init);
    PyModuleDef_Init(init_ptr);
    init_ptr
}

#[allow(non_snake_case)]
#[no_mangle]
#[cold]
pub unsafe extern "C" fn ormsgpack_exec(op: *mut PyObject) -> c_int {
    let py = Python::assume_attached();
    let module = Borrowed::from_ptr(py, op).cast_unchecked::<PyModule>();
    let result: PyResult<()> = (|| {
        let ptr = PyModule_GetState(op).cast::<state::State>();
        std::ptr::write(ptr, state::State::new(py)?);
        let state = &*ptr;

        let ext_type = state.serialize.ext_type.bind(py);
        let fragment_type = state.serialize.fragment_type.bind(py);
        let decode_error = state.deserialize.MsgpackDecodeError.bind(py);
        let encode_error = state.serialize.MsgpackEncodeError.bind(py);

        module.setattr("__version__", env!("CARGO_PKG_VERSION"))?;
        module.setattr("Ext", ext_type)?;
        module.setattr("Fragment", fragment_type)?;
        module.setattr("MsgpackDecodeError", decode_error)?;
        module.setattr("MsgpackEncodeError", encode_error)?;

        module.setattr(
            "OPT_DATETIME_AS_TIMESTAMP_EXT",
            opt::DATETIME_AS_TIMESTAMP_EXT,
        )?;
        module.setattr("OPT_NAIVE_UTC", opt::NAIVE_UTC)?;
        module.setattr("OPT_NON_STR_KEYS", opt::NON_STR_KEYS)?;
        module.setattr("OPT_OMIT_MICROSECONDS", opt::OMIT_MICROSECONDS)?;
        module.setattr("OPT_PASSTHROUGH_BIG_INT", opt::PASSTHROUGH_BIG_INT)?;
        module.setattr("OPT_PASSTHROUGH_DATACLASS", opt::PASSTHROUGH_DATACLASS)?;
        module.setattr("OPT_PASSTHROUGH_DATETIME", opt::PASSTHROUGH_DATETIME)?;
        module.setattr("OPT_PASSTHROUGH_ENUM", opt::PASSTHROUGH_ENUM)?;
        module.setattr("OPT_PASSTHROUGH_SUBCLASS", opt::PASSTHROUGH_SUBCLASS)?;
        module.setattr("OPT_PASSTHROUGH_TUPLE", opt::PASSTHROUGH_TUPLE)?;
        module.setattr("OPT_PASSTHROUGH_UUID", opt::PASSTHROUGH_UUID)?;
        module.setattr("OPT_REPLACE_SURROGATES", opt::REPLACE_SURROGATES)?;
        module.setattr("OPT_SERIALIZE_NUMPY", opt::SERIALIZE_NUMPY)?;
        module.setattr("OPT_SERIALIZE_PYDANTIC", opt::SERIALIZE_PYDANTIC)?;
        module.setattr("OPT_SORT_KEYS", opt::SORT_KEYS)?;
        module.setattr("OPT_UTC_Z", opt::UTC_Z)?;

        Ok(())
    })();

    match result {
        Ok(()) => 0,
        Err(err) => {
            err.restore(py);
            -1
        }
    }
}

#[cold]
#[inline(never)]
fn raise_unpackb_exception(py: Python<'_>, state: &state::State, msg: &str) -> *mut PyObject {
    state.deserialize.error(py, msg).restore(py);
    std::ptr::null_mut()
}

#[cold]
#[inline(never)]
fn raise_packb_exception(py: Python<'_>, state: &state::State, msg: &str) -> *mut PyObject {
    state.serialize.error(py, msg).restore(py);
    std::ptr::null_mut()
}

fn parse_option_arg(opts: Borrowed<'_, '_, PyAny>, mask: i32) -> Result<i32, ()> {
    if opts.is_exact_instance_of::<PyInt>() {
        let opts = unsafe { opts.cast_unchecked::<PyInt>() };
        let val = opts.extract::<i32>().map_err(|_| ())?;
        if val & !mask == 0 {
            Ok(val)
        } else {
            Err(())
        }
    } else if opts.is_none() {
        Ok(0)
    } else {
        Err(())
    }
}

#[no_mangle]
pub unsafe extern "C" fn unpackb(
    module: *mut PyObject,
    args: *const *mut PyObject,
    nargs: Py_ssize_t,
    kwnames: *mut PyObject,
) -> *mut PyObject {
    let py = Python::assume_attached();
    let state = &*PyModule_GetState(module).cast::<state::State>();
    let mut ext_hook_arg: Option<Borrowed<'_, '_, PyAny>> = None;
    let mut option_arg: Option<Borrowed<'_, '_, PyAny>> = None;

    let num_args = PyVectorcall_NARGS(nargs as usize);
    if num_args != 1 {
        let msg = if num_args > 1 {
            "unpackb() accepts only 1 positional argument"
        } else {
            "unpackb() missing 1 required positional argument: 'obj'"
        };
        return raise_unpackb_exception(py, state, msg);
    }
    if !kwnames.is_null() {
        let kwnames = Borrowed::from_ptr(py, kwnames).cast_unchecked::<PyTuple>();
        let ext_hook_str = state.ext_hook_str.bind_borrowed(py);
        let option_str = state.option_str.bind_borrowed(py);
        for (i, arg) in kwnames.iter_borrowed().enumerate() {
            let value = *args.offset(num_args + i as Py_ssize_t);
            if arg.eq(ext_hook_str).unwrap() {
                ext_hook_arg = Some(Borrowed::from_ptr(py, value));
            } else if arg.eq(option_str).unwrap() {
                option_arg = Some(Borrowed::from_ptr(py, value));
            } else {
                return raise_unpackb_exception(
                    py,
                    state,
                    "unpackb() got an unexpected keyword argument",
                );
            }
        }
    }

    let mut optsbits: i32 = 0;
    if let Some(opts) = option_arg {
        match parse_option_arg(opts, opt::UNPACKB_OPT_MASK) {
            Ok(val) => optsbits = val,
            Err(()) => return raise_unpackb_exception(py, state, "Invalid opts"),
        }
    }

    let obj = Borrowed::from_ptr(py, *args);
    match crate::deserialize::deserialize(
        obj,
        &state.deserialize,
        ext_hook_arg,
        optsbits as opt::Opt,
    ) {
        Ok(val) => val.into_ptr(),
        Err(err) => {
            err.restore(py);
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn packb(
    module: *mut PyObject,
    args: *const *mut PyObject,
    nargs: Py_ssize_t,
    kwnames: *mut PyObject,
) -> *mut PyObject {
    let py = Python::assume_attached();
    let state = &*PyModule_GetState(module).cast::<state::State>();
    let mut default_arg: Option<Borrowed<'_, '_, PyAny>> = None;
    let mut option_arg: Option<Borrowed<'_, '_, PyAny>> = None;

    let num_args = PyVectorcall_NARGS(nargs as usize);
    if num_args == 0 {
        return raise_packb_exception(
            py,
            state,
            "packb() missing 1 required positional argument: 'obj'",
        );
    }
    if num_args >= 2 {
        default_arg = Some(Borrowed::from_ptr(py, *args.offset(1)));
    }
    if num_args >= 3 {
        option_arg = Some(Borrowed::from_ptr(py, *args.offset(2)));
    }
    if !kwnames.is_null() {
        let kwnames = Borrowed::from_ptr(py, kwnames).cast_unchecked::<PyTuple>();
        let default_str = state.default_str.bind_borrowed(py);
        let option_str = state.option_str.bind_borrowed(py);
        for (i, arg) in kwnames.iter_borrowed().enumerate() {
            let value = *args.offset(num_args + i as Py_ssize_t);
            if arg.eq(default_str).unwrap() {
                if default_arg.is_some() {
                    return raise_packb_exception(
                        py,
                        state,
                        "packb() got multiple values for argument: 'default'",
                    );
                }
                default_arg = Some(Borrowed::from_ptr(py, value));
            } else if arg.eq(option_str).unwrap() {
                if option_arg.is_some() {
                    return raise_packb_exception(
                        py,
                        state,
                        "packb() got multiple values for argument: 'option'",
                    );
                }
                option_arg = Some(Borrowed::from_ptr(py, value));
            } else {
                return raise_packb_exception(
                    py,
                    state,
                    "packb() got an unexpected keyword argument",
                );
            }
        }
    }

    let mut optsbits: i32 = 0;
    if let Some(opts) = option_arg {
        match parse_option_arg(opts, opt::PACKB_OPT_MASK) {
            Ok(val) => optsbits = val,
            Err(()) => return raise_packb_exception(py, state, "Invalid opts"),
        }
    }

    let obj = Borrowed::from_ptr(py, *args);
    match crate::serialize::serialize(obj, &state.serialize, default_arg, optsbits as opt::Opt) {
        Ok(val) => val.into_ptr(),
        Err(err) => {
            err.restore(py);
            std::ptr::null_mut()
        }
    }
}
