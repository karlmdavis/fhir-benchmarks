# User Story: Switch to [Tracing](https://lib.rs/crates/tracing) for Logs

As a user of the benchmark application,
  I would like the logs to provide more information on causality,
  so that I'm better able to diagnose issues when they're encountered.


# Details

* I'll be honest: I don't have a real good use case or burning need for this right now
    but, rather, I've been watching Tracing for a while now and I'm intrigued by it.
  I mostly just want to try it out and see if it works well.
* In addition, I think it's probably time to move away from NDJSON log output,
    as it's mostly just making the log output less useful right now.
    * And Tracing also supports NDJSON output, if it turns out to be needed in the future.