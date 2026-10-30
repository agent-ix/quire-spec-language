---
id: TC-1274
title: "The replay facade evaluates a state clause at a kind-correct observation"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-491
    type: verifies
---
# TC-1274: The replay facade evaluates a state clause at a kind-correct observation

## Description

Verify `evaluate_clause` returns `Completed`, `Undefined`, `Refused` and
`Incomplete` results at current and pre-call observations, refuses an
observation of the wrong kind, and is reachable from the `qsl_replay` root.
Scope: FR-491-AC-1 to FR-491-AC-6.

## Test Procedure

Test layer: an integration test, `qsl-replay/tests/clause_evaluation_facade.rs`,
that imports only from the `qsl_replay` root (and the model fixture helpers).

1. Evaluate `ParentOrder` at current snapshots that satisfy and violate it.
2. Evaluate `ParentPresent` at pre-call snapshots with and without a parent.
3. Evaluate each clause at the other kind's observation.
4. Settle an S6a `Undefined` outcome of the clause (a unit test in `src/clause_evaluation.rs`).
5. Evaluate an undeclared clause, a non-snapshot byte string, and a clause
   under `work_units = 0`.

Tag the tests `#[trace("TC-1274", "FR-491-AC-n")]`.

## Expected Results

- Step 1: `Completed(true)` then `Completed(false)`. Step 2: `Completed(true)`
  then `Completed(false)`.
- Step 3: `Refused(Coded)` with `wrong_snapshot`/`wrong-observation`.
- Step 4: `Undefined`. Step 5: `Refused(MissingClause)`,
  `Refused(UnreadableObservation)`, `Incomplete(Accounting)`.
