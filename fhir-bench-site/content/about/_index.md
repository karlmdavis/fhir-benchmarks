+++
title = "About"
slug = "about"
date = 2021-05-01T09:19:42+00:00
draft = false
template = "section.html"
+++

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

The benchmark suite is available for review and contribution on GitHub:
  [karlmdavis/fhir-benchmarks](https://github.com/karlmdavis/fhir-benchmarks).

The first official and permanent set of benchmarking data will be published,
  when ready, as "Round 1".
Until then, you can view the work-in-progress transient
  [Benchmarks: Round 0](/benchmarks/round-0) results.