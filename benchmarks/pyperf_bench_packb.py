import pyperf

from benchmarks.generator import LIBRARIES, Generator


def main() -> None:
    runner = pyperf.Runner()
    experiments = Generator().experiments()
    for experiment in experiments:
        group = f"{experiment.name} serialization"
        for library in LIBRARIES:
            runner.bench_func(
                f"{library.name} {group}",
                library.packb,
                experiment.data,
                metadata={
                    "lib": library.name,
                    "group": group,
                },
            )


if __name__ == "__main__":
    main()
