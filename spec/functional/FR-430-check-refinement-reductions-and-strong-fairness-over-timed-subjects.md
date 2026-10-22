---
id: FR-430
title: "Check refinement, reductions and strong fairness over timed subjects"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-026
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-019
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-020
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-021
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-141
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-142
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-151
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-152
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-159
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-239
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-240
    type: depends_on
---
# FR-430: Check refinement, reductions and strong fairness over timed subjects

## Description

QSL SHALL check the three ADR-026 §15 interactions of dense time with its
sibling records. A timed concrete model refines an untimed abstract model
through its step map, a delay being a stutter step of the abstract model
(ADR-020). EN-6 applies symmetry, permuting each object together with its
clocks, and no partial-order reduction (ADR-021, POR-4). Strong fairness
reads enabledness over time, at every instant of a timed behaviour, as
ADR-026 TD-5 reads weak fairness (ADR-019).

## Use case

A specification author refines an untimed `Rpc` protocol by a timed
implementation and requests the refinement: every discrete step of the
implementation maps to an abstract step or a stutter, and the time that
passes between steps is never an abstract step. A verification operator
selects symmetry over a population of timers and gets a smaller symbolic
graph with the same verdict, and a request that also selects partial-order
reduction settles at once. A liveness claim under strong fairness over a
timed model treats an operation enabled only between positions as enabled.

## Inputs

- A `CheckedRefinement` (FR-135) whose concrete model is timed (`time
  dense`, FR-231) and whose abstract model is untimed.
- A `ModelCheckRequest`'s `Reductions` (FR-159) over a timed subject.
- A checked clause over a timed subject whose fairness set holds a
  `strong` constraint (FR-123, ADR-019).

## Outputs

- For a timed refinement: FR-142's `RefinementOutcome`, its counterexample
  a finite timed prefix with exact rational delays (FR-241).
- For reductions: the admitted reductions, or
  `ReductionNotPreserving{PartialOrder, form}`, or V-8
  `unsupported-requested-capability`, as FR-159 settles them.
- For strong fairness: FR-240's liveness outcome.

## Behavior

### Timed refinement

- S3 SHALL admit a refinement whose concrete model is timed and whose
  abstract model is untimed. If the abstract model is timed, then S3 SHALL
  refuse `unsupported_construct`/`declaration-form` at the refinement's
  abstract model reference.
- If the refinement's state mapping or a step row reads a clock field of
  the concrete model, then S3 SHALL refuse
  `unsupported_construct`/`expression-form` at the read, so the mapped
  abstract state is a function of the discrete state alone.
- A delay of the concrete model SHALL map to no abstract step: `check_step`
  (FR-141) SHALL be called only for discrete steps, and the abstract state
  SHALL be unchanged across a delay.
- The safety half SHALL be decided by EN-6 on the symbolic product whose
  states are (discrete product state of FR-142, zone), with successors by
  FR-239's symbolic successor relation and `check_step` run on each
  discrete symbolic edge in canonical breadth-first order. A failing edge
  SHALL refute only when its pre-state has a point with a time-divergent
  continuation (ADR-026 TV-1), and the counterexample SHALL be concretized
  by FR-241.
- The liveness half (FR-143) over a timed concrete subject SHALL read only
  time-divergent concrete behaviours (ADR-026 TS-4): an accepting cycle of
  the product SHALL refute only when its loop has positive total delay
  (ADR-026 EZ-6).

### Reductions over a timed subject

- A symmetry permutation SHALL permute each object of a declared class
  together with its clock fields: the zone's clock indices of object `k`
  SHALL move to the indices of the object `k` is mapped to.
- EN-6 SHALL canonicalise a symbolic state by the FR-152 permutation of
  its discrete state, ties between objects with equal discrete state broken
  by the least permutation in key order, and apply the same permutation to
  the zone. Coverage (FR-239) SHALL be tested on the canonicalised state.
- FR-159's preservation table SHALL read `false` for partial-order
  reduction and every form or item over a timed subject, so a request that
  selects partial-order reduction over a timed subject settles
  `ReductionNotPreserving{PartialOrder, form}` at pre-check step (a), with
  no state expanded.
- EN-6's provider manifest SHALL advertise `symmetry` and no other
  reduction, so a request over a timed subject that selects a state
  constraint settles V-8 `unsupported-requested-capability` naming the
  constraint at FR-159 step (c).
- The symmetry declarations SHALL enter the obligation identity as FR-159
  states for an untimed subject.

### Strong fairness over time

- A timed behaviour SHALL satisfy strong fairness for a constraint when,
  if the constraint is enabled at arbitrarily late instants (positions and
  the instants of the delays between them), one of its transitions is
  taken infinitely often. On a timed lasso (ADR-026 CT-2) the constraint is
  taken somewhere in the loop or enabled at no instant of the loop.
- EN-6 SHALL split each symbolic state by the guards of the claim's strong
  constraints as FR-240 splits by weak ones, so enabledness at an instant is
  exact on the split graph, and the SCC phase SHALL apply ADR-019's strong
  fairness filter on that graph, its certificate stating the nested
  numbering ADR-026 CF-2 names.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-430-AC-1 | An untimed abstract model `Req` (object `r` with `sent: Bool`; operation `send`, postcondition `self.sent = true`) refined by a timed model `TReq` (object `r` with `sent: Bool` and clock `x`; operation `send` with guard `x >= 1`, postcondition `self.sent = true`; step map `send -> send`; mapping `sent -> sent`) returns `Holds` for the safety half, with no `check_step` call for a delay. The variant whose mapping is `sent -> (x >= 1)` refuses `unsupported_construct`/`expression-form` at the read of `x`; a refinement whose abstract model is `TReq` refuses `unsupported_construct`/`declaration-form` at the abstract model reference. | Test (TC-1170) |
| FR-430-AC-2 | `TReq` with a second operation `undo` (guard `x >= 2`, postcondition `self.sent = false`) and step map `undo -> _` returns `Violated` for the safety half with a counterexample of `send` after delay 1 and `undo` after delay 1, `RefinementFailure` at position 2, replaying through FR-241. With the time invariant `x <= 1` the `undo` edge has no admitted pre-state point and the half returns `Holds`. | Test (TC-1170) |
| FR-430-AC-3 | A timed model with population `Timer` over universe `{t1, t2}` (each with `armed: Bool` and clock `x`; `arm` with precondition `not self.armed`, guard `x >= 1`, reset `x`; `disarm` with precondition `self.armed`, guard `x >= 1`, reset `x`): with symmetry `[[t1, t2]]`, the deadlock-freedom item returns `Holds` as with no reduction and stores fewer symbolic states, and `always holds(not (t1.armed and t2.armed))` returns `Violated` as with no reduction, its counterexample replaying; a symbolic state and its image under swapping `t1` and `t2` (discrete fields and clocks) canonicalise to equal states. The deadlock-freedom request selecting partial-order reduction settles `ReductionNotPreserving{PartialOrder, deadlock-freedom}` with no state expanded; selecting a state constraint settles V-8 `unsupported-requested-capability` naming the constraint. | Test (TC-1170) |
| FR-430-AC-4 | A `Blink` model (`time dense`; object `b` with `fired: Bool` and clock `x`; `tick` with guard `x >= 1` and reset `x`; `fire` with guard `x > 0 and x < 1` and postcondition `self.fired = true`; initial `fired = false`) and `eventually holds(b.fired)` under infinite-trace: under `fair { weak Blink::fire }` it returns `Violated` with a lasso of `tick` steps of positive loop delay, because `fire` is never continuously enabled; under `fair { strong Blink::fire }` it returns `Holds` with a certificate, because `fire` is enabled at instants inside every delay of that lasso, though at no position of it. | Test (TC-1170) |

## Dependencies

- ADR-026 §15, TS-4, TD-5, TV-1, CT-2, EZ-6, CF-2; ADR-020 RS-2 to RS-6;
  ADR-021 POR-4, RD-3; ADR-019.
- [FR-141](FR-141-decide-one-concrete-step-against-the-abstract-model.md),
  [FR-142](FR-142-check-a-refinement-s-safety-half-on-the-explicit-state-product.md),
  [FR-151](FR-151-admit-a-request-s-symmetry-declarations.md),
  [FR-159](FR-159-pre-check-selected-reductions-against-the-preservation-table.md),
  [FR-239](FR-239-check-a-timed-claim-by-symbolic-zone-search.md),
  [FR-240](FR-240-decide-timed-liveness-and-time-lock-freedom-on-the-symbolic-graph.md).
- QSpec owns the preservation table, the provider advertisement and the
  refinement and fairness readings over timed subjects (QSpec FR-377,
  FR-382, FR-416).

## References

- ADR-026 §15 (Interactions with the sibling records). Linear STD-306.
