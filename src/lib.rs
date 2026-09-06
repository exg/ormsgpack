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

use crate::ffi::*;
use pyo3::ffi::*;
use std::ffi::CStr;
use std::os::raw::c_char;
use std::os::raw::c_int;
use std::os::raw::c_long;
use std::os::raw::c_void;
use std::ptr::NonNull;

const PACKB_DOC: &CStr =
    c"packb(obj, /, default=None, option=None)\n--\n\nSerialize Python objects to msgpack.";
const UNPACKB_DOC: &CStr =
    c"unpackb(obj, /, *, ext_hook=None, option=None)\n--\n\nDeserialize msgpack to Python objects.";

#[repr(transparent)]
struct ModuleState(Option<Box<state::State>>);

impl ModuleState {
    fn set(&mut self, state: state::State) -> &state::State {
        self.0 = Some(Box::new(state));
        self.get()
    }

    fn get(&self) -> &state::State {
        self.0.as_deref().unwrap()
    }

    fn clear(&mut self) {
        self.0.take();
    }
}

macro_rules! module_add_object {
    ($mptr: expr, $name: expr, $object:expr) => {
        if PyModule_AddObjectRef($mptr, $name.as_ptr(), $object.as_ptr()) == -1 {
            return -1;
        }
    };
}

macro_rules! module_add_int {
    ($mptr:expr, $name:expr, $int:expr) => {
        if PyModule_AddIntConstant($mptr, $name.as_ptr(), $int as c_long) == -1 {
            return -1;
        }
    };
}

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
        m_size: std::mem::size_of::<ModuleState>() as Py_ssize_t,
        m_methods: Box::into_raw(methods).cast::<PyMethodDef>(),
        m_slots: Box::into_raw(slots).cast::<PyModuleDef_Slot>(),
        m_traverse: None,
        m_clear: None,
        m_free: Some(ormsgpack_free),
    });
    let init_ptr = Box::into_raw(init);
    PyModuleDef_Init(init_ptr);
    init_ptr
}

unsafe extern "C" fn ormsgpack_free(op: *mut c_void) {
    let state = &mut *PyModule_GetState(op.cast()).cast::<ModuleState>();
    state.clear();
}

#[allow(non_snake_case)]
#[no_mangle]
#[cold]
pub unsafe extern "C" fn ormsgpack_exec(mptr: *mut PyObject) -> c_int {
    PyDateTime_IMPORT();
    if PyDateTimeAPI().is_null() {
        return -1;
    }

    let Some(new_state) = state::State::new() else {
        return -1;
    };
    let module_state = &mut *PyModule_GetState(mptr).cast::<ModuleState>();
    let state = module_state.set(new_state);

    let version = env!("CARGO_PKG_VERSION");
    let version = OwnedPyObject::from_owned_ptr(PyUnicode_FromStringAndSize(
        version.as_ptr().cast::<c_char>(),
        version.len() as isize,
    ));
    module_add_object!(mptr, c"__version__", version);
    module_add_object!(mptr, c"Ext", state.serialize.ext.type_object);
    module_add_object!(mptr, c"Fragment", state.serialize.fragment.type_object);
    module_add_object!(
        mptr,
        c"MsgpackDecodeError",
        state.deserialize.MsgpackDecodeError
    );
    module_add_object!(
        mptr,
        c"MsgpackEncodeError",
        state.serialize.MsgpackEncodeError
    );

    module_add_int!(
        mptr,
        c"OPT_DATETIME_AS_TIMESTAMP_EXT",
        opt::DATETIME_AS_TIMESTAMP_EXT
    );
    module_add_int!(mptr, c"OPT_NAIVE_UTC", opt::NAIVE_UTC);
    module_add_int!(mptr, c"OPT_NON_STR_KEYS", opt::NON_STR_KEYS);
    module_add_int!(mptr, c"OPT_OMIT_MICROSECONDS", opt::OMIT_MICROSECONDS);
    module_add_int!(mptr, c"OPT_PASSTHROUGH_BIG_INT", opt::PASSTHROUGH_BIG_INT);
    module_add_int!(
        mptr,
        c"OPT_PASSTHROUGH_DATACLASS",
        opt::PASSTHROUGH_DATACLASS
    );
    module_add_int!(mptr, c"OPT_PASSTHROUGH_DATETIME", opt::PASSTHROUGH_DATETIME);
    module_add_int!(mptr, c"OPT_PASSTHROUGH_ENUM", opt::PASSTHROUGH_ENUM);
    module_add_int!(mptr, c"OPT_PASSTHROUGH_SUBCLASS", opt::PASSTHROUGH_SUBCLASS);
    module_add_int!(mptr, c"OPT_PASSTHROUGH_TUPLE", opt::PASSTHROUGH_TUPLE);
    module_add_int!(mptr, c"OPT_PASSTHROUGH_UUID", opt::PASSTHROUGH_UUID);
    module_add_int!(mptr, c"OPT_REPLACE_SURROGATES", opt::REPLACE_SURROGATES);
    module_add_int!(mptr, c"OPT_SERIALIZE_NUMPY", opt::SERIALIZE_NUMPY);
    module_add_int!(mptr, c"OPT_SERIALIZE_PYDANTIC", opt::SERIALIZE_PYDANTIC);
    module_add_int!(mptr, c"OPT_SORT_KEYS", opt::SORT_KEYS);
    module_add_int!(mptr, c"OPT_UTC_Z", opt::UTC_Z);

    0
}

#[cold]
#[inline(never)]
fn raise_unpackb_exception(state: &state::State, msg: &str) -> *mut PyObject {
    unsafe {
        set_python_error(state.deserialize.MsgpackDecodeError.as_ptr(), msg);
    };
    std::ptr::null_mut()
}

#[cold]
#[inline(never)]
fn raise_packb_exception(state: &state::State, msg: &str) -> *mut PyObject {
    unsafe {
        set_python_error(state.serialize.MsgpackEncodeError.as_ptr(), msg);
    };
    std::ptr::null_mut()
}

unsafe fn parse_option_arg(
    opts: Option<NonNull<PyObject>>,
    mask: opt::Opt,
) -> Result<opt::Opt, ()> {
    let Some(opts) = opts else {
        return Ok(0);
    };
    let opts = opts.as_ptr();
    if Py_TYPE(opts) == &raw mut PyLong_Type {
        let val = PyLong_AsLong(opts);
        let val = opt::Opt::try_from(val).map_err(|_| ())?;
        if val & !mask == 0 {
            Ok(val)
        } else {
            Err(())
        }
    } else if opts == Py_None() {
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
    let state = (&*PyModule_GetState(module).cast::<ModuleState>()).get();
    let mut ext_hook: Option<NonNull<PyObject>> = None;
    let mut optsptr: Option<NonNull<PyObject>> = None;

    let num_args = PyVectorcall_NARGS(nargs as usize);
    if num_args != 1 {
        let msg = if num_args > 1 {
            "unpackb() accepts only 1 positional argument"
        } else {
            "unpackb() missing 1 required positional argument: 'obj'"
        };
        return raise_unpackb_exception(state, msg);
    }
    if !kwnames.is_null() {
        let tuple_size = Py_SIZE(kwnames);
        for i in 0..tuple_size {
            let arg = pytuple_get_item(kwnames, i as Py_ssize_t);
            if PyUnicode_Compare(arg, state.ext_hook_str.as_ptr()) == 0 {
                ext_hook = Some(NonNull::new_unchecked(*args.offset(num_args + i)));
            } else if PyUnicode_Compare(arg, state.option_str.as_ptr()) == 0 {
                optsptr = Some(NonNull::new_unchecked(*args.offset(num_args + i)));
            } else {
                return raise_unpackb_exception(
                    state,
                    "unpackb() got an unexpected keyword argument",
                );
            }
        }
    }

    let opts = match parse_option_arg(optsptr, opt::UNPACKB_OPT_MASK) {
        Ok(val) => val,
        Err(()) => return raise_unpackb_exception(state, "Invalid opts"),
    };

    match crate::deserialize::deserialize(*args, &state.deserialize, ext_hook, opts) {
        Ok(val) => val.as_ptr(),
        Err(err) => raise_unpackb_exception(state, &err.message),
    }
}

#[no_mangle]
pub unsafe extern "C" fn packb(
    module: *mut PyObject,
    args: *const *mut PyObject,
    nargs: Py_ssize_t,
    kwnames: *mut PyObject,
) -> *mut PyObject {
    let state = (&*PyModule_GetState(module).cast::<ModuleState>()).get();
    let mut default: Option<NonNull<PyObject>> = None;
    let mut optsptr: Option<NonNull<PyObject>> = None;

    let num_args = PyVectorcall_NARGS(nargs as usize);
    if num_args == 0 {
        return raise_packb_exception(
            state,
            "packb() missing 1 required positional argument: 'obj'",
        );
    }
    if num_args >= 2 {
        default = Some(NonNull::new_unchecked(*args.offset(1)));
    }
    if num_args >= 3 {
        optsptr = Some(NonNull::new_unchecked(*args.offset(2)));
    }
    if !kwnames.is_null() {
        let tuple_size = Py_SIZE(kwnames);
        for i in 0..tuple_size {
            let arg = pytuple_get_item(kwnames, i as Py_ssize_t);
            if PyUnicode_Compare(arg, state.default_str.as_ptr()) == 0 {
                if default.is_some() {
                    return raise_packb_exception(
                        state,
                        "packb() got multiple values for argument: 'default'",
                    );
                }
                default = Some(NonNull::new_unchecked(*args.offset(num_args + i)));
            } else if PyUnicode_Compare(arg, state.option_str.as_ptr()) == 0 {
                if optsptr.is_some() {
                    return raise_packb_exception(
                        state,
                        "packb() got multiple values for argument: 'option'",
                    );
                }
                optsptr = Some(NonNull::new_unchecked(*args.offset(num_args + i)));
            } else {
                return raise_packb_exception(state, "packb() got an unexpected keyword argument");
            }
        }
    }

    let opts = match parse_option_arg(optsptr, opt::PACKB_OPT_MASK) {
        Ok(val) => val,
        Err(()) => return raise_packb_exception(state, "Invalid opts"),
    };

    match crate::serialize::serialize(*args, &state.serialize, default, opts) {
        Ok(val) => val.as_ptr(),
        Err(err) => raise_packb_exception(state, &err),
    }
}
