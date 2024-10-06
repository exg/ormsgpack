# SPDX-License-Identifier: (Apache-2.0 OR MIT)
from dataclasses import InitVar, asdict, dataclass, field
from typing import ClassVar, Optional

import msgpack
import pytest

import ormsgpack


def test_dataclass() -> None:
    """
    packb() dataclass
    """

    @dataclass
    class Dataclass:
        a: str
        b: int
        c: InitVar[str]
        d: ClassVar[str] = "cls"

    obj = Dataclass("a", 1, "")
    assert ormsgpack.packb(obj) == msgpack.packb(
        {
            "a": "a",
            "b": 1,
        }
    )


def test_dataclass_with_slots() -> None:
    """
    packb() dataclass with slots
    """

    @dataclass
    class Dataclass:
        a: str
        b: int
        c: InitVar[str]
        d: ClassVar[str] = "cls"

        __slots__ = (
            "a",
            "b",
            "c",
        )

    obj = Dataclass("a", 1, "")
    assert ormsgpack.packb(obj) == msgpack.packb(
        {
            "a": "a",
            "b": 1,
        }
    )


def test_dataclass_empty() -> None:
    """
    packb() dataclass with no attributes
    """

    @dataclass
    class Dataclass:
        pass

    assert ormsgpack.packb(Dataclass()) == msgpack.packb({})


def test_dataclass_empty_with_slots() -> None:
    """
    packb() dataclass with no attributes and slots
    """

    @dataclass
    class Dataclass:
        __slots__ = ()

    assert ormsgpack.packb(Dataclass()) == msgpack.packb({})


def test_dataclass_with_private_field() -> None:
    """
    packb() dataclass with private field
    """

    @dataclass
    class Dataclass:
        a: str
        b: int
        _c: str

    obj = Dataclass("a", 1, "")
    assert ormsgpack.packb(obj) == msgpack.packb(
        {
            "a": "a",
            "b": 1,
        }
    )


def test_dataclass_with_private_field_and_slots() -> None:
    """
    packb() dataclass with private field and slots
    """

    @dataclass
    class Dataclass:
        a: str
        b: int
        _c: str

        __slots__ = (
            "a",
            "b",
            "_c",
        )

    obj = Dataclass("a", 1, "")
    assert ormsgpack.packb(obj) == msgpack.packb(
        {
            "a": "a",
            "b": 1,
        }
    )


def test_dataclass_with_dict_and_slots() -> None:
    class Base:
        pass

    @dataclass
    class Dataclass(Base):
        a: str
        b: int

        __slots__ = (
            "a",
            "b",
        )

    obj = Dataclass("a", 1)
    assert hasattr(obj, "__dict__")
    assert ormsgpack.packb(obj) == msgpack.packb(
        {
            "a": "a",
            "b": 1,
        }
    )


def test_dataclass_recursive() -> None:
    """
    packb() dataclass recursive
    """

    @dataclass
    class Dataclass:
        a: str
        b: int
        c: Optional["Dataclass"]

    obj = Dataclass("a", 1, Dataclass("b", 2, None))
    assert ormsgpack.packb(obj) == msgpack.packb(
        {
            "a": "a",
            "b": 1,
            "c": {
                "a": "b",
                "b": 2,
                "c": None,
            },
        }
    )


def test_dataclass_circular() -> None:
    """
    packb() dataclass circular
    """

    @dataclass
    class Dataclass:
        a: str
        b: int
        c: Optional["Dataclass"]

    obj1 = Dataclass("a", 1, None)
    obj2 = Dataclass("b", 2, obj1)
    obj1.c = obj2
    with pytest.raises(ormsgpack.MsgpackEncodeError):
        ormsgpack.packb(obj1)


def test_dataclass_subclass() -> None:
    """
    packb() dataclass subclass
    """

    @dataclass
    class Dataclass:
        a: str

    @dataclass
    class Datasubclass(Dataclass):
        b: int

    obj = Datasubclass("a", 1)
    assert ormsgpack.packb(obj) == msgpack.packb(
        {
            "a": "a",
            "b": 1,
        }
    )


def test_dataclass_passthrough() -> None:
    """
    packb() dataclass passes to default with OPT_PASSTHROUGH_DATACLASS
    """

    @dataclass
    class Dataclass:
        a: str
        b: int

    obj = Dataclass("a", 1)
    with pytest.raises(ormsgpack.MsgpackEncodeError):
        ormsgpack.packb(obj, option=ormsgpack.OPT_PASSTHROUGH_DATACLASS)


def test_dataclass_passthrough_default() -> None:
    """
    packb() dataclass passes to default with OPT_PASSTHROUGH_DATACLASS
    """

    @dataclass
    class Dataclass:
        a: str
        b: int

    obj = Dataclass("a", 1)
    assert ormsgpack.packb(
        obj, option=ormsgpack.OPT_PASSTHROUGH_DATACLASS, default=asdict
    ) == msgpack.packb(
        {
            "a": "a",
            "b": 1,
        }
    )
