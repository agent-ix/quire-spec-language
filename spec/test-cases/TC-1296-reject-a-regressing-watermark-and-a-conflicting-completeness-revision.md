---
id: TC-1296
title: "Reject a regressing watermark and a conflicting completeness revision on the spine"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-502
    type: verifies
---
# TC-1296: Reject a regressing watermark and a conflicting completeness revision on the spine

## Description

Verify FR-502-AC-1 to AC-12 and FR-052-AC-13: the spine ledger refuses a
contradicting progress assertion, keeps the retained progress and earlier
results unchanged, and keeps foreign bindings apart.

Catches a ledger that rolls back to a lower watermark, absorbs a lagging
completeness revision, lets a completion close a scope, settles an obligation
from another binding's progress.

## Test Procedure

Binding B is a finite-window clause `eventually[0,5] holds(p)` with `p` false at
stamp 0, clock `ms`. Each step passes B's ledger forward and records the ledger
and the earlier result bytes.

| # | Step | Expected |
| --- | --- | --- |
| 1 | assert W 100, then W 60 | second refused `invalid_runtime_input`/`invalid-value`, `contradicted_dimension` `Watermark`, clock `ms`, retained 100, asserted 60; ledger W 100; earlier result bytes equal |
| 2 | assert W 100 again, then W 130 | both accepted |
| 3 | retain C(100); assert C(60); assert `Inc`; add a position stamped 80 the assertion did not cover | each refused `Completeness`; ledger unchanged |
| 4 | assert C(100), then C(140) | accepted |
| 5 | retain scope `Closed`; assert `Open` | refused `Closure` |
| 6 | scope `Closed` with `Inc`; scope `Open` with C(100) | completeness stays `Inc`; scope stays `Open` |
| 7 | assert W 100 under a second package, a second clause, a second clock name, a second profile, each alone | B's `Pending` stays `Pending`, B's entry unchanged, four new entries |
| 8 | a `ledger` with a `Finite` trace without progress, a `Lasso`, a `Model` and an infinite-trace clause | each refused at stage `admit`, `invalid_runtime_input`/`invalid-value`; ledger unchanged |
| 8b | the same ledger with a `Timestamped` finite-window trace, a fixed-sample `Progressed` trace and an event-position `Progressed` trace; then fixed-sample W 3 then W 1; event-position W 3 then W 9 | each accepted; `Watermark` contradiction; accepted with the `Pending` outcome unchanged |
| 12 | `progress.covered_positions` 3, a request covering 4 positions; then a later trace omitting two covered positions and adding one beyond `through` | `Incomplete(LimitReached{limit: node count, value: 3, setting: progress.covered_positions})`, ledger unchanged; accepted, omitted positions still listed covered |
| 9 | W 100 then W 60 with no ledger | both evaluated from their own traces; with a ledger the second is refused; the returned ledger passed to the next request gives the outcomes of the same sequence replayed from the same initial ledger |
| 10 | row 1 through `run_clause` with the ledger | stage `evaluate`, category `refusal`, same record |
| 11 | retain C(100) over positions 0 to 3; add a position stamped 80 not among them; then one stamped 120 | refused `Completeness`; accepted; the returned ledger lists the covered positions |
| 13 | `progress.covered_positions` 3; B holds two covered positions; a request under a second binding covering two positions, then one covering one | `Incomplete(LimitReached{limit: node count, value: 3, setting: progress.covered_positions})`, ledger unchanged; accepted |
| 14 | retain C(100) over identity `p1` stamped 80; a trace with another identity stamped 80; then `p1` stamped 80 | refused `Completeness`; accepted |
| 15 | any refused request of rows 1, 3, 5, 8 and 12, with and without a supplied ledger | the report returns the supplied ledger, equal to it; none when none was supplied |
| 16 | two clauses `Fast` and `Slow` with identical bodies and different names (one node, occurrences 0 and 1; same package, profile and clock binding name, so only the occurrence differs): W 100 under `Fast`; W 60 under `Slow`; W 60 under `Fast`; rename `Slow` | W 60 under `Slow` accepted with its own entry; W 60 under `Fast` refused `Watermark`, retained 100; `Slow`'s entry holds W 60; the rename changes neither entry's key |

## Expected Results

- Each contradiction carries its dimension, clock, retained and asserted
  values as fields, and leaves the ledger and every earlier result byte for
  byte unchanged.
