---
id: TC-1170
title: "Timed refinement, symmetry with clocks, reductions settled at negotiation and strong fairness over time"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-430
    type: verifies
---
# TC-1170: Timed refinement, symmetry with clocks, reductions settled at negotiation and strong fairness over time

## Description

Verify the ADR-026 §15 interactions: a timed model refining an untimed one with delays as abstract stutters and only positive-delay cycles refuting its liveness half, symmetry permuting objects with their clocks on EN-6, partial-order reduction and state constraints over a `time dense` subject settling V-8 at negotiation, and strong fairness read over time.

Scope: FR-430-AC-1 to FR-430-AC-8.

## Test Procedure

Fixtures: the `Req`, `TReq` and `TReq2` models of FR-430-AC-1, the `trap`/`undo` variants of FR-430-AC-2, the `Timer` model of FR-430-AC-3 the `Blink` model of FR-430-AC-4 the `poll` variants of FR-430-AC-5, the `dead` variants of FR-430-AC-6 and the initial-state variants of FR-430-AC-7.

1. Check the `TReq` refines `Req` refinement and its clock-reading and timed-abstract variants (FR-430-AC-1).
2. Check the `trap`/`undo` refinement with and without the time invariant `when self.trap { x <= 0 }`, and the time-lock-freedom item of the invariant variant (FR-430-AC-2).
3. Check the deadlock-freedom item and the safety claim over `Timer` with and without symmetry; canonicalise the state with `t1` armed and `t2` unarmed and its swapped image; request partial-order reduction, a state constraint, and both together (FR-430-AC-3).
4. Check `eventually holds(b.fired)` over `Blink` under weak and under strong fairness on `fire` (FR-430-AC-4).
5. Check the liveness half of the `poll` refinement with and without the time invariant `when not self.sent { x <= 0 }` (FR-430-AC-5).
6. Check the `dead` refinement of FR-430-AC-6 with and without the invariant `when self.dead { x <= 0 }` (FR-430-AC-6).
7. Check the failing-initial-state refinement with and without the concrete invariant `when true { x <= 0 }`, and the two `Undetermined` `RegisterHistory` variants over a `time dense` `Register`, one with the time invariant on `value` (FR-430-AC-7).
8. Read the certificates of steps 1 and 6 and check each with FR-245; check the tampered variants, and drive the settlement path with a `Holds` outcome whose certificate is withheld to show it does not settle `proved` (FR-430-AC-8).

Tag the tests `#[trace("TC-1170", "FR-430-AC-n")]`.

## Expected Results

- Step 1: `Holds` with no `check_step` call for a delay, routed to EN-6; the two variants refuse with the codes and loci FR-430-AC-1 names.
- Step 2: `Violated` at position 3 with delays 1, 1 and 0, replaying; `Holds` with the invariant, and the time-lock-freedom item refuted.
- Step 3: equal verdicts with and without symmetry, fewer stored symbolic states with it, equal canonical states; V-8 `unsupported-requested-capability` at negotiation for each of partial-order reduction and the state constraint, with no state expanded; the request selecting both names partial-order reduction only.
- Step 4: `Violated` with a `tick` lasso of loop delay 1 under weak fairness; `Holds` with a certificate under strong fairness.
- Step 5: liveness `Checked(HoldsReduced{reductions: []})` with the invariant (the only abstract-unfair loop has zero total delay); without it, liveness `Checked(Violated)` with an empty stem, a one-`poll` loop of delay 1 and `Divergence{constraint: send}`, replaying.
- Step 6: `Holds` with the `undo` post-state a `locked` node and the time-lock-freedom item refuted when `undo` sets `dead`; `Violated` at position 3 with delays 1, 1 and 0, the last point inside the post-state, replaying, when it does not.
- Step 7: `Violated` with `InitialNotAbstract{initial: 0}` and an empty prefix; with the invariant, `inconclusive` `NoAdmittedBehaviour`; both `Undetermined` variants `Undecided(MappingUndetermined)` naming `writes`.
- Step 8: `Accepted` for each honest certificate and `Proved{basis: ZoneCertified, certification: Certified}`; `Rejected` `StepFails` at the `ZoneEdge` with the `locked` mark removed; a withheld certificate never settles `proved`.
