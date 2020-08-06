# Generating Synthetic Data

[Synthea](https://github.com/synthetichealth/synthea),
  an open-source, synthetic patient generator that models the medical history of synthetic patients,
  is used to generate the synthetic FHIR resources used in the benchmarks.


## Running Synthea

Synthea is built and run via Docker:

```
$ docker build --file Dockerfile.synthea --build-arg UID=$(id -u) --build-arg GID=$(id -g) -t fhir-benchmarks/synthea .
$ docker run -it --mount source="$(pwd)/target/",target="/synthea/target/",type=bind fhir-benchmarks/synthea -h
```


## Synthea Options

Synthea's builtin help/usage information:

```
$ docker run -it --mount source="$(pwd)/target/",target="/synthea/target/",type=bind fhir-benchmarks/synthea -h
Usage: run_synthea [options] [state [city]]
Options: [-s seed] [-cs clinicianSeed] [-p populationSize]
         [-g gender] [-a minAge-maxAge]
         [-o overflowPopulation]
         [-m moduleFileWildcardList]
         [-c localConfigFilePath]
         [-d localModulesDirPath]
         [--config* value]
          * any setting from src/main/resources/synthea.properties
Examples:
run_synthea Massachusetts
run_synthea Alaska Juneau
run_synthea -s 12345
run_synthea -p 1000)
run_synthea -s 987 Washington Seattle
run_synthea -s 21 -p 100 Utah "Salt Lake City"
run_synthea -g M -a 60-65
run_synthea -p 10 --exporter.fhir.export true
run_synthea -m moduleFilename:anotherModule:module*
run_synthea --exporter.baseDirectory "./output_tx/" Texas
```

Additional references:

* <https://github.com/synthetichealth/synthea/wiki/Basic-Setup-and-Running>
* <https://github.com/synthetichealth/synthea/wiki/Common-Configuration>


## Synthea Data Generation Statistics

All timings here are from my local development system,
  and are only useful in comparing relative to one another.

* `docker run -it --mount source="$(pwd)/target/",target="/synthea/target/",type=bind fhir-benchmarks/synthea -s 42 -cs 42 -p 1`
    * Runtime: 00:18 seconds
    * Data Size: 13 MB
* `docker run -it --mount source="$(pwd)/target/",target="/synthea/target/",type=bind fhir-benchmarks/synthea -s 42 -cs 42 -p 100`
    * Runtime: 00:29 seconds
    * Data Size: 108 MB
* `docker run -it --mount source="$(pwd)/target/",target="/synthea/target/",type=bind fhir-benchmarks/synthea -s 42 -cs 42 -p 1000`
    * Runtime: 01:54 seconds
    * Data Size: 1.5 GB
    * Compresses via `tar -czf` to 87 MB in 00:18 seconds, which uncompresses in 0:06 seconds.
* `docker run -it --mount source="$(pwd)/target/",target="/synthea/target/",type=bind fhir-benchmarks/synthea -s 42 -cs 42 -p 10000`
    * Runtime: 11:30 seconds
    * Data Size: 14 GB
* Extrapolating from there, it follows that:
    * 1M people would take about 19 hours to generate and 1TB of disk space to store.
    * 100M people would take about 80 days to generate and 100TB of disk space to store.