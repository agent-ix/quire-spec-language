---
id: TC-553
title: "Concrete steps are decided against an abstract protocol with internal closure and observation labels"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-148
    type: verifies
---
# TC-553: Concrete steps are decided against an abstract protocol with internal closure and observation labels

## Description

Verify the abstract protocol state sets with internal closure, visible and
internal concrete steps, refutations that replay, and QSpec FR-177's
divergence-freedom check on the same product.

Scope: FR-148-AC-1 to FR-148-AC-5.

## Test Procedure

Fixtures: `Twice`, `Once` and `CasTwice` (FR-147, FR-148); the lost-update
model (ADR-020 §8) with `peek` removed; `CasTwice` with `peek` kept.

1. Run `check_initial` and the steps `beginA`, `commitA`, then a second
   commit, over `CasTwice`.
2. Check `CasTwice` over universe `counters = {c}`.
3. Check the lost-update model against `Twice` and replay its
   counterexample.
4. Check `CasTwice` against `Once` and replay its counterexample.
5. Check `CasTwice` with `peek` kept and mapped `-> stutter`, with no
   fairness rows.

Tag the tests `#[trace("TC-553", "FR-148-AC-n")]`.

## Expected Results

- Step 1: one state at `a1`, `value` 0; the state at `a2`, `value` 1; the
  finished state with `value` 2.
- Step 2: `proved`, `closed-scope`, `Uncertified`.
- Step 3: `refuted`, `NoAbstractMatch` at the commit that leaves `value`
  unchanged; `reproduced-with-evaluated-witness`.
- Step 4: `refuted`, `NoAbstractMatch{position: 4}` at the second commit;
  reproduced.
- Step 5: `refuted` by the divergence-freedom check, a lasso whose loop is
  one `peek` step at the initial state.
