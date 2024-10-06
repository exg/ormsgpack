# SPDX-License-Identifier: (Apache-2.0 OR MIT)

import pytest

try:
    import xxhash
except ImportError:
    xxhash = None

import msgpack

import ormsgpack


def test_str():
    """
    str
    """
    for obj, ref in (("blah", b'"blah"'), ("東京", b'"\xe6\x9d\xb1\xe4\xba\xac"')):
        assert ormsgpack.packb(obj) == msgpack.packb(obj)
        assert ormsgpack.packb(ref) == msgpack.packb(ref)


def test_str_latin1():
    """
    str latin1
    """
    assert ormsgpack.unpackb(ormsgpack.packb("üýþÿ")) == "üýþÿ"


def test_str_long():
    """
    str long
    """
    for obj in ("aaaa" * 1024, "üýþÿ" * 1024, "好" * 1024, "�" * 1024):
        assert ormsgpack.unpackb(ormsgpack.packb(obj)) == obj


def test_str_very_long():
    """
    str long enough to trigger overflow in bytecount
    """
    for obj in ("aaaa" * 20000, "üýþÿ" * 20000, "好" * 20000, "�" * 20000):
        assert ormsgpack.unpackb(ormsgpack.packb(obj)) == obj


def test_str_replacement():
    """
    str roundtrip �
    """
    assert ormsgpack.packb("�") == msgpack.packb("�")
    assert ormsgpack.unpackb(ormsgpack.packb("�")) == "�"


def test_str_surrogates_packb():
    """
    str unicode surrogates packb()
    """
    pytest.raises(ormsgpack.MsgpackEncodeError, ormsgpack.packb, "\ud800")
    pytest.raises(ormsgpack.MsgpackEncodeError, ormsgpack.packb, "\ud83d\ude80")
    pytest.raises(ormsgpack.MsgpackEncodeError, ormsgpack.packb, "\udcff")
    pytest.raises(ormsgpack.MsgpackEncodeError, ormsgpack.packb, {"\ud83d\ude80": None})


@pytest.mark.skipif(
    xxhash is None, reason="xxhash install broken on win, python3.9, Azure"
)
def test_str_ascii():
    """
    str is ASCII but not compact
    """
    digest = xxhash.xxh32_hexdigest("12345")
    for _ in range(2):
        assert ormsgpack.unpackb(ormsgpack.packb(digest)) == "b30d56b4"


def test_bool():
    """
    bool
    """
    for obj in (True, False):
        assert ormsgpack.packb(obj) == msgpack.packb(obj)
        assert ormsgpack.unpackb(ormsgpack.packb(obj)) == obj


def test_none():
    """
    null
    """
    obj = None
    ref = b"\xc0"
    assert ormsgpack.packb(obj) == ref
    assert ormsgpack.unpackb(ref) == obj


@pytest.mark.parametrize("value", (9223372036854775807, -9223372036854775807))
def test_int_64(value):
    """
    int 64-bit
    """
    assert ormsgpack.unpackb(ormsgpack.packb(value)) == value


@pytest.mark.parametrize("value", (9223372036854775808, 18446744073709551615))
def test_uint_64(value):
    """
    uint 64-bit
    """
    assert ormsgpack.unpackb(ormsgpack.packb(value)) == value


@pytest.mark.parametrize("value", (18446744073709551616, -9223372036854775809))
def test_int_128(value):
    """
    int 128-bit
    """
    pytest.raises(ormsgpack.MsgpackEncodeError, ormsgpack.packb, value)


@pytest.mark.parametrize("value", (9223372036854775807, -9223372036854775807))
def test_int_64_passthrough(value):
    """
    int 64-bit with passthrough
    """
    assert (
        ormsgpack.unpackb(
            ormsgpack.packb(value, option=ormsgpack.OPT_PASSTHROUGH_BIG_INT)
        )
        == value
    )


@pytest.mark.parametrize("value", (9223372036854775808, 18446744073709551615))
def test_uint_64_passthrough(value):
    """
    uint 64-bit with passthrough
    """
    assert (
        ormsgpack.unpackb(
            ormsgpack.packb(value, option=ormsgpack.OPT_PASSTHROUGH_BIG_INT)
        )
        == value
    )


@pytest.mark.parametrize("value", (18446744073709551616, -9223372036854775809))
def test_int_128_passthrough(value):
    """
    int 128-bit with passthrough
    """
    result = ormsgpack.unpackb(
        ormsgpack.packb(
            value,
            option=ormsgpack.OPT_PASSTHROUGH_BIG_INT,
            default=lambda x: {"int": x.to_bytes(16, "little", signed=True)},
        )
    )
    assert list(result.keys()) == ["int"]
    assert int.from_bytes(result["int"], "little", signed=True) == value


@pytest.mark.parametrize(
    "value",
    (
        -1.1234567893,
        -1.234567893,
        -1.34567893,
        -1.4567893,
        -1.567893,
        -1.67893,
        -1.7893,
        -1.893,
        -1.3,
        1.1234567893,
        1.234567893,
        1.34567893,
        1.4567893,
        1.567893,
        1.67893,
        1.7893,
        1.893,
        1.3,
    ),
)
def test_float(value):
    """
    float
    """
    assert ormsgpack.unpackb(ormsgpack.packb(value)) == value


@pytest.mark.parametrize(
    "value",
    (
        31.245270191439438,
        -31.245270191439438,
        121.48791951161945,
        -121.48791951161945,
        100.78399658203125,
        -100.78399658203125,
    ),
)
def test_float_precision_(value):
    """
    float precision
    """
    assert ormsgpack.unpackb(ormsgpack.packb(value)) == value


@pytest.mark.parametrize(
    "value",
    (
        0.8701,
        0.0000000000000000000000000000000000000000000000000123e50,
        0.4e5,
        0.00e-00,
        0.4e-001,
        0.123456789e-12,
        1.234567890e34,
        23456789012e66,
    ),
)
def test_float_edge(value):
    """
    float edge cases
    """
    assert ormsgpack.unpackb(ormsgpack.packb(value)) == value


@pytest.mark.parametrize("value", ("1.337E40", "1.337e+40", "1337e40", "1.337E-4"))
def test_float_notation(value):
    """
    float notation
    """
    assert ormsgpack.unpackb(ormsgpack.packb(value)) == value


def test_list():
    """
    list
    """
    obj = ["a", "😊", True, {"b": 1.1}, 2]
    assert ormsgpack.unpackb(ormsgpack.packb(obj)) == obj


def test_tuple():
    """
    tuple
    """
    obj = ("a", "😊", True, {"b": 1.1}, 2)
    assert ormsgpack.unpackb(ormsgpack.packb(obj)) == list(obj)


def test_tuple_passthrough():
    """
    tuple with passthrough
    """
    obj = ("a", "😊", True, {"b": 1.1}, 2)
    result = ormsgpack.unpackb(
        ormsgpack.packb(
            obj,
            option=ormsgpack.OPT_PASSTHROUGH_TUPLE,
            default=lambda x: {"tuple": list(x)},
        )
    )
    assert list(result.keys()) == ["tuple"]
    assert tuple(result["tuple"]) == obj


def test_dict():
    """
    dict
    """
    obj = {"key": "value"}
    assert ormsgpack.unpackb(ormsgpack.packb(obj)) == obj


def test_dict_large():
    """
    dict with >512 keys
    """
    obj = {"key_%s" % idx: "value" for idx in range(513)}
    assert len(obj) == 513
    assert ormsgpack.unpackb(ormsgpack.packb(obj)) == obj


def test_dict_large_keys():
    """
    dict with keys too large to cache
    """
    obj = {"keeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeey": "value"}
    assert ormsgpack.unpackb(ormsgpack.packb(obj)) == obj


def test_dict_unicode():
    """
    dict unicode keys
    """
    obj = {"🐈": "value"}
    assert ormsgpack.unpackb(ormsgpack.packb(obj)) == obj


def test_dict_similar_keys():
    """
    unpackb() similar keys

    This was a regression in 3.4.2 caused by using
    the implementation in wy instead of wyhash.
    """
    obj = {"cf_status_firefox67": "---", "cf_status_firefox57": "verified"}
    assert ormsgpack.unpackb(ormsgpack.packb(obj)) == obj
