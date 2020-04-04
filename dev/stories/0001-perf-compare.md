# User Story: Compare Performance of FHIR Servers

As a FHIR server implementor, I need an apples-to-apples comparison of performance between FHIR servers,
  so that I can understand how the performance of my FHIR server differs from others -- and in particular,
  where it stands in a ranked comparison.

## Details

* The "apples-to-apples" qualifier is key. Comparisons are only possible when:
    * Results are from reliable benchmarks.
        * If the rankings of measured latencies, throughputs, etc. move around a lot between runs,
          implementors will be less inclined to trust them.
    * The same operation with the same input data and equivalent output results are measured.
        * If one FHIR server produces meaningfully different results from another for the same operation,
          any performance results from it are invalid.
        * For example, if a FHIR server only returns half the fields,
          it's easier for it to achieve higher performance.
        * The results will require at least some validation, and perhaps quite a lot.
          Likely want to:
            1. Fully validate a single iteration of each operation.
            2. Verify the response code and (fuzzy) response size of each iteration.
* The structure of output results will matter a lot.
    * For example, if all we return is the mean average latency,
      many important performance differences may be hidden.
    * At a minimum, we should measure and return:
        * Success vs. failure: counts of which operations/iterations succeeded.
        * Latency: mean, p100, p999, p99, p90, p50.
        * Throughput: overall operations/ierations per second.
    * In addition, the following would also be very valuable:
        * HDR histogram of latency: <https://github.com/HdrHistogram/HdrHistogram_rust>.
        * CPU usage of each Docker container: mean, perhaps others.
        * Memory usage of each Docker container: mean, perhaps others.
    * We may **not** want to return:
        * Raw time series values, whether latency, throughput, CPU, or memory.
            * The memory usage required to store these is cost-prohibitive and might interfere with the benchmarks.
            * We _could_ consider writing it out to disk after each test, though.
* Visualization is likely not a secondary or down-the-road concern.
    * Look at all of the above items:
      I don't see any way to present all of them in a single JSON file (or whatever format) --
      at least not any way that would be meaningfully human-readable.
    * An at-least-basic GUI for reporting results will be a required component for any beta release.
      It may also end up being something that _I_ need before then, just to build this.
* Documentation is also an important concern:
    * If implementors and end users do not understand what is being measured and (roughly) how,
      they will not trust the results.