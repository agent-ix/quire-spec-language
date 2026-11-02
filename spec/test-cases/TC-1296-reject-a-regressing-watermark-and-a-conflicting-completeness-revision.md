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

Verify FR-502-AC-1 to AC-6 and FR-052-AC-13: the spine ledger refuses a
contradicting progress assertion, keeps the retained progress and earlier
results unchanged, and keeps foreign bindings apart.

Catches a ledger that rolls back to a lower watermark, absorbs a lagging
completeness revision, lets a completion close a scope, settles an obligation
from another binding's progress, or advances an event-position clock during
silence.

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
| 8 | event-position clause: accept W 100 | `Pending` stays `Pending`; the same W settles fixed-sample and finite-window clauses; W 60 after W 100 refused on all three |
| 9 | W 100 then W 60 with no ledger | both evaluated from their own traces; with a ledger the second is refused |
| 10 | row 1 through `run_clause` with the ledger | stage `evaluate`, category `refusal`, same record |

## Expected Results

- Each contradiction carries its dimension, clock, retained and asserted
  values as fields, and leaves the ledger and every earlier result byte for
  byte unchanged.
