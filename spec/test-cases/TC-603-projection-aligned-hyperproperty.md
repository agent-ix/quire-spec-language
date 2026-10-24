---
id: TC-603
title: "EN-1 checks a projection-aligned hyperproperty over the projected product"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-178
    type: verifies
---
# TC-603: EN-1 checks a projection-aligned hyperproperty over the projected product

## Description

Verify HP-6 against lockstep on ADR-023 §13's vault, a projected counterexample from a mid-operation start and from idle starts, three-valued prefix evaluation, move counting, determinism, the absence of vacuity and the ranking of rejecting and undefined states.

Scope: FR-178-AC-1 to FR-178-AC-9.

## Test Procedure

Fixtures: §13's vault with `busy` and `mix`, and its leaky variant, each started idle `(0, 0, false)` and `(1, 0, false)`, or with the secret-1 run mid-operation, `(1, 0, true)`.

1. The lockstep clause and the `align skip { V::Vault::mix }` clause over §13's vault.
2. The `align skip` clause over the leaky variant, started mid-operation and started idle.
3. Three-valued evaluation over a joint projected prefix ending with unequal `l`, and over one with equal `l` throughout.
4. Step 2's mid-operation request with `max_depth` 1 and 2; the idle request with `max_depth` 1 and 0; step 2 twice.
5. Step 1's `align skip` clause with FR-178-AC-5's undefined `match` conjunct, and with FR-178-AC-6's meter-exhausting one.
6. Step 1's `align skip` clause with the added `match` conjunct `false`.
7. FR-178-AC-8's two requests, in which a rejecting state and an undefined `match` conjunct lie at one depth.
8. FR-178-AC-5's request with both runs starting mid-operation.

Tag the tests `#[trace("TC-603", "FR-178-AC-n")]`.

## Expected Results

- Step 1: `Violated`; `Holds{Exhaustive}`.
- Step 2: `Violated` with a `Projected` counterexample at `trace_position` 1: one visible step per trace, total lengths 1 and 2 from the mid-operation start, 1 and 1 from idle starts, no loop entry, each step marked.
- Step 3: `false`; no `false`.
- Step 4: `BoundReached{depth: 1}` then the counterexample; the counterexample, then `BoundReached{depth: 0}`; byte-equal counterexamples.
- Step 5: `Violated` with an unmarked `Undefined` counterexample, cause `division-by-zero`; `Undecided(MatchUndetermined)`.
- Step 6: `Holds{Exhaustive}`, never `Undecided(VacuousMatch)`.
- Step 7: the `Undefined` counterexample; the `Projected` counterexample.
- Step 8: an unmarked `Undefined` counterexample whose paths each hold a skipped `mix` and `step(0)`, at position 1.
