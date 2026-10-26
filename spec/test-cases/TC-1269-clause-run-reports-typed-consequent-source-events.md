---
id: TC-1269
title: "The clause run reports typed consequent source events"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-109
    type: verifies
---
# TC-1269: The clause run reports typed consequent source events

## Description

Verify `qsl_replay::spine::run_clause`'s `consequent_source_events` over
nested Boolean clauses, quantified and called-function clauses, runs without
a truth, and temporal runs.

Scope: FR-109-AC-11 to FR-109-AC-13.

## Test Procedure

1. Run `NoCycle`-shaped clauses whose bodies nest `implies` under `and`, `or`,
   `not` and `implies`, with true and false antecedents from literals and from
   the snapshot; compile the same source independently and read the checked
   graph's source-map regions of each `implies` node and its consequent.
2. Run a `forall`, an `exists` and a function-calling body.
3. Run with a zero work budget; a `Function` selection; a temporal clause
   over a lasso that holds, one that is violated and a finite pending prefix.

Tag the tests `#[trace("TC-1269", "FR-109-AC-n")]`.

## Expected Results

- Step 1: `Produced`, one event per `implies` node in pre-order, index equal to
  position, equal `package_id`, regions equal to the source map's, expected
  states; a clause with no implication is `Produced` and empty.
- Step 2: `Refused` with the under-binder or called-function code; truth
  unchanged.
- Step 3: `Unavailable`; `Unavailable`; `Refused(temporal-clause)` twice, then
  `Unavailable`.
