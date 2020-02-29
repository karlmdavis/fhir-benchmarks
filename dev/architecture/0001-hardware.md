# Architecture Decision: What hardware should the benchmarks be run on?

Why are the benchmarks valid if the orchestrator, server apps, and server testers are all being run on the same shared hardware?

Running everything on the same shared hardware definitely presents some problems:

* The server apps will run slower than they might in a "normal" environment, due to resource contention (CPU, etc.) between them and the benchmarks suite itself.
* The server apps will run slower than they might in a "normal" environment, due to resource contention (CPU, etc.) between their various components, such as the server processes and the database processes.

However, those problems are all shared equally by all FHIR servers being benchmarked. Since only one set of FHIR server apps is running at a time, all of the servers are on a level playing field. It is true that these benchmarks will not produce the theoretical maximum performance possible from each FHIR server, but that's not really a goal here, anyways.

On the positive side, running on the same hardware presents the following advantages:

* The exact same process can be used for running locally as for running in the cloud.
* The benchmark orchestrator can stick with just Docker and glue code, rather than also having to tread into the more complicated waters of having to juggle Terraform provisioning, etc.

There is definitely a tradeoff occurring here, but it seems a reasonable one.