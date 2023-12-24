import pyperf

from benchmarks.generator import LIBRARIES, Generator


def main() -> None:
    runner = pyperf.Runner()
    experiments = [e for e in Generator().experiments() if e.unpack]
    for experiment in experiments:
        for library in LIBRARIES:
            runner.bench_func(
                f"{library.name} {experiment.name}",
                library.unpackb,
                experiment.data,
                metadata={
                    "lib": library.name,
                    "group": experiment.name,
                },
            )


if __name__ == "__main__":
    main()
