# Architecture Decision: Use Locust Or Not?

## Background


[Locust](https://locust.io/) is the go-to tool for benchmarking these days.
Due in large part to this popularity, it deserves serious consideration:
  should I craft the benchmarks I need using it?


## Tradeoffs

It presents a lot of advantages:

* Has a web UI to start tests and view their results.
* Uses greenthreads, which greatly increases the amount of load that can be generated per CPU core.
* Supports distributed workers, to scale load out horizontally.
* A solid test scripting approach, using reasonably simple Python scripts.

Unfortunately, it's approach also has some limitations:

* No access to raw results; only statistical summary data.
    * The statistics are decent, but still a long way from an HDR histogram.
* Event time is measured automatically, hardcoded to the HTTP client's request time.
    * It _feels_ like something that can be worked around, but it's not 100% clear, which is a concern.


## Decision

If I'm being honest with myself, I wasn't ever super-interested in using Locust:
  this always felt like something that'd be fun to implement in Rust.
The first limitation above, Locust's lack of raw timing results,
  is also a good reason to avoid using it.
That said, Locust's support for distributed workers gives me real pause.
I very well may need that functionality, and building it out is no trivial thing.

As a compromise position, I've gone looking a couple of times now
  for a Rust crate that's comparable to Locust.
I have not found one.

For the time being, at least, I'm proceeding with writing my benchmark scripts from scratch in Rust.
I'll try to keep an open mind, though, and revisit this approach if it proves unwise.