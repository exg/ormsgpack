import pytest

from .generator import LIBRARIES, Generator

EXPERIMENTS = Generator().experiments()


@pytest.mark.parametrize("library", LIBRARIES, ids=lambda x: x.name)
@pytest.mark.parametrize("experiment", EXPERIMENTS, ids=lambda x: x.name)
def test_packb(benchmark, library, experiment):
    benchmark.group = f"{experiment.name} serialization"
    benchmark.extra_info["lib"] = library.name
    output = benchmark(library.packb, experiment.data)
    benchmark.extra_info["output_size"] = len(output)
