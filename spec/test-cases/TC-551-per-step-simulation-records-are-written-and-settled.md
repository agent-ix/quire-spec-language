---
id: TC-551
title: "Per-step simulation records are written for functional refinements and never settle refuted"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-146
    type: verifies
---
# TC-551: Per-step simulation records are written for functional refinements and never settle refuted

## Description

Verify which `operation-contract` records the refinement's `requirements`
hook writes, their hypotheses, and how their backend outcomes settle.

Scope: FR-146-AC-1 to FR-146-AC-4.

## Test Procedure

Fixtures: `CasRefinesCounter` and its lost-update model (ADR-020 §8);
`RegisterHistory` (FR-138); `Coin` with `side` hidden (FR-141-AC-4).

1. Write the requests for `CasRefinesCounter`, and for it with `commitB ->
   any`.
2. Write the requests for `RegisterHistory` and `Coin`.
3. Add the invariant `CasInv` to `Impl::Counter` and write the request
   again.
4. Settle a backend "holds" for the `commitA` record, a backend
   counterexample for the lost-update `commitB` record from `(1, 0, f, 0,
   t)`, and each record with no SMT candidate registered.

Tag the tests `#[trace("TC-551", "FR-146-AC-n")]`.

## Expected Results

- Step 1: eight records (one `Initial`, seven `Step`) with empty hypotheses;
  none with `commitB -> any`.
- Step 2: no `StepSimulationRecord`; one refinement record each.
- Step 3: every record lists `CasInv`; `CasInv`'s own record is present; the
  `Initial` identity differs from step 1's.
- Step 4: `proved`, `decisive-witness`, `Proved{Inductive{depth: 1}}`;
  `inconclusive`, `Inconclusive(InductionNotClosed{depth: 1})`;
  `unsupported`.
