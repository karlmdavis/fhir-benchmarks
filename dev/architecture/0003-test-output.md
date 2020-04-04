# Architecture Decision: What Should the Test Output Look Like?

I suspect that a very common workflow for this project later on will be:

1. FHIR server implementor decides to add or improve benchmark for their FHIR server.
1. Implemetor downloads this project,
     points it at a local copy of their server,
     and then starts working to implement and/or improve the benchmarks for their server.
1. After each change to their server and/or the benchmark,
     implementor will need to find, view, and understand the results.
1. Implementor will use results to guide next steps,
     often re-running the benchmarks and going through this loop for a while.

I'd been thinking it'd be possible for implementors to just review a single
  JSON output file to inspect the results, and iterate on their servers' performance.
That's... likely not true, though?
It's entirely likely that some servers will support dozens of benchmarks.
It's unlikely that a single JSON file for that much data will be human-readable.

What if the benchmark tooling makes it simple for implementors to run only one test at a time, though?
Even still, I think it's fair to say that they'll eventually need a way to visualize the complete results.
In particular, they'll likely often want to compare their server to another one.

I'm going to need to spend time on the visual design of the results.
I mean... no shit, right?
What I'm realizing now, though, is that getting useful output and comparisons
  will need to be an **early** concern for the project -- won't be able to defer it for long.

TODO: Not sure I have a concrete answer to how to represent HDR Histogram data in output files.