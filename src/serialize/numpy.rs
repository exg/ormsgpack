use crate::ffi::*;
use crate::opt::*;
use crate::serialize::datetimelike::NaiveDateTime;
use chrono::{DateTime, NaiveDate};
use pyo3::ffi::*;
use pyo3::prelude::*;
use pyo3::types::{PyCapsule, PyList, PyString};
use serde::ser::{Serialize, SerializeSeq, Serializer};
use std::os::raw::{c_char, c_int, c_void};
use std::sync::OnceLock;

pub struct NumpyTypes {
    pub array: Py<PyAny>,
    pub float64: Py<PyAny>,
    pub float32: Py<PyAny>,
    pub float16: Py<PyAny>,
    pub int64: Py<PyAny>,
    pub int32: Py<PyAny>,
    pub int16: Py<PyAny>,
    pub int8: Py<PyAny>,
    pub uint64: Py<PyAny>,
    pub uint32: Py<PyAny>,
    pub uint16: Py<PyAny>,
    pub uint8: Py<PyAny>,
    pub bool_: Py<PyAny>,
    pub datetime64: Py<PyAny>,
}

#[cold]
fn load_numpy_types(py: Python<'_>) -> PyResult<Option<NumpyTypes>> {
    let numpy = match py.import("numpy") {
        Ok(module) => module,
        Err(_) => return Ok(None),
    };

    Ok(Some(NumpyTypes {
        array: numpy.getattr("ndarray")?.unbind(),
        float16: numpy.getattr("half")?.unbind(),
        float32: numpy.getattr("float32")?.unbind(),
        float64: numpy.getattr("float64")?.unbind(),
        int8: numpy.getattr("int8")?.unbind(),
        int16: numpy.getattr("int16")?.unbind(),
        int32: numpy.getattr("int32")?.unbind(),
        int64: numpy.getattr("int64")?.unbind(),
        uint16: numpy.getattr("uint16")?.unbind(),
        uint32: numpy.getattr("uint32")?.unbind(),
        uint64: numpy.getattr("uint64")?.unbind(),
        uint8: numpy.getattr("uint8")?.unbind(),
        bool_: numpy.getattr("bool_")?.unbind(),
        datetime64: numpy.getattr("datetime64")?.unbind(),
    }))
}

pub struct State {
    numpy_types: OnceLock<PyResult<Option<NumpyTypes>>>,
    array_struct_str: Py<PyString>,
    descr_str: Py<PyString>,
    dtype_str: Py<PyString>,
}

impl State {
    #[cold]
    pub fn new(py: Python<'_>) -> Self {
        Self {
            numpy_types: OnceLock::new(),
            array_struct_str: PyString::intern(py, "__array_struct__").unbind(),
            descr_str: PyString::intern(py, "descr").unbind(),
            dtype_str: PyString::intern(py, "dtype").unbind(),
        }
    }

    pub fn get_numpy_types(&self, py: Python<'_>) -> PyResult<&Option<NumpyTypes>> {
        match self.numpy_types.get_or_init(|| load_numpy_types(py)) {
            Ok(types) => Ok(types),
            Err(err) => Err(err.clone_ref(py)),
        }
    }
}

// https://numpy.org/doc/1.26/reference/arrays.interface.html#object.__array_struct__

#[repr(C)]
pub struct PyArrayInterface {
    pub two: c_int,
    pub nd: c_int,
    pub typekind: c_char,
    pub itemsize: c_int,
    pub flags: c_int,
    pub shape: *mut Py_intptr_t,
    pub strides: *mut Py_intptr_t,
    pub data: *mut c_void,
    pub descr: *mut PyObject,
}

#[derive(Clone, Copy)]
enum ItemType {
    BOOL,
    DATETIME64(NumpyDatetimeUnit),
    F16,
    F32,
    F64,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
}

impl ItemType {
    fn find(
        array: *mut PyArrayInterface,
        obj: Borrowed<'_, '_, PyAny>,
        state: &State,
    ) -> Option<ItemType> {
        match unsafe { ((*array).typekind, (*array).itemsize) } {
            (098, 1) => Some(ItemType::BOOL),
            (077, 8) => {
                let unit = NumpyDatetimeUnit::from_pyobject(obj, state);
                Some(ItemType::DATETIME64(unit))
            }
            (102, 2) => Some(ItemType::F16),
            (102, 4) => Some(ItemType::F32),
            (102, 8) => Some(ItemType::F64),
            (105, 1) => Some(ItemType::I8),
            (105, 2) => Some(ItemType::I16),
            (105, 4) => Some(ItemType::I32),
            (105, 8) => Some(ItemType::I64),
            (117, 1) => Some(ItemType::U8),
            (117, 2) => Some(ItemType::U16),
            (117, 4) => Some(ItemType::U32),
            (117, 8) => Some(ItemType::U64),
            _ => None,
        }
    }
}

pub enum PyArrayError {
    Malformed,
    NotContiguous,
    UnsupportedDataType,
}

struct NumpyArrayData {
    data: *const c_void,
    len: usize,
    kind: ItemType,
    opts: Opt,
}

impl Serialize for NumpyArrayData {
    #[inline(never)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.len))?;
        match self.kind {
            ItemType::BOOL => {
                let slice: &[u8] =
                    unsafe { std::slice::from_raw_parts(self.data.cast::<u8>(), self.len) };
                for &each in slice.iter() {
                    let value = each == 1;
                    seq.serialize_element(&value).unwrap();
                }
            }
            ItemType::DATETIME64(unit) => {
                let slice: &[i64] =
                    unsafe { std::slice::from_raw_parts(self.data.cast::<i64>(), self.len) };
                for &each in slice.iter() {
                    let value = unit
                        .datetime(each, self.opts)
                        .map_err(serde::ser::Error::custom)?;
                    seq.serialize_element(&value).unwrap();
                }
            }
            ItemType::F16 => {
                let slice: &[u16] =
                    unsafe { std::slice::from_raw_parts(self.data.cast::<u16>(), self.len) };
                for &each in slice.iter() {
                    let value = half::f16::from_bits(each).to_f32();
                    seq.serialize_element(&value).unwrap();
                }
            }
            ItemType::F32 => {
                let slice: &[f32] =
                    unsafe { std::slice::from_raw_parts(self.data.cast::<f32>(), self.len) };
                for &each in slice.iter() {
                    seq.serialize_element(&each).unwrap();
                }
            }
            ItemType::F64 => {
                let slice: &[f64] =
                    unsafe { std::slice::from_raw_parts(self.data.cast::<f64>(), self.len) };
                for &each in slice.iter() {
                    seq.serialize_element(&each).unwrap();
                }
            }
            ItemType::I8 => {
                let slice: &[i8] =
                    unsafe { std::slice::from_raw_parts(self.data.cast::<i8>(), self.len) };
                for &each in slice.iter() {
                    seq.serialize_element(&each).unwrap();
                }
            }
            ItemType::I16 => {
                let slice: &[i16] =
                    unsafe { std::slice::from_raw_parts(self.data.cast::<i16>(), self.len) };
                for &each in slice.iter() {
                    seq.serialize_element(&each).unwrap();
                }
            }
            ItemType::I32 => {
                let slice: &[i32] =
                    unsafe { std::slice::from_raw_parts(self.data.cast::<i32>(), self.len) };
                for &each in slice.iter() {
                    seq.serialize_element(&each).unwrap();
                }
            }
            ItemType::I64 => {
                let slice: &[i64] =
                    unsafe { std::slice::from_raw_parts(self.data.cast::<i64>(), self.len) };
                for &each in slice.iter() {
                    seq.serialize_element(&each).unwrap();
                }
            }
            ItemType::U8 => {
                let slice: &[u8] =
                    unsafe { std::slice::from_raw_parts(self.data.cast::<u8>(), self.len) };
                for &each in slice.iter() {
                    seq.serialize_element(&each).unwrap();
                }
            }
            ItemType::U16 => {
                let slice: &[u16] =
                    unsafe { std::slice::from_raw_parts(self.data.cast::<u16>(), self.len) };
                for &each in slice.iter() {
                    seq.serialize_element(&each).unwrap();
                }
            }
            ItemType::U32 => {
                let slice: &[u32] =
                    unsafe { std::slice::from_raw_parts(self.data.cast::<u32>(), self.len) };
                for &each in slice.iter() {
                    seq.serialize_element(&each).unwrap();
                }
            }
            ItemType::U64 => {
                let slice: &[u64] =
                    unsafe { std::slice::from_raw_parts(self.data.cast::<u64>(), self.len) };
                for &each in slice.iter() {
                    seq.serialize_element(&each).unwrap();
                }
            }
        }
        seq.end()
    }
}

enum NumpyArrayNode {
    Internal(Vec<NumpyArrayNode>),
    Leaf(NumpyArrayData),
}

impl Serialize for NumpyArrayNode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Internal(children) => {
                let mut seq = serializer.serialize_seq(Some(children.len()))?;
                for child in children {
                    seq.serialize_element(child).unwrap();
                }
                seq.end()
            }
            Self::Leaf(value) => value.serialize(serializer),
        }
    }
}

// >>> arr = numpy.array([[[1, 2], [3, 4]], [[5, 6], [7, 8]]], numpy.int32)
// >>> arr.ndim
// 3
// >>> arr.shape
// (2, 2, 2)
// >>> arr.strides
// (16, 8, 4)
pub struct NumpyArray {
    _capsule: Py<PyAny>,
    root: NumpyArrayNode,
}

impl NumpyArray {
    #[inline(never)]
    pub fn new(
        obj: Borrowed<'_, '_, PyAny>,
        state: &State,
        opts: Opt,
    ) -> Result<Self, PyArrayError> {
        unsafe {
            let capsule = obj
                .getattr(state.array_struct_str.bind_borrowed(obj.py()))
                .unwrap();
            let array = capsule
                .cast_unchecked::<PyCapsule>()
                .pointer_checked(None)
                .unwrap()
                .cast::<PyArrayInterface>()
                .as_ptr();
            if (*array).two != 2 {
                return Err(PyArrayError::Malformed);
            }
            if (*array).flags & 0x1 != 0x1 {
                return Err(PyArrayError::NotContiguous);
            }
            let num_dimensions = (*array).nd as usize;
            if num_dimensions == 0 {
                return Err(PyArrayError::UnsupportedDataType);
            }
            match ItemType::find(array, obj, state) {
                None => Err(PyArrayError::UnsupportedDataType),
                Some(kind) => {
                    let root = if num_dimensions > 1 {
                        let mut position = Vec::with_capacity(num_dimensions);
                        NumpyArray::build(array, kind, opts, 0, &mut position)
                    } else {
                        let shape = std::slice::from_raw_parts(
                            (*array).shape.cast::<isize>(),
                            num_dimensions,
                        );
                        NumpyArrayNode::Leaf(NumpyArrayData {
                            data: (*array).data,
                            len: shape[0] as usize,
                            kind: kind,
                            opts: opts,
                        })
                    };
                    Ok(NumpyArray {
                        _capsule: capsule.unbind(),
                        root: root,
                    })
                }
            }
        }
    }

    fn build(
        array: *mut PyArrayInterface,
        kind: ItemType,
        opts: Opt,
        depth: usize,
        position: &mut Vec<isize>,
    ) -> NumpyArrayNode {
        let num_dimensions = unsafe { (*array).nd as usize };
        let shape =
            unsafe { std::slice::from_raw_parts((*array).shape.cast::<isize>(), num_dimensions) };
        let strides =
            unsafe { std::slice::from_raw_parts((*array).strides.cast::<isize>(), num_dimensions) };
        let num_children = shape[depth];
        let mut children = Vec::with_capacity(num_children as usize);
        for i in 0..num_children {
            position.push(i);
            let child = if depth < num_dimensions - 2 {
                NumpyArray::build(array, kind, opts, depth + 1, position)
            } else {
                let offset = strides
                    .iter()
                    .zip(position.iter())
                    .map(|(a, b)| a * b)
                    .sum::<isize>();
                NumpyArrayNode::Leaf(NumpyArrayData {
                    data: unsafe { (*array).data.offset(offset) },
                    len: shape[num_dimensions - 1] as usize,
                    kind: kind,
                    opts: opts,
                })
            };
            position.pop();
            children.push(child);
        }
        NumpyArrayNode::Internal(children)
    }
}

impl Serialize for NumpyArray {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.root.serialize(serializer)
    }
}

/// This mimicks the units supported by numpy's datetime64 type.
///
/// See
/// https://github.com/numpy/numpy/blob/v1.26.4/numpy/core/include/numpy/ndarraytypes.h#L244-L258
#[derive(Clone, Copy)]
enum NumpyDatetimeUnit {
    NaT,
    Years,
    Months,
    Weeks,
    Days,
    Hours,
    Minutes,
    Seconds,
    Milliseconds,
    Microseconds,
    Nanoseconds,
    Picoseconds,
    Femtoseconds,
    Attoseconds,
    Generic,
}

impl std::fmt::Display for NumpyDatetimeUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let unit = match self {
            Self::NaT => "NaT",
            Self::Years => "years",
            Self::Months => "months",
            Self::Weeks => "weeks",
            Self::Days => "days",
            Self::Hours => "hours",
            Self::Minutes => "minutes",
            Self::Seconds => "seconds",
            Self::Milliseconds => "milliseconds",
            Self::Microseconds => "microseconds",
            Self::Nanoseconds => "nanoseconds",
            Self::Picoseconds => "picoseconds",
            Self::Femtoseconds => "femtoseconds",
            Self::Attoseconds => "attoseconds",
            Self::Generic => "generic",
        };
        write!(f, "{unit}")
    }
}

enum NumpyDateTimeError {
    UnsupportedUnit(NumpyDatetimeUnit),
    Unrepresentable { unit: NumpyDatetimeUnit, val: i64 },
}

impl std::fmt::Display for NumpyDateTimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedUnit(unit) => write!(f, "unsupported numpy.datetime64 unit: {unit}"),
            Self::Unrepresentable { unit, val } => {
                write!(f, "unrepresentable numpy.datetime64: {val} {unit}")
            }
        }
    }
}

impl NumpyDatetimeUnit {
    /// Create a `NumpyDatetimeUnit` from a Python object holding a numpy array.
    ///
    /// This function must only be called with numpy arrays.
    ///
    /// We need to look inside the `obj.dtype.descr` attribute of the Python
    /// object rather than using the `descr` field of the `__array_struct__`
    /// because that field isn't populated for datetime64 arrays; see
    /// https://github.com/numpy/numpy/issues/5350.
    fn from_pyobject(obj: Borrowed<'_, '_, PyAny>, state: &State) -> Self {
        let dtype = obj
            .getattr(state.dtype_str.bind_borrowed(obj.py()))
            .unwrap();
        let descr = dtype
            .getattr(state.descr_str.bind_borrowed(obj.py()))
            .unwrap();
        unsafe {
            let item = descr
                .cast_into_unchecked::<PyList>()
                .get_item(0)
                .unwrap()
                .cast_into_unchecked::<pyo3::types::PyTuple>();
            let descr_str = item
                .get_borrowed_item(1)
                .unwrap()
                .cast_unchecked::<PyString>();
            let uni = unicode_to_str(descr_str).unwrap();

            if uni.len() < 5 {
                return Self::NaT;
            }
            // unit descriptions are found at
            // https://github.com/numpy/numpy/blob/v1.26.4/numpy/core/src/multiarray/datetime.c#L81-L98
            match &uni[4..uni.len() - 1] {
                "Y" => Self::Years,
                "M" => Self::Months,
                "W" => Self::Weeks,
                "D" => Self::Days,
                "h" => Self::Hours,
                "m" => Self::Minutes,
                "s" => Self::Seconds,
                "ms" => Self::Milliseconds,
                "us" => Self::Microseconds,
                "ns" => Self::Nanoseconds,
                "ps" => Self::Picoseconds,
                "fs" => Self::Femtoseconds,
                "as" => Self::Attoseconds,
                "generic" => Self::Generic,
                _ => unreachable!(),
            }
        }
    }

    /// Return a `NaiveDateTime` for a value in array with this unit.
    ///
    /// Returns an `Err(NumpyDateTimeError)` if the value is invalid for this unit.
    fn datetime(&self, val: i64, opts: Opt) -> Result<NaiveDateTime, NumpyDateTimeError> {
        match self {
            Self::Years => Ok(NaiveDate::from_ymd_opt(
                (val + 1970)
                    .try_into()
                    .map_err(|_| NumpyDateTimeError::Unrepresentable { unit: *self, val })?,
                1,
                1,
            )
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()),
            Self::Months => Ok(NaiveDate::from_ymd_opt(
                (val / 12 + 1970)
                    .try_into()
                    .map_err(|_| NumpyDateTimeError::Unrepresentable { unit: *self, val })?,
                (val % 12 + 1)
                    .try_into()
                    .map_err(|_| NumpyDateTimeError::Unrepresentable { unit: *self, val })?,
                1,
            )
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()),
            Self::Weeks => Ok(DateTime::from_timestamp(val * 7 * 24 * 60 * 60, 0)
                .unwrap()
                .naive_utc()),
            Self::Days => Ok(DateTime::from_timestamp(val * 24 * 60 * 60, 0)
                .unwrap()
                .naive_utc()),
            Self::Hours => Ok(DateTime::from_timestamp(val * 60 * 60, 0)
                .unwrap()
                .naive_utc()),
            Self::Minutes => Ok(DateTime::from_timestamp(val * 60, 0).unwrap().naive_utc()),
            Self::Seconds => Ok(DateTime::from_timestamp(val, 0).unwrap().naive_utc()),
            Self::Milliseconds => Ok(DateTime::from_timestamp(
                val / 1_000,
                (val % 1_000 * 1_000_000)
                    .try_into()
                    .map_err(|_| NumpyDateTimeError::Unrepresentable { unit: *self, val })?,
            )
            .unwrap()
            .naive_utc()),
            Self::Microseconds => Ok(DateTime::from_timestamp(
                val / 1_000_000,
                (val % 1_000_000 * 1_000)
                    .try_into()
                    .map_err(|_| NumpyDateTimeError::Unrepresentable { unit: *self, val })?,
            )
            .unwrap()
            .naive_utc()),
            Self::Nanoseconds => Ok(DateTime::from_timestamp(
                val / 1_000_000_000,
                (val % 1_000_000_000)
                    .try_into()
                    .map_err(|_| NumpyDateTimeError::Unrepresentable { unit: *self, val })?,
            )
            .unwrap()
            .naive_utc()),
            _ => Err(NumpyDateTimeError::UnsupportedUnit(*self)),
        }
        .map(|dt| NaiveDateTime { dt, opts })
    }
}

macro_rules! define_numpy_type {
    ($name:ident, $object_name:ident, $type:ty) => {
        #[repr(C)]
        struct $object_name {
            ob_base: PyObject,
            value: $type,
        }

        #[repr(transparent)]
        pub struct $name<'a, 'py> {
            obj: Borrowed<'a, 'py, PyAny>,
        }

        impl<'a, 'py> $name<'a, 'py> {
            pub fn new(obj: Borrowed<'a, 'py, PyAny>) -> Self {
                $name { obj }
            }
        }

        impl Serialize for $name<'_, '_> {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                let value = unsafe { (*self.obj.as_ptr().cast::<$object_name>()).value };
                value.serialize(serializer)
            }
        }
    };
}

define_numpy_type!(NumpyBool, NumpyBoolObject, bool);
define_numpy_type!(NumpyFloat32, NumpyFloat32Object, f32);
define_numpy_type!(NumpyFloat64, NumpyFloat64Object, f64);
define_numpy_type!(NumpyInt8, NumpyInt8Object, i8);
define_numpy_type!(NumpyInt16, NumpyInt16Object, i16);
define_numpy_type!(NumpyInt32, NumpyInt32Object, i32);
define_numpy_type!(NumpyInt64, NumpyInt64Object, i64);
define_numpy_type!(NumpyUint8, NumpyUint8Object, u8);
define_numpy_type!(NumpyUint16, NumpyUint16Object, u16);
define_numpy_type!(NumpyUint32, NumpyUint32Object, u32);
define_numpy_type!(NumpyUint64, NumpyUint64Object, u64);

#[repr(C)]
struct NumpyDatetime64Object {
    ob_base: PyObject,
    value: i64,
}

pub struct NumpyDatetime64<'a, 'py> {
    obj: Borrowed<'a, 'py, PyAny>,
    state: &'a State,
    opts: Opt,
}

impl<'a, 'py> NumpyDatetime64<'a, 'py> {
    pub fn new(obj: Borrowed<'a, 'py, PyAny>, state: &'a State, opts: Opt) -> Self {
        NumpyDatetime64 { obj, state, opts }
    }
}

impl Serialize for NumpyDatetime64<'_, '_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let unit = NumpyDatetimeUnit::from_pyobject(self.obj, self.state);
        let value = unsafe { (*self.obj.as_ptr().cast::<NumpyDatetime64Object>()).value };
        unit.datetime(value, self.opts)
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }
}

#[repr(C)]
struct NumpyFloat16Object {
    ob_base: PyObject,
    value: u16,
}

#[repr(transparent)]
pub struct NumpyFloat16<'a, 'py> {
    obj: Borrowed<'a, 'py, PyAny>,
}

impl<'a, 'py> NumpyFloat16<'a, 'py> {
    pub fn new(obj: Borrowed<'a, 'py, PyAny>) -> Self {
        NumpyFloat16 { obj }
    }
}

impl Serialize for NumpyFloat16<'_, '_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = unsafe { (*self.obj.as_ptr().cast::<NumpyFloat16Object>()).value };
        half::f16::from_bits(value).to_f32().serialize(serializer)
    }
}
