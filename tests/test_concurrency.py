# SPDX-License-Identifier: (Apache-2.0 OR MIT)

import concurrent.futures

import pytest

InterpreterPoolExecutor = (
    concurrent.futures.InterpreterPoolExecutor(max_workers=4)
    if hasattr(concurrent.futures, "InterpreterPoolExecutor")
    else None
)


@pytest.mark.parametrize(
    "executor",
    (
        pytest.param(
            concurrent.futures.ThreadPoolExecutor(max_workers=4),
            id="threads",
        ),
        pytest.param(
            InterpreterPoolExecutor,
            id="interpreters",
            marks=pytest.mark.skipif(
                InterpreterPoolExecutor is None,
                reason="InterpreterPoolExecutor not available",
            ),
        ),
    ),
)
def test_concurrency(executor: concurrent.futures.Executor) -> None:
    def run(obj: object) -> object:
        import ormsgpack

        return ormsgpack.unpackb(ormsgpack.packb(obj))

    obj = {str(i): i for i in range(1024)}
    with executor:
        futures = [executor.submit(run, obj) for _ in range(256)]
        for future in concurrent.futures.as_completed(futures):
            assert future.result() == obj


def test_dict_grows_during_iteration() -> None:
    import ormsgpack

    obj = {"0": object(), "1": 1}

    def default(_: object) -> object:
        obj["2"] = 2
        return 0

    packed = ormsgpack.packb(obj, default=default)
    unpacked = ormsgpack.unpackb(packed)
    assert unpacked == {"0": 0, "1": 1}


def test_dict_shrinks_during_iteration() -> None:
    import ormsgpack

    obj = {"0": object(), "1": 1}

    def default(_: object) -> object:
        obj.pop("1")
        return 0

    with pytest.raises(ormsgpack.MsgpackEncodeError):
        ormsgpack.packb(obj, default=default)


def test_list_grows_during_iteration() -> None:
    import ormsgpack

    obj = [object(), 1]

    def default(_: object) -> object:
        obj.append(2)
        return 0

    packed = ormsgpack.packb(obj, default=default)
    unpacked = ormsgpack.unpackb(packed)
    assert unpacked == [0, 1]


def test_list_shrinks_during_iteration() -> None:
    import ormsgpack

    obj = [object(), 1]

    def default(_: object) -> object:
        obj.pop()
        return 0

    with pytest.raises(ormsgpack.MsgpackEncodeError):
        ormsgpack.packb(obj, default=default)
