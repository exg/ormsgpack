import pyperf

import ormsgpack
from benchmarks.generator import LIBRARIES, Generator


def main() -> None:
    runner = pyperf.Runner()
    experiments = [e for e in Generator().experiments() if e.name in {"dict"}]
    for experiment in experiments:
        data = ormsgpack.packb(experiment.data)
        group = "deserialization"
        for library in LIBRARIES:
            runner.bench_func(
                f"{library.name} {group}",
                library.unpackb,
                data,
                metadata={
                    "lib": library.name,
                    "group": group,
                },
            )


if __name__ == "__main__":
    main()
