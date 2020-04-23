# FHIR Benchmarks

![CI: Rust Basics](https://github.com/karlmdavis/fhir-benchmarks/workflows/CI%3A%20Rust%20Basics/badge.svg)

This project aims to benchmark the various FHIR server implementations that are available, detailing their resource usage and speed. The benchmarks are community-managed in this project, regularly run automatically using the code in this project, and the results are then published to:

<TODO>

## How Are These Benchmarks Architected?

Glad you asked! The benchmarks have three main components:

1. The benchmark orchestrator, which sets everything up, runs each individual set of benchmarks, and then collects the results.
1. The benchmark server apps, which are the FHIR servers under test. These often contain multiple components themselves, e.g. a server, a database, etc.
1. The benchmark server testers, which run the supported benchmarks against their paired benchmark server apps.

The orchestrator has a bunch of subcomponents, such as a source data generator (based on TODO) and a read-only source database for those applications that need it.

For a full benchmark run, the overall process goes like this:

1. Build everything. See TODO for details.
1. Start the orchestrator container. See these for details:
    * TODO
1. Setup all global resources, such as the test data that will be used. See TODO for details.
1. Loop over the supported FHIR servers, listed in TODO:
    1. Setup all server-specific resources. At the end of this, the benchmark server apps should all be ready & waiting.
        * The interface is defined here: TODO.
        * For an example implementation, see: TODO.
    1. Kick off the benchmark server testers, which will run the benchmark suite against the benchmark server apps.
        * The interface is defined here: TODO.
        * For an example implementation, see: TODO.
1. ... ?
1. Profit!

## How Do I Run the Benchmarks Locally?

The benchmarks are all built and runnable as Docker containers. In production, these containers are deployed to the cloud and run there but they can just as easily be run locally.

First, ensure you have installed the following prerequisites:

* TODO

Then, run this command to run all of the benchmarks:

    $ TODO

Or to run just a portion of the benchmarks, you could instead run something like the following:

    $ TODO

## How Do I Add a New FHIR Server to the Benchmarks?

Woohoo! We're excited to see the set of benchmarks expanding!

The process for adding a new benchmark goes something like this:

1. Find one of the existing benchmarks in TODO to copy from.
1. Copy-paste those benchmarks:
    
    ```
    $ cd fhir-benchmarks
    $ cp TODO mynewserver/
    ```

1. Hack, hackity, hack those benchmarks until they successfully run using your new FHIR server rather than the one you copied from.

Overall, the benchmarks themselves aren't really that complicated. The hard parts all have to do with automating the installation and configuration of your FHIR server.
