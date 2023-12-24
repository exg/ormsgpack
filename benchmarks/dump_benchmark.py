#!/usr/bin/env python3

import json
import sys
from collections import defaultdict
from collections.abc import Iterator
from itertools import chain


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
    with open(path) as f:
        data = json.load(f)

    benchmarks = defaultdict(list)
    for bench in data["benchmarks"]:
        extra_info = bench["extra_info"]
        stats = bench["stats"]
        row = [
            extra_info["lib"],
            "{:.2f}".format(stats["median"] * 1000),
            "{:.2f}".format(stats["ops"]),
        ]
        if "output_size" in extra_info:
            row.append(str(int(extra_info["output_size"] / 1024)))
        benchmarks[bench["group"]].append(row)
    return benchmarks


def main() -> None:
    benchmarks = read_benchmarks(sys.argv[1])
    header = [
        "Library",
        "Median (ms)",
        "Operations per second",
        "Output size (KiB)",
    ]
    for name in sorted(benchmarks.keys()):
        benchmark = benchmarks[name]
        print(f"#### {name}\n")
        print(format_table(header[: len(benchmark[0])], benchmark))
        print()


main()
