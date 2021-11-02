# Bug: HAPI 'POST /Organization' Failures With Timeouts

Getting a lot of operation failures with the benchmarks logging this:

```
{
  "msg": "Operation 'POST /Organization' failed: 'ServerOperationIterationState { _inner: ServerOperationIterationFailed { completed: ServerOperationIterationCompleted { start: ServerOperationIterationStarting { started: 2021-05-31T18:22:55.066093905Z }, completed: 2021-05-31T18:23:05.066276646Z }, error: Operation timed out: 'future has timed out' } }",
  "level": "WARN",
  "ts": "2021-05-31T18:23:45.09314212000:00"
}
```

These consistently pop up with more concurrency, e.g. when running on `eddings` about a quarter of requests at `concurrent_users: 10` are failing due to this.


## Planning

This user story is included in the
  [Round 1 Release Plan](../plans/0001-round-1.md).