---
id: SR-1224
title: "Gap analysis of quire-spec-language PR #602: FR-101 AC-3, AC-7, AC-10, AC-12 to AC-15 and FR-097-AC-5"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@ca788c9bdfee6a0ce072a9a85e249dafae834880; PR #602: FR-101-AC-3, AC-7, AC-10, AC-12, AC-13, AC-14, AC-15, FR-097-AC-5 against qsl-eval/tests/it/finite_simulation.rs and qsl-eval/src/simulation/sample.rs tests; TC-439, TC-453, TC-454, TC-455, TC-474, TC-536 (FR-101 half)"
review_set: subset
---
# Gap analysis of quire-spec-language PR #602

## Summary

Every in-scope AC has at least one test with a correct trace. The oracles
are literal: expected `Outcome` values, frontiers as digests, stats, causes
and categories, and sampler indices. An independent re-implementation of the
sampler reproduces the AC-3 vectors and the new n=2 and n=7 vectors. TC-536's
FR-126-AC-8 half and the FR-162 reduction hooks are out of scope, as the
brief states.

Coverage by AC:
- AC-3: the TC-210 five-way vector and the further vectors, the step-0 preimage bytes and digest, and step advance across no-digest steps. All are literal.
- AC-7: `BoundReached{depth: 2}` with frontier `[<2>]` and category inconclusive. Also N/N+1 tests for the horizon, states and transitions, each tested exhaustive at N+1.
- AC-10: `StepLimit` whether or not successors exist.
- AC-12: both causes, the full `Stopped` value and the category.
- AC-13: exhaustive, horizon, a state put back by `Bounded`, and a stopped state.
- AC-14: all three stop directions, and `FindingMismatch` by clearing a finding.
- AC-15: the defaults, `Exhaustive`, `Bounded` at `Limit::States(2)`, and the setting names.
- FR-097-AC-5: TC-439 covers every variant.

## Verdict

Changes requested: one medium finding and two low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-14's finding-mismatch half is tested in one direction only, by clearing the recorded findings of an existing entry. No test adds an entry past the trace, duplicates an entry, or changes an entry's `state` digest, and each of those replays `Ok` today (SR-1223 FND-001). Add those three edits to `tc_474_replay_refuses_a_trace_whose_findings_differ`, each expecting `FindingMismatch`. | qsl-eval/tests/it/finite_simulation.rs:1919-1947 |
| FND-002 | low | FR-101's Status says `Stopped` takes "the catalog's category for any other catalogued code, and internal failure for a code the catalog does not define". The tests exercise only `resource_exhausted` (the override) and `runtime_invariant`. Neither the fall-through to the catalog (for example `invalid_runtime_input` giving `Refusal`) nor an uncatalogued code giving `InternalFailure` is tested. Add both rows to the cause loop in `tc_474_an_expansion_stop_ends_exploration_stopped`. | qsl-eval/tests/it/finite_simulation.rs:1778-1805; qsl-eval/src/simulation/explore.rs:171-182 |
| FND-003 | low | AC-13 and FR-101 say entries come "in expansion order". Every TC-474 exploration yields at most one entry, so the order is unasserted. Give two states findings (for example `1` and `2` on `branch_graph`) and assert both entries, in breadth-first order. | qsl-eval/tests/it/finite_simulation.rs:1807-1863 |
