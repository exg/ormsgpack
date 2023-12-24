import dataclasses
from typing import List

import msgpack

import ormsgpack


@dataclasses.dataclass
class Member:
    id: int
    active: bool


@dataclasses.dataclass
class Object:
    id: int
    name: str
    members: List[Member]


objects_as_dataclass = [
    Object(i, str(i) * 3, [Member(j, True) for j in range(0, 10)])
    for i in range(100000, 102000)
]


def default(__obj):
    if dataclasses.is_dataclass(__obj):
        return dataclasses.asdict(__obj)


def test_dataclass_msgpack(benchmark):
    benchmark.group = "dataclass serialization"
    benchmark.extra_info["lib"] = "msgpack"
    output = benchmark(msgpack.packb, objects_as_dataclass, default=default)
    benchmark.extra_info["output_size"] = len(output)


def test_dataclass_ormsgpack(benchmark):
    benchmark.group = "dataclass serialization"
    benchmark.extra_info["lib"] = "ormsgpack"
    output = benchmark(ormsgpack.packb, objects_as_dataclass)
    benchmark.extra_info["output_size"] = len(output)
