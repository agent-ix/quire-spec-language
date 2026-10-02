---
id: FR-157
title: "Make writes to a fairness class's enabling footprint visible under partial-order reduction"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-019
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-021
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-123
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-155
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-156
    type: depends_on
---
# FR-157: Make writes to a fairness class's enabling footprint visible under partial-order reduction

## Description

Ample sets that satisfy C0 to C3 keep a stuttering-equivalent representative
of every behaviour on the atoms, but not necessarily a fair one: a reduced
path may add a transition the behaviour never takes, and if that transition
changes whether a fairness class is enabled, the only representative of a
fair behaviour can be unfair, and the fairness filter then discards a real
violation. Under a non-empty fairness set, QSL's `model_check` SHALL treat as
visible every transition whose write footprint meets the enabling footprint
of a member of a fairness class it does not belong to (ADR-021 POR-10),
where the enabling footprint is the member's whole read footprint, its
membership included (FR-155). With
that rule the reduced product keeps a fair accepting cycle whenever the full
product has one, for weak and strong constraints of either granularity
(POR-11).

## Use case

A verification operator checks a liveness claim under `fair weak go`, where
`go` waits on two flags that other operations set. The reduction must not
explore only the paths where the flags get set and `go` then waits forever
enabled, because the real violation is a path where the flags are never set
and `go` is never enabled (US-019).

## Inputs

- The clause's fairness set (FR-123), authored and derived constraints
  alike, including ADR-027's scheduler constraints over a protocol subject.
- Each transition's `Footprint` (FR-155), including its enabling footprint.

## Outputs

The visibility of each transition identity for the run, which FR-156's C2
reads.

## Behavior

- **Fairness classes.** The engine SHALL take as each constraint's class
  the set of transition identities it covers: every identity of
  the operation for `whole`, the one identity for `each` (ADR-018 FA-1),
  and for a protocol subject the steps ADR-027 PA-1 assigns to the target.
- **Fairness visibility.** When the fairness set is non-empty, a transition
  `t` SHALL be visible when, for some class `K` with `t ∉ K` and some member
  `k ∈ K`, a location of `t`'s write footprint meets a location of `k`'s
  enabling footprint (FR-155 "Meeting"). This adds to FR-156's visibility
  rule; it does not remove any visible transition.
- **Class membership.** A transition that adds a member to a class or
  removes one writes that member's `Membership` location, which is in its
  enabling footprint, so a `creates` that adds a member to a `whole` class,
  and a `deletes` that removes one, SHALL be visible under this rule.
- **Empty fairness set.** When the fairness set is empty, this rule SHALL add
  no visible transition.
- **Unchanged conditions.** C0, C1, C3 and the proviso SHALL be as FR-156
  states; only C2 reads the extended visibility.
- **Fairness filter.** The filter SHALL read each state's full enabled set
  (FR-126), never the ample set.
- **Determinism.** Visibility SHALL be a function of the subject, the
  fairness set and the item's atoms.

## Acceptance Criteria

The `Gate` unit: object type `Gate` with fields `a`, `c`, `d`, `done`, all
`Boolean`; population `gates`, universe `{g}`; initial state all false; four
operations, each with a receiver-scoped frame: `arm()` (`modifies self.c`,
pre `not self.c`, post `self.c`); `arm2()` (`modifies self.d`, pre `self.c
and not self.d`, post `self.d`); `go()` (`modifies self.done`, pre `self.c
and self.d and not self.done`, post `self.done`); and `toggle()` (`modifies
self.a`, post `self.a = not pre(self.a)`). Canonical transition order is
`arm`, `arm2`, `go`, `toggle`. A state is written (`a`, `c`, `d`, `done`).
The claim `Opens` is `eventually holds(x.done)` over `(x: M::Gate)`. The
enabling footprint of `go(g)` is `{Field{g, c}, Field{g, d}, Field{g, done},
Membership{gates, g}}`:
`go`'s enabledness reads two locations, `c` and `d`, that other operations
write.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-157-AC-1 | `Opens` under `fair weak go` with partial-order reduction. `arm` and `arm2` are visible: each writes a location of `go(g)`'s enabling footprint and neither is in `go`'s class. `go` is visible by the atom. `toggle` is invisible. At `(F,F,F,F)` the ample set is `{toggle}`, and at `(T,F,F,F)` the candidate `{toggle}` fails C3, so that state is fully expanded. The reduced product keeps the cycle `(F,F,F,F) ⇄ (T,F,F,F)`, where `go` is disabled, and that cycle passes the weak fairness filter. The run returns `Violated` with an empty stem and the loop `toggle, toggle`, the same verdict and lasso as the unreduced run. | Test (TC-578) |
| FR-157-AC-2 | Why the rule matters, over the same subject and claim. With `arm` and `arm2` invisible, `{arm}` satisfies C0 to C3 at `(F,F,F,F)`: `arm2` and `go` cannot run before `arm`, `arm` is invisible, and its successor is new. It wins the tie with `{toggle}` on canonical order. `{arm2}` then wins at `(F,T,F,F)`. The only accepting cycle left is `(F,T,T,F) ⇄ (T,T,T,F)`, on which `go` is enabled throughout and never taken, so the filter rejects it and the run would return `Holds`, a wrong proof. The test computes both ample-set choices at `(F,F,F,F)` through the visibility function, once with the fairness set and once with the empty set. It asserts `{toggle}` with the fairness set and `{arm}` with the empty set, and asserts that the AC-1 run returns `Violated`. | Test (TC-578) |
| FR-157-AC-3 | `Opens` with the empty fairness set and partial-order reduction: no fairness visibility applies, `arm` and `arm2` are invisible, and the ample sets are `{arm}` at `(F,F,F,F)` and `{arm2}` at `(F,T,F,F)`. The run returns `Violated` with stem `arm, arm2` and loop `toggle, toggle`. The unreduced run returns `Violated` with an empty stem and loop `toggle, toggle`. Both replay (FR-128). | Test (TC-579) |
| FR-157-AC-4 | Granularity: with `fair weak each go` the classes are one identity each, and AC-1's visibility and verdict are unchanged. ADR-021 §7.4 over `ProtocolSystem` (ADR-027 footprints and default scheduler constraints, partial-order reduction): `fork`, `send(S)` and `attempt(D)` are visible; `duplicate`, `lose`, `join` and `finish` are invisible. The item returns `Holds`, which settles `proved` with basis `Reduced{[PartialOrder{BreadthFirstRevisit}]}`. With `scheduling adversarial` the fairness set is empty and the item returns `Violated` with the lasso `fork`, `send(S)`, then the loop `duplicate`, `lose`. Each verdict equals the unreduced verdict. | Test (TC-579) |
| FR-157-AC-5 | ADR-021 §7.6's `Gate` with `go()`'s pre `not self.done` and post `self.done and pre(self.c) and pre(self.d)`, under `fair weak go` with partial-order reduction: `go(g)`'s enabling footprint still holds `Field{g, c}` and `Field{g, d}`, so `arm` and `arm2` are visible; the ample set at `(F,F,F,F)` is `{toggle}`, and the run returns `Violated` with the loop `toggle, toggle` over 6 states, as the unreduced run does. | Test (TC-617) |
| FR-157-AC-6 | `Gate` with an operation `spawn()` framed `creates Gate`, under `fair weak whole go`: `spawn` writes `AnyMembership{gates}`, which meets `Membership{gates, g}` in `go(g)`'s enabling footprint, so `spawn` is visible. | Test (TC-617) |

## Dependencies

- ADR-021 POR-10, POR-11, PT-2 (fairness rows), §3 "Strong fairness under
  POR", §7.4; ADR-018 FA-1 to FA-4; ADR-019 SR-1 (full enabled sets).
- [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md)
  (the fairness set), [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (the fairness filter), [FR-155](FR-155-derive-and-enforce-read-and-write-footprints.md)
  (enabling footprints), [FR-156](FR-156-expand-ample-sets-with-the-breadth-first-proviso.md)
  (C2 and the choice).
- ADR-027 PA-1 to PA-3 (protocol fairness classes and scheduler constraints)
  and FT-1 to FT-5 (protocol footprints).
- QSpec owns the preservation table's fairness rows (ADR-021 QS-5).

## References

- QSpec half: QSpec FR-383 and FR-384 (Linear STD-134; ADR-021 §9 QS-2, QS-5). Protocol transition
  system: ADR-027, Linear QSL-396. Strong fairness: ADR-019, Linear QSL-365.
  Owning ticket: Linear QSL-368.
