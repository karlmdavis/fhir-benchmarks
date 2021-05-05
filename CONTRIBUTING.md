# How to Contribute to the FHIR Benchmarks Project

If you'd like to add support for a new FHIR server, additional operations,
  or otherwise help to build out these benchmarks,
  you'd be very welcome!

At the moment, this page is mostly a TODO, but here are some quick notes:

* See the [./dev/architecture/](./dev/architecture) directory
    for a record of architectural decisions that have been explored and settled.
* See the [./dev/stories/](./dev/stories) directory
    for a record of the user stories that were and are being worked on.
    * Why not use GitHub Issues?
      Good question!
      This is mostly just an experiment to see if this is a workable approach;
        I like the idea of having everything available locally and want to try it.
      So far, it seems to be going okay.
* Support for our existing FHIR servers mostly resides in the
    `fhir-bench-orchestrator/src/servers` module.
  Start there if you're looking to add support for a new FHIR server.
* Support for our existing operations mostly resides in the
    `fhir-bench-orchestrator/src/test_framework` module.
  Start there if you'd like to see additional operations get benchmarked.
* Feel free to reach out on <https://chat.fhir.org/> 
    to Karl Davis with any questions.
  (Also please follow up if I don't get back to you right away;
    this a side project and gentle prodding is helpful.)