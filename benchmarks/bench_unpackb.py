import pytest

from .generator import LIBRARIES, Generator

EXPERIMENTS = [e for e in Generator().experiments() if e.unpack]


@pytest.mark.parametrize("experiment", EXPERIMENTS, ids=lambda x: x.name)
@pytest.mark.parametrize("library", LIBRARIES, ids=lambda x: x.name)
def test_unpackb(benchmark, experiment, library):
    benchmark.group = experiment.name
    benchmark.extra_info["lib"] = library.name
    benchmark(library.unpackb, experiment.data)
