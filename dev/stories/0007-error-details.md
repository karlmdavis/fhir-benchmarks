# User Story: Increase Detail in the Application's Errors

As a developer of this application,
  I need errors that occur in the application to provide more detail,
  so that I can track them down and solve them.


## Planning

This user story is included in the
  [Round 1 Release Plan](../plans/0001-round-1.md).


## Details

Right now, the application's output on WSL and in GitHub Actions looks like this:

```
{
  "started": "2020-05-03T02:18:04.587023500Z",
  "completed": null,
  "servers": [
    {
      "server": "HAPI FHIR JPA Server",
      "launch": {
        "started": "2020-05-03T02:18:04.587230300Z",
        "completed": "2020-05-03T02:18:04.614979100Z",
        "outcome": {
          "Errs": [
            "IoError(Os { code: 2, kind: NotFound, message: \"No such file or directory\" })"
          ]
        }
      },
      "operations": null,
      "shutdown": null
    }
  ]
}
```

What caused that `IoError`?
What file was it trying to find?
Where in the application did this occur?

There's no way to tell.


## Research

I have found the following blog posts on error handling in Rust to be very useful:

* [From failure to Fehler](https://boats.gitlab.io/blog/post/failure-to-fehler/)
    * Recommends the use of the [anyhow](https://crates.io/crates/anyhow) library for error handling in applications.
* [Error Handling Survey — 2019-11-13](https://blog.yoshuawuyts.com/error-handling-survey/)
    * Details all of the important error handling libraries out there at this point in time.
* [Error Handling in a Correctness-Critical Rust Project](http://sled.rs/errors)
    * Doesn't apply so much to the problem at hand,
        but is nevertheless quite stuck in my brain.

My takeaways from the above posts are:

1. I should be using Rust's `Error` trait, rather than a giant error enum like I currently am.
2. I should be using something like [anyhow](https://crates.io/crates/anyhow) to provide error context, if nothing else.


## Outstanding Questions

1. Can I get useful error representations from anyhow or whatever into my `FrameworkResults` struct?