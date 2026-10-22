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
  - target: ix://agent-ix/quire-spec-language/FR-144
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
- A `ModelCheckRequest`'s `Reductions` (FR-159) over a `time dense` subject.
- A checked clause over a timed subject whose fairness set holds a
  `strong` constraint (FR-123, ADR-019).

## Outputs

- For a timed refinement: FR-142's `RefinementOutcome`, its counterexample
  a finite timed prefix with exact rational delays (FR-241).
- For reductions: the admitted symmetry, or V-8
  `unsupported-requested-capability` at negotiation (FR-159 Capability).
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
- Negotiation SHALL route the refinement's record (FR-144) whose concrete
  subject declares `time dense` to EN-6's provider, as FR-144 states. A
  refinement record over an untimed or `time tick` concrete subject keeps
  FR-144's route to the explicit-state engine.
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

### Reductions over a dense-time subject

The rules of this section apply to a subject that declares `time dense`
(EN-6's subjects); a `time tick` subject is an untimed subject read through
digital time (ADR-026 TS-6) and keeps FR-159 unchanged.

- A symmetry permutation SHALL permute each object of a declared class
  together with its clock fields: the zone's clock indices of object `k`
  SHALL move to the indices of the object `k` is mapped to.
- EN-6 SHALL canonicalise a symbolic state by the FR-152 permutation of
  its discrete state, ties between objects with equal discrete state broken
  by the least permutation in key order, and apply the same permutation to
  the zone. Coverage (FR-239) SHALL be tested on the canonicalised state. A
  state and its image under a permutation canonicalise to equal states when
  the discrete permutation is unique (no two objects of a class have equal
  discrete state); otherwise they may stay distinct, which is sound and
  only reduces less.
- EN-6's provider manifest SHALL advertise `symmetry` and no other
  reduction. So, by FR-159's Capability rule, negotiation SHALL settle an
  item over a dense-time subject that selects partial-order reduction or a
  state constraint V-8 `unsupported-requested-capability`, naming the first
  such reduction in the order partial-order, constraint, with no candidate
  run and no state expanded.
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
| FR-430-AC-1 | An untimed abstract model `Req` (object `r` with `sent: Bool`, initial `sent = false`; operation `send`, postcondition `self.sent = true`) refined by a timed model `TReq` (`time dense`; object `r` with `sent: Bool` and clock `x`, initial `sent = false` and `x = 0`; operation `send` with guard `x >= 1`, postcondition `self.sent = true`; step map `send -> send`; mapping `sent -> sent`) returns `Holds` for the safety half, with no `check_step` call for a delay, and the request routes to EN-6. The variant whose mapping is `sent -> (x >= 1)` refuses `unsupported_construct`/`expression-form` at the read of `x`. A refinement whose abstract model is `TReq` and whose concrete model is a copy `TReq2` of `TReq` refuses `unsupported_construct`/`declaration-form` at the abstract model reference. | Test (TC-1170) |
| FR-430-AC-2 | `TReq` extended with `trap: Bool` (initial `false`), `send` resetting `x`, an operation `enter` (precondition `self.sent`, guard `x >= 1`, reset `x`, postcondition `self.trap = true`) and an operation `undo` (precondition `self.trap and self.sent`, postcondition `self.sent = false`), with step map `send -> send`, `enter -> stutter`, `undo -> stutter`: the safety half returns `Violated` with the counterexample `send` after delay 1, `enter` after delay 1, `undo` after delay 0, `RefinementFailure` at position 3, replaying through FR-241. With the time invariant `when self.trap { x <= 0 }`, the only pre-state points of `undo` (trap set, `x = 0`) reach no time-divergent continuation, so the failing edge does not refute (ADR-026 TV-1) and the half returns `Holds`, while the time-lock-freedom item of that model is refuted. | Test (TC-1170) |
| FR-430-AC-3 | A `time dense` model with population `Timer` over universe `{t1, t2}` (each with `armed: Bool` and clock `x`; `arm` with precondition `not self.armed`, guard `x >= 1`, reset `x`; `disarm` with precondition `self.armed`, guard `x >= 1`, reset `x`): with symmetry `[[t1, t2]]`, the deadlock-freedom item returns `Holds` as with no reduction and stores fewer symbolic states, and `always holds(not (t1.armed and t2.armed))` returns `Violated` as with no reduction, its counterexample replaying. The symbolic state with `t1` armed, `t2` unarmed and zone `Z`, and its image with `t1` unarmed, `t2` armed and `Z` with the clocks of `t1` and `t2` swapped, canonicalise to equal states. The deadlock-freedom item selecting partial-order reduction, and the same item selecting a state constraint, each settle V-8 `unsupported-requested-capability` at negotiation, naming the reduction, with no state expanded. | Test (TC-1170) |
| FR-430-AC-4 | A `Blink` model (`time dense`; object `b` with `fired: Bool` and clock `x`; `tick` with guard `x >= 1` and reset `x`; `fire` with guard `x > 0 and x < 1` and postcondition `self.fired = true`; time invariant `when true { x <= 1 }`; initial `fired = false`, `x = 0`), in which `tick` fires exactly at `x = 1` and every delay between ticks is 1, and `eventually holds(b.fired)` under infinite-trace: under `fair { weak Blink::fire }` it returns `Violated` with a lasso of `tick` steps of loop delay 1, because `fire` is never continuously enabled; under `fair { strong Blink::fire }` it returns `Holds` with a certificate, because `fire` is enabled at instants inside every delay of that lasso, though at no position of it. | Test (TC-1170) |

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
