---
id: TC-1170
title: "Timed refinement, symmetry with clocks, partial-order refusal and strong fairness over time"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-430
    type: verifies
---
# TC-1170: Timed refinement, symmetry with clocks, partial-order refusal and strong fairness over time

## Description

Verify the ADR-026 §15 interactions: a timed model refining an untimed one with delays as abstract stutters, symmetry permuting objects with their clocks on EN-6, partial-order reduction and state constraints settling at pre-check over a timed subject, and strong fairness read over time.

Scope: FR-430-AC-1 to FR-430-AC-4.

## Test Procedure

Fixtures: the `Req` and `TReq` models of FR-430-AC-1, the `undo` variants of FR-430-AC-2, the `Timer` model of FR-430-AC-3 and the `Blink` model of FR-430-AC-4.

1. Check the `TReq` refines `Req` refinement and its clock-reading and timed-abstract variants (FR-430-AC-1).
2. Check the `undo` refinement with and without the time invariant `x <= 1` (FR-430-AC-2).
3. Check the deadlock-freedom item and the safety claim over `Timer` with and without symmetry; canonicalise a state and its swapped image; request partial-order reduction and a state constraint (FR-430-AC-3).
4. Check `eventually holds(b.fired)` over `Blink` under weak and under strong fairness on `fire` (FR-430-AC-4).

Tag the tests `#[trace("TC-1170", "FR-430-AC-n")]`.

## Expected Results

- Step 1: `Holds` with no `check_step` call for a delay; the two variants refuse with the codes and loci FR-430-AC-1 names.
- Step 2: `Violated` at position 2 with delays 1 and 1, replaying; `Holds` with the invariant.
- Step 3: equal verdicts with and without symmetry, fewer stored symbolic states with it, equal canonical states; `ReductionNotPreserving{PartialOrder, deadlock-freedom}` and V-8 `unsupported-requested-capability`, each with no state expanded.
- Step 4: `Violated` with a `tick` lasso of positive loop delay under weak fairness; `Holds` with a certificate under strong fairness.
