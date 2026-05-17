// SPDX-License-Identifier: (Apache-2.0 OR MIT)

use crate::ffi::PyObjectWithType;
use crate::ffi::*;
use crate::opt::*;
use crate::serialize::datetimelike::{DateLike, DateTimeLike, TimeLike};
use serde::ser::{Serialize, Serializer};
use serde_bytes::Bytes;

pub struct State {
    pub normalize_str: OwnedPyObject,
    pub utcoffset_str: OwnedPyObject,
}

impl State {
    #[cold]
    pub fn new() -> Option<Self> {
        Some(Self {
            normalize_str: OwnedPyObject::try_intern(c"normalize")?,
            utcoffset_str: OwnedPyObject::try_intern(c"utcoffset")?,
        })
    }
}

#[repr(transparent)]
pub struct Date<'a> {
    obj: BorrowedPyObject<'a>,
}

impl<'a> Date<'a> {
    #[inline]
    pub fn try_new(obj: PyObjectWithType<'a>) -> Option<Self> {
        let datetime_api = unsafe { *pyo3::ffi::PyDateTimeAPI() };
        if obj.get_type_ptr() == datetime_api.DateType {
            Some(Self {
                obj: obj.as_borrowed(),
            })
        } else {
            None
        }
    }
}

impl DateLike for Date<'_> {
    fn year(&self) -> i32 {
        unsafe { pyo3::ffi::PyDateTime_GET_YEAR(self.obj.as_ptr()) as i32 }
    }

    fn month(&self) -> i32 {
        unsafe { pyo3::ffi::PyDateTime_GET_MONTH(self.obj.as_ptr()) as i32 }
    }

    fn day(&self) -> i32 {
        unsafe { pyo3::ffi::PyDateTime_GET_DAY(self.obj.as_ptr()) as i32 }
    }
}

impl Serialize for Date<'_> {
    #[inline(never)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut cursor = std::io::Cursor::new([0u8; 32]);
        DateLike::write_rfc3339(self, &mut cursor).unwrap();
        let len = cursor.position() as usize;
        let value = unsafe { std::str::from_utf8_unchecked(&cursor.get_ref()[0..len]) };
        serializer.serialize_str(value)
    }
}

pub enum TimeError {
    HasTimezone,
}

impl std::fmt::Display for TimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HasTimezone => write!(f, "datetime.time must not have tzinfo set"),
        }
    }
}

pub struct Time<'a> {
    obj: BorrowedPyObject<'a>,
    opts: Opt,
}

impl<'a> Time<'a> {
    #[inline]
    pub fn try_new(obj: PyObjectWithType<'a>, opts: Opt) -> Result<Option<Self>, TimeError> {
        let datetime_api = unsafe { *pyo3::ffi::PyDateTimeAPI() };
        if obj.get_type_ptr() != datetime_api.TimeType {
            return Ok(None);
        }
        let tzinfo = unsafe { pyo3::ffi::PyDateTime_TIME_GET_TZINFO(obj.as_ptr()) };
        if tzinfo != unsafe { pyo3::ffi::Py_None() } {
            return Err(TimeError::HasTimezone);
        }
        Ok(Some(Self {
            obj: obj.as_borrowed(),
            opts,
        }))
    }
}

impl TimeLike for Time<'_> {
    fn hour(&self) -> i32 {
        unsafe { pyo3::ffi::PyDateTime_TIME_GET_HOUR(self.obj.as_ptr()) as i32 }
    }

    fn minute(&self) -> i32 {
        unsafe { pyo3::ffi::PyDateTime_TIME_GET_MINUTE(self.obj.as_ptr()) as i32 }
    }

    fn second(&self) -> i32 {
        unsafe { pyo3::ffi::PyDateTime_TIME_GET_SECOND(self.obj.as_ptr()) as i32 }
    }

    fn microsecond(&self) -> i32 {
        unsafe { pyo3::ffi::PyDateTime_TIME_GET_MICROSECOND(self.obj.as_ptr()) as i32 }
    }
}

impl Serialize for Time<'_> {
    #[inline(never)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut cursor = std::io::Cursor::new([0u8; 32]);
        TimeLike::write_rfc3339(self, &mut cursor, self.opts).unwrap();
        let len = cursor.position() as usize;
        let value = unsafe { std::str::from_utf8_unchecked(&cursor.get_ref()[0..len]) };
        serializer.serialize_str(value)
    }
}

pub enum DateTimeError {
    LibraryUnsupported,
}

impl std::fmt::Display for DateTimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LibraryUnsupported => write!(f, "datetime's timezone library is not supported: use datetime.timezone.utc, pendulum, pytz, or dateutil"),
        }
    }
}

unsafe fn utcoffset(
    obj: BorrowedPyObject<'_>,
    state: &State,
) -> Result<Option<i32>, DateTimeError> {
    let ptr = obj.as_ptr();
    let tzinfo = BorrowedPyObject::from_ptr(pyo3::ffi::PyDateTime_DATE_GET_TZINFO(ptr)).unwrap();
    if tzinfo.as_ptr() == unsafe { pyo3::ffi::Py_None() } {
        return Ok(None);
    }
    let maybe_delta = if tzinfo.hasattr(state.normalize_str.as_borrowed()) {
        // pytz
        let maybe_normalized = tzinfo.call_method_one_arg(state.normalize_str.as_borrowed(), obj);
        if let Some(normalized) = maybe_normalized {
            normalized.call_method_no_args(state.utcoffset_str.as_borrowed())
        } else {
            None
        }
    } else {
        tzinfo.call_method_one_arg(state.utcoffset_str.as_borrowed(), obj)
    };
    let Some(delta) = maybe_delta else {
        pyo3::ffi::PyErr_Clear();
        return Err(DateTimeError::LibraryUnsupported);
    };
    let day = pyo3::ffi::PyDateTime_DELTA_GET_DAYS(delta.as_ptr());
    let second = pyo3::ffi::PyDateTime_DELTA_GET_SECONDS(delta.as_ptr());
    let offset = if day == -1 {
        // datetime.timedelta(days=-1, seconds=68400) -> -05:00
        -86400 + second
    } else {
        // datetime.timedelta(seconds=37800) -> +10:30
        second
    };
    Ok(Some(offset))
}

pub struct DateTime<'a> {
    obj: BorrowedPyObject<'a>,
    opts: Opt,
    offset: Option<i32>,
}

impl<'a> DateTime<'a> {
    #[inline]
    pub fn try_new(
        obj: PyObjectWithType<'a>,
        state: &State,
        opts: Opt,
    ) -> Result<Option<Self>, DateTimeError> {
        let datetime_api = unsafe { *pyo3::ffi::PyDateTimeAPI() };
        if obj.get_type_ptr() != datetime_api.DateTimeType {
            return Ok(None);
        }
        let offset = unsafe { utcoffset(obj.as_borrowed(), state)? };
        Ok(Some(Self {
            obj: obj.as_borrowed(),
            opts,
            offset,
        }))
    }
}

impl DateLike for DateTime<'_> {
    fn year(&self) -> i32 {
        unsafe { pyo3::ffi::PyDateTime_GET_YEAR(self.obj.as_ptr()) as i32 }
    }

    fn month(&self) -> i32 {
        unsafe { pyo3::ffi::PyDateTime_GET_MONTH(self.obj.as_ptr()) as i32 }
    }

    fn day(&self) -> i32 {
        unsafe { pyo3::ffi::PyDateTime_GET_DAY(self.obj.as_ptr()) as i32 }
    }
}

impl TimeLike for DateTime<'_> {
    fn hour(&self) -> i32 {
        unsafe { pyo3::ffi::PyDateTime_DATE_GET_HOUR(self.obj.as_ptr()) as i32 }
    }

    fn minute(&self) -> i32 {
        unsafe { pyo3::ffi::PyDateTime_DATE_GET_MINUTE(self.obj.as_ptr()) as i32 }
    }

    fn second(&self) -> i32 {
        unsafe { pyo3::ffi::PyDateTime_DATE_GET_SECOND(self.obj.as_ptr()) as i32 }
    }

    fn microsecond(&self) -> i32 {
        unsafe { pyo3::ffi::PyDateTime_DATE_GET_MICROSECOND(self.obj.as_ptr()) as i32 }
    }
}

impl DateTimeLike for DateTime<'_> {
    fn offset(&self) -> Option<i32> {
        self.offset
    }

    fn to_utc_datetime(&self) -> chrono::DateTime<chrono::Utc> {
        let offset = chrono::FixedOffset::east_opt(self.offset.unwrap_or_default()).unwrap();
        chrono::NaiveDateTime::new(
            chrono::NaiveDate::from_ymd_opt(self.year(), self.month() as u32, self.day() as u32)
                .unwrap(),
            chrono::NaiveTime::from_hms_micro_opt(
                self.hour() as u32,
                self.minute() as u32,
                self.second() as u32,
                self.microsecond() as u32,
            )
            .unwrap(),
        )
        .and_local_timezone(offset)
        .unwrap()
        .into()
    }
}

impl Serialize for DateTime<'_> {
    #[inline(never)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut cursor = std::io::Cursor::new([0u8; 32]);
        if self.opts & DATETIME_AS_TIMESTAMP_EXT != 0
            && (self.offset().is_some() || self.opts & NAIVE_UTC != 0)
        {
            DateTimeLike::write_timestamp(self, &mut cursor).unwrap();
            let len = cursor.position() as usize;
            let timestamp = &cursor.get_ref()[0..len];
            serializer.serialize_newtype_variant("", 128, "", Bytes::new(timestamp))
        } else {
            DateTimeLike::write_rfc3339(self, &mut cursor, self.opts).unwrap();
            let len = cursor.position() as usize;
            let value = unsafe { std::str::from_utf8_unchecked(&cursor.get_ref()[0..len]) };
            serializer.serialize_str(value)
        }
    }
}
