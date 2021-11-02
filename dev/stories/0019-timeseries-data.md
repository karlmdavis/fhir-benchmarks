# User Story: Timeseries Data: Latency, Operation Count, Request Size

As a benchmark contributor and/or FHIR server implementer,
  I would like the benchmark application to output its request data
  to a timeseries database and analysis suite,
  such as [InfluxDB](https://www.influxdata.com/products/influxdb/) 
  plus [Grafana](https://grafana.com/),
  so that I can analyze the performance of my FHIR server
  over time during each benchmark operation.


## Planning

This user story is included in the
  [Round 1 Release Plan](../plans/0001-round-1.md).


## Details

* This is something Wind Tunnel provides.
* I've heard from FHIR server implementers that this is an important feature for them.
* For some FHIR servers, they might have native support for InfluxDB,
    and it'd be interesting to turn that on during benchmarking.