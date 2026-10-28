---
id: SR-1709002
title: "Gap analysis of quire-spec-language PR #1709: bare-clock collection counts and Clock parameters and results (580p)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@68e00d6d6; PR #1709 against FR-230 Operation parameters, Results, Initial element count, FR-230-AC-13, AC-16, AC-46..48 (TC-685 steps 29-32), FR-120-AC-26"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
---
# Gap analysis of quire-spec-language PR #1709

## Summary

Ticket: QSL-580, slice 580p. Plan completion: not assessed.

**Covered.**
- `a_collection_of_bare_clocks_ranges_over_its_counts`: domains, initial
  states, a granted unbounded collection requiring a bound, slot carry by
  position, and a fresh slot.
- `a_bare_clock_collection_ranges_its_initial_count`:
  - AC-46's `s: Sequence<Clock>[1, 3]` and `b: Bag<Clock>[0, 2]`;
  - AC-47's `Set<Clock>`/`OrderedSet<Clock>[0, 5]` giving 0 and 1, and
    `Set<Clock>[2, 5]` giving none;
  - AC-48's ungranted unbounded collection ascending to `MaxClocks`, with
    the bound check admitting it;
  - distinct state keys per count.
- `a_clock_parameter_ranges_over_the_places_it_may_write`: S3 ranges,
  tags, `clock-write`, an alias parameter, and `c = c` refusing.
- `a_clock_parameter_has_one_instance_per_place`: AC-13's instances, the
  `clock_place` argument, and guard instances.

**Deferred in the PR body.** AC-46's `st: Sequence<Stopwatch>` and `qo`,
AC-47's `so`/`Set<Stopwatch>`, and AC-47's `Map` (QSL-1169). AC-39..45
(presence bits and place ids) are 580q.

## Verdict

**Gaps remain** for the two defects SR-1709001 found and for the
analysis-level settlements.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Untested paths: a receiver-scoped frame on a reset `Clock` parameter (SR-1709001 FND-001); a data parameter declared after a `Clock` parameter, read by a hyper step label or a footprint (FND-002); two `Clock` parameters on one operation (the cartesian instances); and a `Clock` result used in a postcondition or a call (only S3 checks `bump(c: Clock): Clock`). | qsl-eval/tests/it/timed_subject.rs:4037-4160; qsl-semantics/tests/it/clock_places.rs |
| FND-002 | low | AC-47 and AC-48 state their outcome as an analysis settlement: `Set<Clock>[2, 5]` as `Undecided(NoInitialState)` (FR-239), and the unbounded collection as an `incomplete`/`limit-reached` run on `max_clocks`. The tests stop at the initial-state list and the expansion's `TimedStop::MaxClocks`. Add one EN-6 (or explore) assertion for each settlement. | FR-230-AC-47, AC-48; qsl-eval/tests/it/timed_subject.rs |
| FND-003 | low | Record the AC rows deferred to 580q and QSL-1169 on QSL-580 (AC-46 `st`/`qo`, AC-47 `so`/`Set<Stopwatch>`/`Map`, AC-39..45), so they keep owners beyond the PR body. | QSL-580; FR-230-AC-39..47 |

## Dispositions

Round 1, fix commit cc106f1bf, head 81fa6e048. The gates are those in SR-1709001's dispositions.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Each path now has a test, and each passes. The receiver frame: `a_receiver_scoped_clock_parameter_binds_the_receivers_place`. A data parameter after a Clock parameter: `a_data_argument_after_a_clock_parameter_keeps_its_index`. Two Clock parameters: `two_clock_parameters_bind_every_pair_of_places`, 8 distinct instances, each resetting `c`'s place only. A Clock result: `an_operation_with_a_clock_result_steps`. |
| FND-002 | fixed | `en6_settles_the_bare_clock_counts` asserts both settlements at analysis level. `Set<Clock>[2, 5]` gives `Undecided(NoInitialState)`. An unbounded `Sequence<Clock>` under `max_clocks` 2 gives `Stopped(LimitReached(MaxClocks(2)))`. |
| FND-003 | fixed | Recorded on QSL-580 (comment 5bba1fe4, "Deferred FR-230 rows ... SR-1709002 GA FND-003"), with the 580q rows and the blocking clock_place selector ruling. |
