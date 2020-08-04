# Synthetic Data

As a reader/consumer of benchmarks,
  I need the data used by the FHIR servers when they're being genchmarked to be realistic,
  so that I can trust that the benchmarks are representative of real world performance.


## Details

I'm planning to use Synthea, so that covers most of the concerns, I think.

That said, care will still need to be taken to ensure:

1. Each FHIR server gets exactly the same data.
2. Any/all randomness is statically seeded so that conditions are reproducible
   (at least until library versions change).
    * Looks like Synthea has a couple of unresolved issues with this:
      <https://github.com/synthetichealth/synthea/issues/657>.
      I suppose I'll want to store the generated data somewhere, then.
      At least temporarily.
3. The same queries should be made of all FHIR servers;
   if resource/record selection is randomized, the same series must be used for all servers.
4. As much data as is reasonably possible is generated for full benchmark runs,
   to ensure that FHIR servers and their DBs are appropriately stressed.
5. Data generation configuration should be hashed and used to cache the output somewhere durable.
   Realistic amounts of data will take **hours** to generate,
     so this is rather critical for reasonable development and testing;
     re-generating the data every run is **not** feasible.
    * That said, the caching could be deferred to a later story.