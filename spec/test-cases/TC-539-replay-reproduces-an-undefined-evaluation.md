---
id: TC-539
title: "Replay reproduces an undefined claim evaluation at its position"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: verifies
---
# TC-539: Replay reproduces an undefined claim evaluation at its position

## Description

Verify that `replay_model_trace` reproduces an `UndefinedEvaluation`
counterexample by evaluating the replayed letters and finding the first
undefined one at `where` with the recorded cause, and that any other
position or a missing position is a disagreement.

Scope: FR-128-AC-5 to FR-128-AC-8.

## Test Procedure

Take TC-537 step 3's first counterexample, with the `Counter` snapshot and
source in the byte provision and universe `{c}` in the request.

1. Replay it.
2. Replay it with `where` set to 1.
3. Replay it with its last step removed, so it ends at value 1; then with
   `where.locus` naming another expression of the clause.
4. Replay TC-537 step 4's deadlock-freedom counterexample (`terminal when
   6 mod (3 - c.value) = 0`, prefix to value 3).
5. Replay that prefix with `kind: Deadlock`.
6. Replay FR-126-AC-6's empty-prefix counterexample of `always holds(true)`
   over the `Counter` subject with `pre CanInc { 6 mod (2 - self.value) >= 0 }`
   from value 2, with the `CanInc` locus, then with the clause's atom as
   locus.
7. Add FR-128-AC-7's `pre CanInc { 6 mod (3 - self.value) >= 0 }` to
   step 4's subject and replay the prefix to value 3 with `where.locus`
   naming `CanInc`, then with it naming the `terminal when` predicate.
8. Run FR-128-AC-8's constrained claim over ADR-021 §7.3's subject with
   `constraint Odd`, and replay its counterexample through a request that
   selects no reduction; then with its last step removed; then with `cause`
   changed; then in an envelope over the unit declaring `Odd` as an
   `invariant`, with that unit's `package_id`.

Tag the tests `#[trace("TC-539", "FR-128-AC-n")]`.

## Expected Results

- Step 1: `reproduced-with-evaluated-witness`, `trace_position` 2, value
  `UndefinedEvaluation{where: position 2, cause: division-by-zero}`.
- Steps 2 and 3: `inconclusive`, `ReplayParity`, each time.
- Step 4: `reproduced-with-evaluated-witness`, `trace_position` 3, value
  `UndefinedEvaluation{where: position 3, cause: division-by-zero}`.
- Step 5: `reproduced-with-evaluated-witness`, value
  `Undefined(UndefinedEvaluation{where: position 3, cause: division-by-zero})`.
- Step 6: `reproduced-with-evaluated-witness`, `trace_position` 0, value
  `UndefinedEvaluation{where: position 0, locus: CanInc, cause:
  division-by-zero}`; then `inconclusive`, `ReplayParity`.
- Step 7: `reproduced-with-evaluated-witness`, `trace_position` 3, with
  the `CanInc` locus; `inconclusive`, `ReplayParity` with the predicate
  locus.
- Step 8: `reproduced-with-evaluated-witness` with the
  `UndefinedEvaluation{where, cause: division-by-zero}` value and
  `where.locus` naming `Odd`; `inconclusive`, `ReplayParity`, twice; refused
  `stale_dependency`/`content-mismatch` naming the locus.
