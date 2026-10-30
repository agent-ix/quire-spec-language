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
Scope: FR-491-AC-1 to FR-491-AC-8.

## Test Procedure

Test layer: integration tests `qsl-replay/tests/clause_evaluation_facade.rs`
(calls and result types from the `qsl_replay` root; only its model fixture reads
`qsl_semantics` for the digest the unit's `model` statement names) and
`qsl-replay/tests/clause_evaluation_root_only.rs` (root items only,
FR-491-AC-6), plus unit tests in `src/clause_evaluation.rs`.

1. Evaluate `ParentOrder` at current snapshots that satisfy and violate it.
2. Evaluate `ParentPresent` at pre-call snapshots with and without a parent.
3. Evaluate each clause at the other kind's observation.
4. Evaluate `10 mod self.versionNumber >= 0` at versions 0 and 1; settle
   `Undefined`/`Completed` outcomes and input refusals (unit tests).
5. Evaluate a snapshot over `observation_limits.document_bytes` and at it.
6. Evaluate an undeclared clause, a non-snapshot byte string, and a clause
   under `work_units = 0`.

Tag the tests `#[trace("TC-1274", "FR-491-AC-n")]`.

## Expected Results

- Step 1: `Completed(true)` then `Completed(false)`. Step 2: `Completed(true)`
  then `Completed(false)`.
- Step 3: `Refused(WrongObservation)`.
- Step 4: `Undefined` (division-by-zero) then `Completed(true)`. Step 5:
  `Incomplete(Observation)` then `Completed(true)`. Step 6: `Refused(MissingClause)`,
  `Refused(UnreadableObservation)`, `Incomplete(Accounting)`.
