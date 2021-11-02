# Bug: HAPI Failures After Launch

Intermittently, HAPI is experiencing test case failures early on in a run.
These failures look like this in the log:

```
{
  "msg": "request failed",
  "level": "WARN",
  "ts": "2020-05-10T22:03:24.164416817-04:00",
  "err": "reqwest::Error { kind: Request, url: \"http://localhost:8080/hapi-fhir-jpaserver/fhir/metadata\", source: hyper::Error(Io, Os { code: 104, kind: ConnectionReset, message: \"Connection reset by peer\" }) }",
  "url": "http://localhost:8080/hapi-fhir-jpaserver/fhir/metadata"
}
{
  "msg": "request failed",
  "level": "WARN",
  "ts": "2020-05-10T22:03:24.171806321-04:00",
  "err": "reqwest::Error { kind: Request, url: \"http://localhost:8080/hapi-fhir-jpaserver/fhir/metadata\", source: hyper::Error(IncompleteMessage) }",
  "url": "http://localhost:8080/hapi-fhir-jpaserver/fhir/metadata"
}
{
  "msg": "request failed",
  "level": "WARN",
  "ts": "2020-05-10T22:03:24.181346040-04:00",
  "err": "reqwest::Error { kind: Request, url: \"http://localhost:8080/hapi-fhir-jpaserver/fhir/metadata\", source: hyper::Error(IncompleteMessage) }",
  "url": "http://localhost:8080/hapi-fhir-jpaserver/fhir/metadata"
}
// ... trimmed out many additional occurrences of previous error, for brevity
{
  "started": "2020-05-11T02:03:21.230350766Z",
  "completed": "2020-05-11T02:04:04.849673309Z",
  "servers": [
    {
      "server": "HAPI FHIR JPA Server",
      "launch": {
        "started": "2020-05-11T02:03:21.230377730Z",
        "completed": "2020-05-11T02:03:24.153744551Z",
        "outcome": {
          "Ok": []
        }
      },
      "operations": [
        {
          "operation": "metadata",
          "started": "2020-05-11T02:03:24.153756589Z",
          "iterations": 1000,
          "completed": "2020-05-11T02:03:59.957950149Z",
          "failures": 80,
          "metrics": null
        }
      ],
      "shutdown": {
        "started": "2020-05-11T02:03:59.957961401Z",
        "completed": "2020-05-11T02:04:04.849668513Z",
        "outcome": {
          "Ok": []
        }
      }
    }
  ]
}
```

These errors _seem_ to clear up once HAPI has been running for a little bit.


## Planning

This user story is included in the
  [Round 1 Release Plan](../plans/0001-round-1.md).


## Status

This issue appears to be resolved by <https://github.com/karlmdavis/fhir-benchmarks/pull/8>.