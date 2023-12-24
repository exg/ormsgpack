from collections.abc import Callable
from typing import Any

import pytest

import ormsgpack

from .generator import Generator, datasets

GENERATOR = Generator()
DATASETS = datasets(GENERATOR)


@pytest.mark.parametrize("dataset", DATASETS.values(), ids=DATASETS.keys())
def test_packb(benchmark: Callable[..., Any], dataset: object) -> None:
    benchmark(
        lambda: ormsgpack.packb(
            dataset,
            option=ormsgpack.OPT_SERIALIZE_NUMPY | ormsgpack.OPT_SERIALIZE_PYDANTIC,
        )
    )


@pytest.mark.parametrize("dataset", [DATASETS["dict"]])
def test_unpackb(benchmark: Callable[..., Any], dataset: object) -> None:
    data = ormsgpack.packb(dataset)
    benchmark(ormsgpack.unpackb, data)
