import pytest

import ormsgpack

from .generator import LIBRARIES, Generator

EXPERIMENTS = [e for e in Generator().experiments() if e.name in {"dict"}]


@pytest.mark.parametrize("library", LIBRARIES, ids=lambda x: x.name)
@pytest.mark.parametrize("experiment", EXPERIMENTS, ids=lambda x: x.name)
def test_unpackb(benchmark, library, experiment):
    data = ormsgpack.packb(experiment.data)
    benchmark.group = "deserialization"
    benchmark.extra_info["lib"] = library.name
    benchmark(library.unpackb, data)
