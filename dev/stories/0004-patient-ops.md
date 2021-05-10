# User Story: Support `Patient` Resource Operations

As a benchmark user,
  I'd like to see `Patient` resource operations included in the benchmarks,
  so that I can gain insight into various FHIR server's performance
  with this important resource.

## Details

I was originally thinking this would be a good resource to benchmark next
  because Synthea produces so many of them.
However, that's silly,
  because there are other resource types that Synthea produces much more of.
Nevertheless, all of _those_ resource depend on `Patient`,
  so I might as well do it first, anyways.