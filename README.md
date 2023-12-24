# ormsgpack
![PyPI](https://img.shields.io/pypi/v/ormsgpack)
![PyPI - Downloads](https://img.shields.io/pypi/dm/ormsgpack)

ormsgpack is a fast msgpack serialization library for Python derived
from [orjson](https://github.com/ijl/orjson), with native support for
various Python types.

ormsgpack supports CPython 3.9, 3.10, 3.11, 3.12 and 3.13. Releases
follow semantic versioning and serializing a new object type without
an opt-in flag is considered a breaking change.

ormsgpack is licensed under both the Apache 2.0 and MIT licenses. The
repository and issue tracker is
[github.com/aviramha/ormsgpack](https://github.com/aviramha/ormsgpack), and patches may be
submitted there. There is a
[CHANGELOG](https://github.com/aviramha/ormsgpack/blob/master/CHANGELOG.md)
available in the repository.

1. [Usage](#usage)
    1. [Install](#install)
    2. [Quickstart](#quickstart)
    4. [Serialize](#serialize)
        1. [default](#default)
        2. [option](#option)
    5. [Deserialize](#deserialize)
2. [Types](#types)
    1. [dataclass](#dataclass)
    2. [datetime](#datetime)
    3. [enum](#enum)
    4. [float](#float)
    5. [int](#int)
    6. [numpy](#numpy)
    7. [uuid](#uuid)
    8. [pydantic](#pydantic)
3. [Performance](#performance)
4. [Questions](#questions)
5. [Packaging](#packaging)
6. [License](#license)

## Usage

### Install

To install a wheel from PyPI:

```sh
pip install --upgrade "pip>=20.3" # manylinux_x_y, universal2 wheel support
pip install --upgrade ormsgpack
```

To build a wheel, see [packaging](#packaging).

### Quickstart

This is an example of serializing, with options specified, and deserializing:

```python
>>> import ormsgpack, datetime, numpy
>>> data = {
...     "type": "job",
...     "created_at": datetime.datetime(1970, 1, 1),
...     "status": "🆗",
...     "payload": numpy.array([[1, 2], [3, 4]]),
... }
>>> ormsgpack.packb(data, option=ormsgpack.OPT_NAIVE_UTC | ormsgpack.OPT_SERIALIZE_NUMPY)
b'\x84\xa4type\xa3job\xaacreated_at\xb91970-01-01T00:00:00+00:00\xa6status\xa4\xf0\x9f\x86\x97\xa7payload\x92\x92\x01\x02\x92\x03\x04'
>>> ormsgpack.unpackb(_)
{'type': 'job', 'created_at': '1970-01-01T00:00:00+00:00', 'status': '🆗', 'payload': [[1, 2], [3, 4]]}
```

### Serialize

```python
def packb(
    __obj: Any,
    default: Optional[Callable[[Any], Any]] = ...,
    option: Optional[int] = ...,
) -> bytes: ...
```

`packb()` serializes Python objects to msgpack.

It natively serializes
`bytes`, `str`, `dict`, `list`, `tuple`, `int`, `float`, `bool`,
`dataclasses.dataclass`, `typing.TypedDict`, `datetime.datetime`,
`datetime.date`, `datetime.time`, `uuid.UUID`, `numpy.ndarray`, and
`None` instances. It supports arbitrary types through `default`. It
serializes subclasses of `str`, `int`, `dict`, `list`,
`dataclasses.dataclass`, and `enum.Enum`. It does not serialize subclasses
of `tuple` to avoid serializing `namedtuple` objects as arrays. To avoid
serializing subclasses, specify the option `ormsgpack.OPT_PASSTHROUGH_SUBCLASS`.

The output is a `bytes` object.

The global interpreter lock (GIL) is held for the duration of the call.

It raises `MsgpackEncodeError` on an unsupported type. This exception message
describes the invalid object with the error message
`Type is not msgpack serializable: ...`. To fix this, specify
[default](#default).

It raises `MsgpackEncodeError` on a `str` that contains invalid UTF-8.

It raises `MsgpackEncodeError` if a `dict` has a key of a type other than `str` or `bytes`,
unless `OPT_NON_STR_KEYS` is specified.

It raises `MsgpackEncodeError` if the output of `default` recurses to handling by
`default` more than 254 levels deep.

It raises `MsgpackEncodeError` on circular references.

It raises `MsgpackEncodeError`  if a `tzinfo` on a datetime object is
unsupported.

`MsgpackEncodeError` is a subclass of `TypeError`.

#### default

To serialize a subclass or arbitrary types, specify `default` as a
callable that returns a supported type. `default` may be a function,
lambda, or callable class instance. To specify that a type was not
handled by `default`, raise an exception such as `TypeError`.

```python
>>> import ormsgpack, decimal
>>> def default(obj):
...     if isinstance(obj, decimal.Decimal):
...         return str(obj)
...     raise TypeError
...
>>> ormsgpack.packb(decimal.Decimal("0.0842389659712649442845"))
TypeError: Type is not msgpack serializable: decimal.Decimal
>>> ormsgpack.packb(decimal.Decimal("0.0842389659712649442845"), default=default)
b'\xb80.0842389659712649442845'
>>> ormsgpack.packb({1, 2}, default=default)
TypeError: Type is not msgpack serializable: set
```

The `default` callable may return an object that itself
must be handled by `default` up to 254 times before an exception
is raised.

It is important that `default` raise an exception if a type cannot be handled.
Python otherwise implicitly returns `None`, which appears to the caller
like a legitimate value and is serialized:

```python
>>> import ormsgpack, decimal
>>> def default(obj):
...     if isinstance(obj, decimal.Decimal):
...         return str(obj)
...
>>> ormsgpack.packb({"set":{1, 2}}, default=default)
b'\x81\xa3set\xc0'
>>> ormsgpack.unpackb(_)
{'set': None}
```

To serialize a type as a MessagePack extension type, return an
`ormsgpack.Ext` object. The instantiation arguments are an integer in
the range `[0, 127]` and a `bytes` object, defining the type and
value, respectively.

```python
>>> import ormsgpack, decimal
>>> def default(obj):
...     if isinstance(obj, decimal.Decimal):
...         return ormsgpack.Ext(0, str(obj).encode())
...     raise TypeError
...
>>> ormsgpack.packb(decimal.Decimal("0.0842389659712649442845"), default=default)
b'\xc7\x18\x000.0842389659712649442845'
```

#### option

To modify how data is serialized, specify `option`. Each `option` is an integer
constant in `ormsgpack`. To specify multiple options, mask them together, e.g.,
`option=ormsgpack.OPT_NON_STR_KEYS | ormsgpack.OPT_NAIVE_UTC`.

##### OPT_NAIVE_UTC

Serialize `datetime.datetime` objects without a `tzinfo` and `numpy.datetime64`
objects as UTC. This has no effect on `datetime.datetime` objects that have
`tzinfo` set.

```python
>>> import ormsgpack, datetime
>>> ormsgpack.packb(
...     datetime.datetime(1970, 1, 1, 0, 0, 0),
... )
b'\xb31970-01-01T00:00:00'
>>> ormsgpack.unpackb(_)
'1970-01-01T00:00:00'
>>> ormsgpack.packb(
...     datetime.datetime(1970, 1, 1, 0, 0, 0),
...     option=ormsgpack.OPT_NAIVE_UTC,
... )
b'\xb91970-01-01T00:00:00+00:00'
>>> ormsgpack.unpackb(_)
'1970-01-01T00:00:00+00:00'
```

##### OPT_NON_STR_KEYS

Serialize `dict` keys of type other than `str`. This allows `dict` keys
to be one of `str`, `int`, `float`, `bool`, `None`, `datetime.datetime`,
`datetime.date`, `datetime.time`, `enum.Enum`, and `uuid.UUID`.

```python
>>> import ormsgpack, datetime, uuid
>>> ormsgpack.packb(
...     {uuid.UUID("7202d115-7ff3-4c81-a7c1-2a1f067b1ece"): [1, 2, 3]},
...     option=ormsgpack.OPT_NON_STR_KEYS,
... )
b'\x81\xd9$7202d115-7ff3-4c81-a7c1-2a1f067b1ece\x93\x01\x02\x03'
>>> ormsgpack.unpackb(_)
{'7202d115-7ff3-4c81-a7c1-2a1f067b1ece': [1, 2, 3]}
>>> ormsgpack.packb(
...     {datetime.datetime(1970, 1, 1, 0, 0, 0): [1, 2, 3]},
...     option=ormsgpack.OPT_NON_STR_KEYS | ormsgpack.OPT_NAIVE_UTC,
... )
b'\x81\xb91970-01-01T00:00:00+00:00\x93\x01\x02\x03'
>>> ormsgpack.unpackb(_)
{'1970-01-01T00:00:00+00:00': [1, 2, 3]}
```

These types are generally serialized how they would be as
values, e.g., `datetime.datetime` is still an RFC 3339 string and respects
options affecting it.

This option has the risk of creating duplicate keys. This is because non-`str`
objects may serialize to the same `str` as an existing key, e.g.,
`{"1970-01-01T00:00:00+00:00": true, datetime.datetime(1970, 1, 1, 0, 0, 0): false}`.
The last key to be inserted to the `dict` will be serialized last and a msgpack deserializer will presumably take the last
occurrence of a key (in the above, `false`). The first value will be lost.

This option is not compatible with `ormsgpack.OPT_SORT_KEYS`.

##### OPT_OMIT_MICROSECONDS

Do not serialize the microsecond component of `datetime.datetime`,
`datetime.time` and `numpy.datetime64` instances.

```python
>>> import ormsgpack, datetime
>>> ormsgpack.packb(
...     datetime.datetime(1970, 1, 1, 0, 0, 0, 1),
... )
b'\xba1970-01-01T00:00:00.000001'
>>> ormsgpack.unpackb(_)
'1970-01-01T00:00:00.000001'
>>> ormsgpack.packb(
...     datetime.datetime(1970, 1, 1, 0, 0, 0, 1),
...     option=ormsgpack.OPT_OMIT_MICROSECONDS,
... )
b'\xb31970-01-01T00:00:00'
>>> ormsgpack.unpackb(_)
'1970-01-01T00:00:00'
```

##### OPT_PASSTHROUGH_BIG_INT

Enables passthrough of big (Python) ints. By setting this option, one can set a `default` function for ints larger than 63 bits, smaller ints are still serialized efficiently.

```python
>>> import ormsgpack
>>> ormsgpack.packb(
...     2**65,
... )
TypeError: Integer exceeds 64-bit range
>>> ormsgpack.packb(
...     2**65,
...     option=ormsgpack.OPT_PASSTHROUGH_BIG_INT,
...     default=lambda _: {"type": "bigint", "value": str(_) }
... )
b'\x82\xa4type\xa6bigint\xa5value\xb436893488147419103232'
>>> ormsgpack.unpackb(_)
{'type': 'bigint', 'value': '36893488147419103232'}
```

##### OPT_PASSTHROUGH_DATACLASS

Passthrough `dataclasses.dataclass` instances to `default`. This allows
customizing their output but is much slower.


```python
>>> import ormsgpack, dataclasses
>>> @dataclasses.dataclass
... class User:
...     id: str
...     name: str
...     password: str
...
>>> def default(obj):
...     if isinstance(obj, User):
...         return {"id": obj.id, "name": obj.name}
...     raise TypeError
...
>>> ormsgpack.packb(User("3b1", "asd", "zxc"))
b'\x83\xa2id\xa33b1\xa4name\xa3asd\xa8password\xa3zxc'
>>> ormsgpack.packb(User("3b1", "asd", "zxc"), option=ormsgpack.OPT_PASSTHROUGH_DATACLASS)
TypeError: Type is not msgpack serializable: User
>>> ormsgpack.packb(
...     User("3b1", "asd", "zxc"),
...     option=ormsgpack.OPT_PASSTHROUGH_DATACLASS,
...     default=default,
... )
b'\x82\xa2id\xa33b1\xa4name\xa3asd'
```

##### OPT_PASSTHROUGH_DATETIME

Passthrough `datetime.datetime`, `datetime.date`, and `datetime.time` instances
to `default`. This allows serializing datetimes to a custom format, e.g.,
HTTP dates:

```python
>>> import ormsgpack, datetime
>>> def default(obj):
...     if isinstance(obj, datetime.datetime):
...         return obj.strftime("%a, %d %b %Y %H:%M:%S GMT")
...     raise TypeError
...
>>> ormsgpack.packb({"created_at": datetime.datetime(1970, 1, 1)})
b'\x81\xaacreated_at\xb31970-01-01T00:00:00'
>>> ormsgpack.packb({"created_at": datetime.datetime(1970, 1, 1)}, option=ormsgpack.OPT_PASSTHROUGH_DATETIME)
TypeError: Type is not msgpack serializable: datetime.datetime
>>> ormsgpack.packb(
...     {"created_at": datetime.datetime(1970, 1, 1)},
...     option=ormsgpack.OPT_PASSTHROUGH_DATETIME,
...     default=default,
... )
b'\x81\xaacreated_at\xbdThu, 01 Jan 1970 00:00:00 GMT'
```

This does not affect datetimes in `dict` keys if using OPT_NON_STR_KEYS.

##### OPT_PASSTHROUGH_SUBCLASS

Passthrough subclasses of builtin types to `default`.

```python
>>> import ormsgpack
>>> class Secret(str):
...     pass
...
>>> def default(obj):
...     if isinstance(obj, Secret):
...         return "******"
...     raise TypeError
...
>>> ormsgpack.packb(Secret("zxc"))
b'\xa3zxc'
>>> ormsgpack.packb(Secret("zxc"), option=ormsgpack.OPT_PASSTHROUGH_SUBCLASS)
TypeError: Type is not msgpack serializable: Secret
>>> ormsgpack.packb(Secret("zxc"), option=ormsgpack.OPT_PASSTHROUGH_SUBCLASS, default=default)
b'\xa6******'
```

This does not affect serializing subclasses as `dict` keys if using
OPT_NON_STR_KEYS.

##### OPT_PASSTHROUGH_TUPLE

Passthrough tuples to `default`.

```python
>>> import ormsgpack
>>> ormsgpack.packb(
...     (9193, "test", 42),
... )
b'\x93\xcd#\xe9\xa4test*'
>>> ormsgpack.unpackb(_)
[9193, 'test', 42]
>>> ormsgpack.packb(
...     (9193, "test", 42),
...     option=ormsgpack.OPT_PASSTHROUGH_TUPLE,
...     default=lambda _: {"type": "tuple", "value": list(_)}
... )
b'\x82\xa4type\xa5tuple\xa5value\x93\xcd#\xe9\xa4test*'
>>> ormsgpack.unpackb(_)
{'type': 'tuple', 'value': [9193, 'test', 42]}
```

##### OPT_SERIALIZE_NUMPY

Serialize `numpy.ndarray` instances. For more, see
[numpy](#numpy).

##### OPT_SERIALIZE_PYDANTIC
Serialize `pydantic.BaseModel` instances.

##### OPT_SORT_KEYS

Serialize `dict` keys and pydantic model fields in sorted order. The default
is to serialize in an unspecified order.

This can be used to ensure the order is deterministic for hashing or tests.
It has a substantial performance penalty and is not recommended in general.

```python
>>> import ormsgpack
>>> ormsgpack.packb({"b": 1, "c": 2, "a": 3})
b'\x83\xa1b\x01\xa1c\x02\xa1a\x03'
>>> ormsgpack.packb({"b": 1, "c": 2, "a": 3}, option=ormsgpack.OPT_SORT_KEYS)
b'\x83\xa1a\x03\xa1b\x01\xa1c\x02'
```

The sorting is not collation/locale-aware:

```python
>>> import ormsgpack
>>> ormsgpack.packb({"a": 1, "ä": 2, "A": 3}, option=ormsgpack.OPT_SORT_KEYS)
b'\x83\xa1A\x03\xa1a\x01\xa2\xc3\xa4\x02'
```

`dataclass` also serialize as maps but this has no effect on them.

##### OPT_UTC_Z

Serialize a UTC timezone on `datetime.datetime` and `numpy.datetime64` instances
as `Z` instead of `+00:00`.

```python
>>> import ormsgpack, datetime
>>> ormsgpack.packb(
...     datetime.datetime(1970, 1, 1, 0, 0, 0, tzinfo=datetime.timezone.utc),
... )
b'\xb91970-01-01T00:00:00+00:00'
>>> ormsgpack.packb(
...     datetime.datetime(1970, 1, 1, 0, 0, 0, tzinfo=datetime.timezone.utc),
...     option=ormsgpack.OPT_UTC_Z
... )
b'\xb41970-01-01T00:00:00Z'
```

### Deserialize
```python
def unpackb(
    __obj: Union[bytes, bytearray, memoryview],
    /,
    ext_hook: Optional[Callable[[int, bytes], Any]] = ...,
    option: Optional[int] = ...,
) -> Any: ...
```

`unpackb()` deserializes msgpack to Python objects. It deserializes to `dict`,
`list`, `int`, `float`, `str`, `bool`, `bytes` and `None` objects.

`bytes`, `bytearray`, `memoryview` input are accepted.

ormsgpack maintains a cache of map keys for the duration of the process. This
causes a net reduction in memory usage by avoiding duplicate strings. The
keys must be at most 64 bytes to be cached and 512 entries are stored.

The global interpreter lock (GIL) is held for the duration of the call.

It raises `MsgpackDecodeError` if given an invalid type or invalid
msgpack.

`MsgpackDecodeError` is a subclass of `ValueError`.

#### ext_hook

To deserialize extension types, specify the optional `ext_hook`
argument. The value should be a callable and is invoked with the
extension type and value as arguments.

```python
>>> import ormsgpack, decimal
>>> def ext_hook(tag, data):
...     if tag == 0:
...         return decimal.Decimal(data.decode())
...     raise TypeError
...
>>> ormsgpack.packb(
...     ormsgpack.Ext(0, str(decimal.Decimal("0.0842389659712649442845")).encode())
... )
b'\xc7\x18\x000.0842389659712649442845'
>>> ormsgpack.unpackb(_, ext_hook=ext_hook)
Decimal('0.0842389659712649442845'
```

#### option
`unpackb()` supports the `OPT_NON_STR_KEYS` option, that is similar to original msgpack's `strict_map_key=False`.
Be aware that this option is considered unsafe and disabled by default in msgpack due to possibility of HashDoS.

## Types

### dataclass

ormsgpack serializes instances of `dataclasses.dataclass` natively. It serializes
instances 40-50x as fast as other libraries and avoids a severe slowdown seen
in other libraries compared to serializing `dict`.

It is supported to pass all variants of dataclasses, including dataclasses
using `__slots__`, frozen dataclasses, those with optional or default
attributes, and subclasses. There is a performance benefit to not
using `__slots__`.

Dataclasses are serialized as maps, with every attribute serialized and in
the order given on class definition:

```python
>>> import dataclasses, ormsgpack, typing
>>> @dataclasses.dataclass
... class Member:
...     id: int
...     active: bool = dataclasses.field(default=False)
...
>>> @dataclasses.dataclass
... class Object:
...     id: int
...     name: str
...     members: typing.List[Member]
...
>>> ormsgpack.packb(Object(1, "a", [Member(1, True), Member(2)]))
b'\x83\xa2id\x01\xa4name\xa1a\xa7members\x92\x82\xa2id\x01\xa6active\xc3\x82\xa2id\x02\xa6active\xc2'
```

### datetime

ormsgpack serializes `datetime.datetime` objects to
[RFC 3339](https://tools.ietf.org/html/rfc3339) format,
e.g., "1970-01-01T00:00:00+00:00". This is a subset of ISO 8601 and is
compatible with `isoformat()` in the standard library.

```python
>>> import ormsgpack, datetime, zoneinfo
>>> ormsgpack.packb(
...     datetime.datetime(2018, 12, 1, 2, 3, 4, 9, tzinfo=zoneinfo.ZoneInfo('Australia/Adelaide'))
... )
b'\xd9 2018-12-01T02:03:04.000009+10:30'
>>> ormsgpack.unpackb(_)
'2018-12-01T02:03:04.000009+10:30'
>>> ormsgpack.packb(
...     datetime.datetime.fromtimestamp(4123518902).replace(tzinfo=datetime.timezone.utc)
... )
b'\xb92100-09-02T00:55:02+00:00'
>>> ormsgpack.unpackb(_)
'2100-09-02T00:55:02+00:00'
>>> ormsgpack.packb(
...     datetime.datetime.fromtimestamp(4123518902)
... )
b'\xb32100-09-02T00:55:02'
>>> ormsgpack.unpackb(_)
'2100-09-02T00:55:02'
```

`datetime.datetime` supports instances with a `tzinfo` that is `None`,
`datetime.timezone.utc`, a timezone instance from the python3.9+ `zoneinfo`
module, or a timezone instance from the third-party `pendulum`, `pytz`, or
`dateutil`/`arrow` libraries.

`datetime.time` objects must not have a `tzinfo`.

```python
>>> import ormsgpack, datetime
>>> ormsgpack.packb(datetime.time(12, 0, 15, 290))
b'\xaf12:00:15.000290'
>>> ormsgpack.unpackb(_)
'12:00:15.000290'
```

`datetime.date` objects will always serialize.

```python
>>> import ormsgpack, datetime
>>> ormsgpack.packb(datetime.date(1900, 1, 2))
b'\xaa1900-01-02'
>>> ormsgpack.unpackb(_)
'1900-01-02'
```

Errors with `tzinfo` result in `MsgpackEncodeError` being raised.

To disable serialization of `datetime` objects specify the option
`ormsgpack.OPT_PASSTHROUGH_DATETIME`.

To use "Z" suffix instead of "+00:00" to indicate UTC ("Zulu") time, use the option
`ormsgpack.OPT_UTC_Z`.

To assume datetimes without timezone are UTC, use the option `ormsgpack.OPT_NAIVE_UTC`.

### enum

ormsgpack serializes enums natively. Options apply to their values.

```python
>>> import enum, datetime, ormsgpack
>>> class DatetimeEnum(enum.Enum):
...     EPOCH = datetime.datetime(1970, 1, 1, 0, 0, 0)
...
>>> ormsgpack.packb(DatetimeEnum.EPOCH)
b'\xb31970-01-01T00:00:00'
>>> ormsgpack.unpackb(_)
'1970-01-01T00:00:00'
>>> ormsgpack.packb(DatetimeEnum.EPOCH, option=ormsgpack.OPT_NAIVE_UTC)
b'\xb91970-01-01T00:00:00+00:00'
>>> ormsgpack.unpackb(_)
'1970-01-01T00:00:00+00:00'
```

Enums with members that are not supported types can be serialized using
`default`:

```python
>>> import enum, ormsgpack
>>> class Custom:
...     def __init__(self, val):
...         self.val = val
...
>>> def default(obj):
...     if isinstance(obj, Custom):
...         return obj.val
...     raise TypeError
...
>>> class CustomEnum(enum.Enum):
...     ONE = Custom(1)
...
>>> ormsgpack.packb(CustomEnum.ONE, default=default)
b'\x01'
>>> ormsgpack.unpackb(_)
1
```

### float

ormsgpack serializes and deserializes double precision floats with no loss of
precision and consistent rounding.

### int

ormsgpack serializes and deserializes 64-bit integers by default. The range
supported is a signed 64-bit integer's minimum (-9223372036854775807) to
an unsigned 64-bit integer's maximum (18446744073709551615).

### numpy

ormsgpack natively serializes `numpy.ndarray` and individual
`numpy.float64`, `numpy.float32`, `numpy.float16`,
`numpy.int64`, `numpy.int32`, `numpy.int16`, `numpy.int8`,
`numpy.uint64`, `numpy.uint32`, `numpy.uint16`, `numpy.uint8`,
`numpy.uintp`, `numpy.intp`, `numpy.datetime64`, and `numpy.bool`
instances.

`numpy.datetime64` instances are serialized as RFC 3339 strings.

ormsgpack is faster than all compared libraries at serializing
numpy instances. Serializing numpy data requires specifying
`option=ormsgpack.OPT_SERIALIZE_NUMPY`.

```python
>>> import ormsgpack, numpy
>>> ormsgpack.packb(
...     numpy.array([[1, 2, 3], [4, 5, 6]]),
...     option=ormsgpack.OPT_SERIALIZE_NUMPY,
... )
b'\x92\x93\x01\x02\x03\x93\x04\x05\x06'
>>> ormsgpack.unpackb(_)
[[1, 2, 3], [4, 5, 6]]
```

The array must be a contiguous C array (`C_CONTIGUOUS`) and one of the
supported datatypes.

If an array is not a contiguous C array or contains an supported datatype,
ormsgpack falls through to `default`. In `default`, `obj.tolist()` can be
specified. If an array is malformed, which is not expected,
`ormsgpack.MsgpackEncodeError` is raised.

### uuid

ormsgpack serializes `uuid.UUID` instances to
[RFC 4122](https://tools.ietf.org/html/rfc4122) format, e.g.,
"f81d4fae-7dec-11d0-a765-00a0c91e6bf6".

```python
>>> import ormsgpack, uuid
>>> ormsgpack.packb(uuid.UUID('f81d4fae-7dec-11d0-a765-00a0c91e6bf6'))
b'\xd9$f81d4fae-7dec-11d0-a765-00a0c91e6bf6'
>>> ormsgpack.unpackb(_)
'f81d4fae-7dec-11d0-a765-00a0c91e6bf6'
>>> ormsgpack.packb(uuid.uuid5(uuid.NAMESPACE_DNS, "python.org"))
b'\xd9$886313e1-3b8a-5372-9b90-0c9aee199e5d'
>>> ormsgpack.unpackb(_)
'886313e1-3b8a-5372-9b90-0c9aee199e5d
```

### Pydantic
ormsgpack serializes `pydantic.BaseModel` instances natively, with
[duck-typing](https://docs.pydantic.dev/2.10/concepts/serialization/#serializing-with-duck-typing).
This is equivalent to serializing
`model.model_dump(serialize_as_any=True)` with Pydantic V2 or
`model.dict()`with Pydantic V1.

## Performance

The tables below show the results of benchmarks on real and synthetic data:

* canada - coordinates of the Canadian border in GeoJSON format, containing floats and arrays
* citm_catalog - concert data, containing nested dictionaries of strings and arrays of integers
* github - a GitHub activity feed, containing dictionaries of strings and arrays of dictionaries
* twitter - results of a search on Twitter for "一", containing CJK strings, dictionaries of strings and arrays of dictionaries
* a list of 2000 dataclasses
* a `numpy.ndarray` of random `numpy.bool_` values with dimensions `(100000, 200)`
* a `numpy.ndarray` of random `numpy.float64` values with dimensions`(50000, 100)`
* a `numpy.ndarray` of random `numpy.int32` values with dimensions `(100000, 100)`
* a list of 2000 pydantic models

With msgpack, dataclasses, numpy arrays and pydantic models are converted to builtin types using `default`.

### Data

### Reproducing

The above was measured using Python 3.11.2 on Debian 12 (x86_64) with ormsgpack 1.6.0 and msgpack 1.1.0.

The latency results can be reproduced using `./scripts/benchmark.py`.

## Questions

### Why can't I install it from PyPI?

Probably `pip` needs to be upgraded to version 20.3 or later to support
the latest manylinux_x_y or universal2 wheel formats.

### Will it deserialize to dataclasses, UUIDs, decimals, etc or support object_hook?

No. This requires a schema specifying what types are expected and how to
handle errors etc. This is addressed by data validation libraries a
level above this.

## Packaging

To package ormsgpack requires [Rust](https://www.rust-lang.org/) 1.70
or newer and the [maturin](https://github.com/PyO3/maturin) build
tool. The default feature `unstable-simd` enables the usage of SIMD
operations and requires nightly Rust. The recommended build command
is:

```sh
maturin build --release --strip
```

ormsgpack is tested on Linux/amd64, Linux/aarch64, Linux/armv7, macOS/amd64 and Windows/amd64.

There are no runtime dependencies other than libc.

## License

orjson was written by ijl <<ijl@mailbox.org>>, copyright 2018 - 2021, licensed
under both the Apache 2 and MIT licenses.

ormsgpack was forked from orjson by Aviram Hassan and is now maintained by Emanuele Giaquinta (@exg), licensed
same as orjson.
