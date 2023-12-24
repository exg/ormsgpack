import pytest

from .generator import LIBRARIES, Generator

EXPERIMENTS = [e for e in Generator().experiments() if not e.unpack]


@pytest.mark.parametrize("library", LIBRARIES, ids=lambda x: x.name)
@pytest.mark.parametrize("experiment", EXPERIMENTS, ids=lambda x: x.name)
def test_packb(benchmark, experiment, library):
    benchmark.group = experiment.name
    benchmark.extra_info["lib"] = library.name
    output = benchmark(library.packb, experiment.data)
    benchmark.extra_info["output_size"] = len(output)
