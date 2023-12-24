import msgpack
import numpy
import pytest

import ormsgpack

RNG = numpy.random.default_rng()

DATA = (
    RNG.integers(2**31, size=(100000, 100), dtype=numpy.int32),
    RNG.random(size=(50000, 100)),
    RNG.choice((True, False), size=(100000, 200)),
)


def default(__obj):
    if isinstance(__obj, numpy.ndarray):
        return __obj.tolist()


@pytest.mark.parametrize("data", DATA)
def test_numpy_msgpack(benchmark, data):
    benchmark.group = f"numpy {data.dtype} serialization"
    benchmark.extra_info["lib"] = "msgpack"
    output = benchmark(msgpack.packb, data, default=default)
    benchmark.extra_info["output_size"] = len(output)


@pytest.mark.parametrize("data", DATA)
def test_numpy_ormsgpack(benchmark, data):
    benchmark.group = f"numpy {data.dtype} serialization"
    benchmark.extra_info["lib"] = "ormsgpack"
    output = benchmark(ormsgpack.packb, data, option=ormsgpack.OPT_SERIALIZE_NUMPY)
    benchmark.extra_info["output_size"] = len(output)
