# FHIR Server Compliance

There's quite a bit of overlap between testing the performance of a FHIR server
  and verifying its compliance:
  it's likely that a benchmark suite such as this one will
  expose a FHIR server's non-compliance with the specification,
  as it's trying to test every server implementation in exactly the same way.
Those compliance issues will often show up as benchmark test failures.

Sometimes, though, for the purposes of benchmarking,
  we decide to work around compliance issues that are found.
Whenever that's done, for transparency, we try to do the following:

1. File an issue in the server's ticket system with the details of the problem.
2. Note that issue here, for transparency and future reference.

## Compliance Issues Found To Date

### [Firely Spark](https://github.com/FirelyTeam/spark)

#### Expunging Resources: Custom Endpoint

Tracking issue: TODO

The FHIR specification provides a XX endpoint,
  which allows server administrator's to wipe a server clean of all resources.
Spark, however, instead provides a custom endpoint for this operation:
  [MaintenanceApiController.cs](https://github.com/FirelyTeam/spark/blob/master/src/Spark/Controllers/MaintenanceApiController.cs).

It's also worth noting that this operation does not seem to be usable
  when Spark is run via its default Docker build.
A tracking issue for *this* problem has been filed here:
  [Issue #295: Ability to clear/wipe the server when running via Docker Compose](https://github.com/FirelyTeam/spark/issues/295).

#### Unable to POST Resources with an ID

Tracking issue: TODO

The FHIR specification states the following,
  in <http://hl7.org/fhir/http.html#create>:

> The resource does not need to have an id element
>   (this is one of the few cases where a resource exists without an id element).
> If an id is provided, the server SHALL ignore it.

However, when trying to post a resource with an ID,
  Spark resturns an HTTP `400` error with the following body:

```
{
  "resourceType": "OperationOutcome",
  "issue": [{
    "severity": "error",
    "diagnostics": "The request should not contain an id"
  }]
}
```