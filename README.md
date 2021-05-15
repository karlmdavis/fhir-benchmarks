# FHIR Benchmarks

[![CI: Rust Basics](https://github.com/karlmdavis/fhir-benchmarks/workflows/CI%3A%20Rust%20Basics/badge.svg)](./workflows/rust_basics.yml)

The FHIR landscape is expanding and maturing rapidly.
Users now have many great FHIR server implementation options to choose between.
This project aims to help with those choices by
  reporting on the performance of FHIR server implementations,
  via a set of repeatable and isolated benchmarks that have captured:

* The throughput of various operations,
    i.e. responses per second.
* The latency of various operations,
    i.e. how many milliseconds per response,
    at various percentiles,
    e.g. "100% of responses take less than 42 milliseconds",
    "99.9% of responses take less than 30 milliseconds",
    "50% of responses take less than 10 milliseconds (i.e. the median latency)",
    etc.
* How those operations performed under different levels of load/concurrency.
* How those operations performed with sample data sets of different sizes.
* The success-to-failure ratio of those operations.

Each round of benchmarking will be run periodically and published publicly
  to [fhir-benchmarks.com](https://fhir-benchmarks.com/),
  allowing users to see the relative performance of various FHIR servers
  and FHIR server implementers to gauge (and improve!) their server's performance.


## So What's Next?

Some of the most immediate next steps include:

* Adding support for benchmarking additional operations, e.g. `Patient` writes, reads, and searches.
* Adding support for additional FHIR servers, e.g. [IBM FHIR Server](https://ibm.github.io/FHIR/).
* Adding infrastructure and automation to run the benchmarks periodically and automatically.


## How Do I Run the Benchmarks Myself?

The benchmark orchestrator and operations are all written in [Rust](https://www.rust-lang.org/),
  which run against locally hosted FHIR servers run as Docker containers,
  using sample data generated via [Synthea](https://synthetichealth.github.io/synthea/).

First, ensure you have installed the following prerequisites:

* [Rust](https://www.rust-lang.org/) >= v1.51.0, as installed via [rustup](https://www.rust-lang.org/learn/get-started).
* [Docker](https://www.docker.com/) >= v20.10.6
* [Docker Compose](https://docs.docker.com/compose/) >= v1.27.4
* (Ubuntu only) install `libssl-dev`

Then, run these commands to clone, build, and run the benchmark suite's tests:

```shell
$ git clone https://github.com/karlmdavis/fhir-benchmarks.git
$ cd fhir-benchmarks
$ # Build in debug mode and run tests:
$ cargo test
$ # Build in release mode and run benchmarks:
$ cargo run --release \
  | tee ./results/results-release-$(date --iso-8601=seconds).json
```

Both the tests and the benchmarks themselves will automatically build the Docker containers for the FHIR servers and Synthea, as needed.

Or to run the benchmarks with a customized config (see the defaults in [./fhir-bench-orchestrator/src/config.rs](./fhir-bench-orchestrator/src/config.rs)):

```shell
$ FHIR_BENCH_ITERATIONS=1000 \
  FHIR_BENCH_CONCURRENCY_LEVELS=1,2,8 \
  FHIR_BENCH_POPULATION_SIZE=1000 \
    cargo run --release \
    | tee ./results/benchmark-$(date --iso-8601=seconds).json
```


## How Can I Contribute to this Effort?

If you'd like to add support for a new FHIR server, additional operations,
  or otherwise help to build out these benchmarks,
  you'd be very welcome!
Please see [CONTRIBUTING.md](./CONTRIBUTING.md) for further details.
