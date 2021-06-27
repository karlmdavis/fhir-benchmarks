# User Story: Improve Debugging of Operation Failures 

At higher concurrency levels right now,
  I'm seeing a lot of operation timeout failures for both HAPI and Spark.

However, the only way to debug those failures
  is to stare at the log output during a benchmark run,
  wait for failures to get logged,
  and then quickly try to run `docker logs ...` for the container.
As debugging experiences go,
  this is bad.

Instead, I think we need to start collecting logs for the FHIR servers,
  writing those logs to disk,
  and then referencing all of the log file locations in the JSON output.

## Details

* For larger benchmark runs, these log files are liable to eat a lot of disk space,
    perhaps even enough to exhaust the benchmark system's free space.
  I may want to compress them upfront, to help mitigate this.
* Haven't spent some time trying out the goofy manual debugging procedure above for HAPI timeouts,
    it's not clear to me that these logs
    will actually provide enough information to diagnose the problems.
  Nevertheless, this seems like a necessary first step towards improving debugging in general.