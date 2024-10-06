# SPDX-License-Identifier: (Apache-2.0 OR MIT)

import uuid

import msgpack
import pytest

import ormsgpack


class Custom:
    def __init__(self) -> None:
        self.name = uuid.uuid4().hex

    def __str__(self) -> str:
        return f"{self.__class__.__name__}({self.name})"


def test_default_not_callable() -> None:
    """
    packb() default not callable
    """
    with pytest.raises(ormsgpack.MsgpackEncodeError) as exc_info:
        ormsgpack.packb(Custom(), default=NotImplementedError)
    assert str(exc_info.value) == "default serializer exceeds recursion limit"


def test_default_func() -> None:
    """
    packb() default function
    """
    ref = Custom()

    def default(obj: object) -> object:
        return str(obj)

    assert ormsgpack.packb(ref, default=default) == msgpack.packb(str(ref))


def test_default_raises_exception() -> None:
    """
    packb() default function raises exception
    """

    def default(obj: object) -> object:
        raise NotImplementedError

    with pytest.raises(ormsgpack.MsgpackEncodeError) as exc_info:
        ormsgpack.packb(Custom(), default=default)
    assert str(exc_info.value) == "Type is not msgpack serializable: Custom"


def test_default_returns_invalid_string() -> None:
    """
    packb() default function returns invalid string
    """
    ref = Custom()

    def default(obj: object) -> object:
        return "\ud800"

    with pytest.raises(ormsgpack.MsgpackEncodeError):
        ormsgpack.packb(ref, default=default)


def test_default_lambda() -> None:
    """
    packb() default lambda
    """
    ref = Custom()
    assert ormsgpack.packb(ref, default=lambda x: str(x)) == msgpack.packb(str(ref))


def test_default_callable() -> None:
    """
    packb() default callable
    """
    ref = Custom()

    class Default:
        def __call__(self, obj: object) -> object:
            return str(obj)

    assert ormsgpack.packb(ref, default=Default()) == msgpack.packb(str(ref))


def test_default_recursion() -> None:
    """
    packb() default recursion limit
    """

    class Recursive:
        def __init__(self, cur: int) -> None:
            self.cur = cur

    def default(obj: Recursive) -> Recursive | int:
        if obj.cur > 0:
            obj.cur -= 1
            return obj
        return 0

    assert ormsgpack.packb(
        [Recursive(254), Recursive(254)], default=default
    ) == msgpack.packb([0, 0])


def test_default_recursion_infinite() -> None:
    """
    packb() default infinite recursion
    """
    ref = Custom()

    def default(obj: object) -> object:
        return obj

    with pytest.raises(ormsgpack.MsgpackEncodeError):
        ormsgpack.packb(ref, default=default)
