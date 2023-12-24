#!/usr/bin/env python3

import sys
from collections import defaultdict
from collections.abc import Iterator
from itertools import chain

import pyperf


def justify(row: list[str], widths: list[int]) -> Iterator[str]:
    for i, w in enumerate(widths):
        if i == 0:
            yield row[i].ljust(w)
        else:
            yield row[i].rjust(w)


def format_table(header: list[str], table: list[list[str]]) -> str:
    widths = [
        max(len(row[i]) for row in chain([header], table)) for i in range(len(table[0]))
    ]
    return "\n".join(
        [
            "| {} |".format(" | ".join(justify(header, widths))),
            "|{}|".format(
                "|".join(
                    (":{}" if i == 0 else "{}:").format("-" * (w + 1))
                    for i, w in enumerate(widths)
                )
            ),
        ]
        + ["| {} |".format(" | ".join(justify(row, widths))) for row in table]
    )


def read_benchmarks(path: str) -> defaultdict[str, list[list[str]]]:
    benchmark_suite = pyperf.BenchmarkSuite.load(path)
    benchmarks = defaultdict(list)
    for benchmark in benchmark_suite.get_benchmarks():
        metadata = benchmark.get_metadata()
        row = [
            metadata["lib"],
            "{:.2f}".format(benchmark.median() * 1000000),
        ]
        benchmarks[metadata["group"]].append(row)
    return benchmarks


def main() -> None:
    benchmarks = read_benchmarks(sys.argv[1])
    header = [
        "Library",
        "Median (ms)",
    ]
    for name in sorted(benchmarks.keys()):
        benchmark = benchmarks[name]
        print(f"#### {name}\n")
        print(format_table(header[: len(benchmark[0])], benchmark))
        print()


main()
