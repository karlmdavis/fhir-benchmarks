# User Story: Analyze Synthea Output

As a benchmark contributor,
  I would like the logs to report on how many resources Synthea produced,
  broken out by resource type and total storage size,
  so that I have a better idea which operations I might want to add support for next.


## Planning

This user story is included in the
  [Round 1 Release Plan](../plans/0001-round-1.md).


## Details

* This will help to determine which resource's operations
    will most stress servers in terms of data volume.
* I suspect that `Patient` resources are actually not the most common,
    and perhaps there are far more `Encounter` resources,
    but it's really just a guess.