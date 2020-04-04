# Design Decision: How Should Errors Inside the Test Loop Be Reported?

## Background

I've now spent several hours trying to get Serde to serialize `Result` enums to JSON.
It's not gone well: too many remote data structures and enums,
  and Serde's `Serializer` doesn't seem to support arbitrarily nested fields
  (which is kinda' bonkers, but oh well).
Before pouring more time into this problem, it seems appopriate to stop and ask:
  is this even a good idea?

## Design Thoughts & Notes

* There's no need to return the errors back to users as structured output.
* What users need there is solid error logging,
    and a count of how many operations succeeded vs. failed,
    so they know when to go look at those logs.
* One concern I have, though, is:
    Given the multiple Docker instances floating around this thing,
    how do I ensure that all of the logs end up in one spot?
    * Let's assume for the moment I can ensure/force everything
        to log to STDOUT/ERR in those Docker instances.
        * This is probably not true without a decent chunk of effort.
            But: that effort is worth it, and should be made.
    * <https://docs.docker.com/config/containers/logging/>
* Should the orchestrator also just log errors it encounters inside the testing loop?
    * I think so?
    * They can't be serialized, it seems, without a ridiculous amount of pain.
        * Thought: I could add `From` impls for a new serializable error struct.
            Don't know that it buys me anything I want, though.
    * Throwing a panic or aborting early seem like bad options.
    * So: ensure that failures and errors are flagged _somehow_ but leave details to the logs.
* Should I store data on each individual test request's success vs. failure and timestamp?
    * I suspect some back of the napkin math will demonstrate this is a terrible idea.
        * Let's assume only 10K requests per test (which is low),
            100 tests per server, 10 servers, and... 10 bytes per request record.
        * That'd be 100,000,000 bytes of results.
    * That's _possible_ sure, but means that the JSON output file
        ceases to be human-readable and is instead only machine-readable.

## Decision

Focus on serializing output that matters to consumers of the benchmark results.
Implementors can get the debugging and development data they need from logs.