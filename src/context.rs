// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::deserialize::KeyMap;
use pyo3::ffi::*;
use std::ffi::CStr;
use std::ptr::null_mut;
use std::sync::OnceLock;

use crate::ext::create_ext_type;

pub struct NumpyTypes {
    pub array: *mut PyTypeObject,
    pub float64: *mut PyTypeObject,
    pub float32: *mut PyTypeObject,
    pub float16: *mut PyTypeObject,
    pub int64: *mut PyTypeObject,
    pub int32: *mut PyTypeObject,
    pub int16: *mut PyTypeObject,
    pub int8: *mut PyTypeObject,
    pub uint64: *mut PyTypeObject,
    pub uint32: *mut PyTypeObject,
    pub uint16: *mut PyTypeObject,
    pub uint8: *mut PyTypeObject,
    pub bool_: *mut PyTypeObject,
    pub datetime64: *mut PyTypeObject,
}

#[repr(C)]
pub struct Context {
    pub dataclass_field_type: *mut PyTypeObject,
    pub date_type: *mut PyTypeObject,
    pub time_type: *mut PyTypeObject,
    pub datetime_type: *mut PyTypeObject,
    pub enum_type: *mut PyTypeObject,
    pub ext_type: *mut PyTypeObject,
    pub numpy_types: OnceLock<Option<NumpyTypes>>,
    pub uuid_type: *mut PyTypeObject,
    pub array_struct_str: *mut PyObject,
    pub dataclass_fields_str: *mut PyObject,
    pub default_str: *mut PyObject,
    pub descr_str: *mut PyObject,
    pub dict_str: *mut PyObject,
    pub dtype_str: *mut PyObject,
    pub ext_hook_str: *mut PyObject,
    pub field_type_str: *mut PyObject,
    pub fields_str: *mut PyObject,
    pub int_str: *mut PyObject,
    pub normalize_str: *mut PyObject,
    pub option_str: *mut PyObject,
    pub pydantic_validator_str: *mut PyObject,
    pub slots_str: *mut PyObject,
    pub utcoffset_str: *mut PyObject,
    pub value_str: *mut PyObject,
    pub msgpack_encode_error: *mut PyObject,
    pub msgpack_decode_error: *mut PyObject,
    pub key_map: KeyMap<512>,
}

#[cold]
pub unsafe fn init_context(context: *mut Context) {
    PyDateTime_IMPORT();
    Py_INCREF(PyExc_TypeError);
    Py_INCREF(PyExc_ValueError);
    *context = Context {
        dataclass_field_type: look_up_type(c"dataclasses", c"_FIELD"),
        datetime_type: (*PyDateTimeAPI()).DateTimeType,
        date_type: (*PyDateTimeAPI()).DateType,
        time_type: (*PyDateTimeAPI()).TimeType,
        enum_type: look_up_type(c"enum", c"EnumMeta"),
        ext_type: create_ext_type(),
        numpy_types: OnceLock::new(),
        uuid_type: look_up_type(c"uuid", c"UUID"),
        array_struct_str: PyUnicode_InternFromString(c"__array_struct__".as_ptr()),
        dataclass_fields_str: PyUnicode_InternFromString(c"__dataclass_fields__".as_ptr()),
        default_str: PyUnicode_InternFromString(c"default".as_ptr()),
        descr_str: PyUnicode_InternFromString(c"descr".as_ptr()),
        dict_str: PyUnicode_InternFromString(c"__dict__".as_ptr()),
        dtype_str: PyUnicode_InternFromString(c"dtype".as_ptr()),
        ext_hook_str: PyUnicode_InternFromString(c"ext_hook".as_ptr()),
        field_type_str: PyUnicode_InternFromString(c"_field_type".as_ptr()),
        fields_str: PyUnicode_InternFromString(c"__fields__".as_ptr()),
        int_str: PyUnicode_InternFromString(c"int".as_ptr()),
        normalize_str: PyUnicode_InternFromString(c"normalize".as_ptr()),
        option_str: PyUnicode_InternFromString(c"option".as_ptr()),
        pydantic_validator_str: PyUnicode_InternFromString(c"__pydantic_validator__".as_ptr()),
        slots_str: PyUnicode_InternFromString(c"__slots__".as_ptr()),
        utcoffset_str: PyUnicode_InternFromString(c"utcoffset".as_ptr()),
        value_str: PyUnicode_InternFromString(c"value".as_ptr()),
        msgpack_encode_error: PyExc_TypeError,
        msgpack_decode_error: PyExc_ValueError,
        key_map: KeyMap::new(),
    };
}

#[cold]
unsafe fn look_up_numpy_type(
    numpy_module_dict: *mut PyObject,
    np_type: &CStr,
) -> *mut PyTypeObject {
    PyMapping_GetItemString(numpy_module_dict, np_type.as_ptr()).cast::<PyTypeObject>()
}

#[cold]
pub fn load_numpy_types() -> Option<NumpyTypes> {
    unsafe {
        let numpy = PyImport_ImportModule(c"numpy".as_ptr());
        if numpy.is_null() {
            PyErr_Clear();
            return None;
        }

        let numpy_module_dict = PyObject_GenericGetDict(numpy, null_mut());
        let types = NumpyTypes {
            array: look_up_numpy_type(numpy_module_dict, c"ndarray"),
            float16: look_up_numpy_type(numpy_module_dict, c"half"),
            float32: look_up_numpy_type(numpy_module_dict, c"float32"),
            float64: look_up_numpy_type(numpy_module_dict, c"float64"),
            int8: look_up_numpy_type(numpy_module_dict, c"int8"),
            int16: look_up_numpy_type(numpy_module_dict, c"int16"),
            int32: look_up_numpy_type(numpy_module_dict, c"int32"),
            int64: look_up_numpy_type(numpy_module_dict, c"int64"),
            uint16: look_up_numpy_type(numpy_module_dict, c"uint16"),
            uint32: look_up_numpy_type(numpy_module_dict, c"uint32"),
            uint64: look_up_numpy_type(numpy_module_dict, c"uint64"),
            uint8: look_up_numpy_type(numpy_module_dict, c"uint8"),
            bool_: look_up_numpy_type(numpy_module_dict, c"bool_"),
            datetime64: look_up_numpy_type(numpy_module_dict, c"datetime64"),
        };
        Py_DECREF(numpy_module_dict);
        Py_DECREF(numpy);
        Some(types)
    }
}

#[cold]
unsafe fn look_up_type(module_name: &CStr, type_name: &CStr) -> *mut PyTypeObject {
    let module = PyImport_ImportModule(module_name.as_ptr());
    let module_dict = PyObject_GenericGetDict(module, null_mut());
    let ptr = PyMapping_GetItemString(module_dict, type_name.as_ptr()).cast::<PyTypeObject>();
    Py_DECREF(module_dict);
    Py_DECREF(module);
    ptr
}
