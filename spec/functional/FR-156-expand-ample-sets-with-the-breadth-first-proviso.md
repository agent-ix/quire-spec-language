---
id: FR-156
title: "Expand ample sets with the breadth-first cycle proviso"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-019
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-021
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-123
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-124
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-155
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-162
    type: depends_on
---
# FR-156: Expand ample sets with the breadth-first cycle proviso

## Description

Under partial-order reduction, QSL's `model_check` SHALL expand at each
product state an ample subset `A(s)` of the enabled transitions `E(s)` that
satisfies Peled's conditions C0 to C3 (ADR-021 POR-6), chosen
deterministically from stubborn-set candidates (POR-7), with C3 in its
breadth-first form: a reduced expansion is admitted only when every
successor through it is a product state not yet discovered (POR-8). A
transition is visible when it can change an atom's value (POR-5); only
invisible transitions are left out. Under a non-empty fairness set FR-157
adds fairness visibility.

## Use case

A verification operator checks `eventually holds(x.v = 2)` over three
independent counters. Only `bump(x)` can change the atom, so the engine
explores one interleaving of the other bumps, 7 states in place of 27, and
proves the claim (US-019).

## Inputs

- A product state with model state `s`, its full enabled set `E(s)` in
  canonical transition order (FR-101), and each member's `Footprint`
  (FR-155).
- The item's atoms and, when selected, the state constraint (FR-158).
- The set of product states discovered so far.

## Outputs

`A(s) ⊆ E(s)`, and whether `s` is fully expanded (`A(s) = E(s)`).

## Behavior

- **Atom footprints.** S3 SHALL derive the read footprint of each atom of a
  checked temporal clause as FR-155 derives a clause's, with the `over`
  parameter symbolic, and record which atoms read the operation anchor of
  their position and the operations they name. `model_check` SHALL
  instantiate each atom footprint per instance.
- **Visibility.** A transition SHALL be visible when its write footprint
  meets a location an atom reads, or a location the state constraint reads,
  or when an atom reads the anchor of its position and names its operation.
  For a deadlock-freedom item no transition SHALL be visible by reason of
  `deadlocked` or the `terminal when` predicate; the engine evaluates both at
  each terminal state it reaches (FR-124). Every other transition SHALL be
  invisible. FR-157 adds the fairness rule.
- **Candidates.** For each enabled `t` in canonical transition order, the
  candidate seeded by `t` SHALL be the least set of transition identities of
  the subject at `s` that contains `t` and, for each enabled member, every
  transition identity dependent on it (FR-155 `independent` false), and for
  each disabled member, every transition identity whose write footprint
  meets that member's enabling footprint, which is its whole read footprint
  (FR-155).
- **Choice.** `A(s)` SHALL be the enabled part of the candidate with the
  fewest enabled members that satisfies C2 (when it is not all of `E(s)`,
  every member is invisible) and C3 (when it is not all of `E(s)`, every
  successor through it is a product state not yet discovered), ties broken
  by the seed's canonical order. When no candidate satisfies both, the
  engine SHALL expand `E(s)`.
- **C0.** `A(s)` SHALL be empty only when `E(s)` is empty.
- **Successors through `A(s)`.** The engine SHALL judge C3 on product states: the
  model successor paired with the automaton's move on it, canonicalised when
  symmetry is selected (FR-162).
- **Systems.** A request selecting partial-order reduction over a system
  whose `footprint` hook is not implemented SHALL settle V-8 in the
  pre-check (FR-159). `ModelSystem` implements it (FR-155); `ProtocolSystem`
  implements it with ADR-027 FT-1 to FT-4.
- **Determinism.** `A(s)` SHALL be a function of the subject, the item, the
  discovered set and `s`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-156-AC-1 | ADR-021 §7.2, instance `x = a`, empty fairness set, `ReachesTwo` (`eventually holds(x.v = 2)`): `bump(a)` is visible and `bump(b)`, `bump(c)` are invisible. The ample sets are, in order, `{bump(b)}` at `(0,0,0)` and `(0,1,0)`, `{bump(c)}` at `(0,2,0)` and `(0,2,1)`, then `E(s)` at `(0,2,2)` and `(1,2,2)`, and none at `(2,2,2)`. The run stores 7 model states and 6 product states for the negation and returns `Holds`; the unreduced run stores 27 model states and also returns `Holds`. | Test (TC-576) |
| FR-156-AC-2 | ADR-021 §7.2's deadlock-freedom item over the same subject with partial-order reduction selected: no transition is visible, the reduced search reaches `(2,2,2)` and returns `Violated` with `kind: Deadlock` and a six-step prefix, which is the path of AC-1's table. | Test (TC-576) |
| FR-156-AC-3 | C3 and ignoring: the `Ring` unit has object type `Ring` (`v: Int[0, 2]`, `step()` framed `modifies self.v`, post `self.v = (pre(self.v) + 1) mod 3`) with one object `r`, and object type `Counter` with `bump()` as in AC-1 and one object `q`, both at 0. The claim `always holds(q.v < 2)` (TP-1) makes `bump(q)` visible. The ample set is `{step(r)}` at `(r=0, q=0)` and `(1, 0)`; at `(2, 0)` the candidate `{step(r)}` reaches the discovered `(0, 0)`, fails C3, and the state is fully expanded, so `bump(q)` is explored. The run returns `Violated` at `q.v = 2` with the six-step prefix `step, step, bump, step, step, bump`, which replays (FR-128); the unreduced run also returns `Violated`. | Test (TC-577) |
| FR-156-AC-4 | A dependency closure through a disabled member: with `arm2()` (pre `self.c`, frame `modifies self.d`) disabled at `s` and `arm()` writing `c`, the candidate seeded by any transition dependent on `arm2` contains `arm`. Two runs of AC-1 give equal ample sets at every state and equal outcomes. | Test (TC-577) |
| FR-156-AC-5 | ADR-021 §7.6's guard-in-postcondition subject with partial-order reduction and `always holds(not g.alarm)`: at the initial state `boom` is disabled (no post-state satisfies its postcondition), the candidate seeded by `disarm` holds `boom` and, through `boom`'s enabling footprint, `set`, so the initial state is fully expanded; the run returns `Violated` with prefix `set, boom` over 6 states, as the unreduced run does. | Test (TC-589) |
| FR-156-AC-6 | ADR-021 §7.6's deleting-the-receiver subject with `always holds(not k.bad)`: `{cancel(j)}` is not an ample set, because `boom(j)` is dependent on it, and the run returns `Violated` by `boom(j)` over 4 states, as the unreduced run does. The type-wide-delete subject with Job `m` and `cancel()` framed `deletes Job`: `{cancel(m)}` is not an ample set, and the run returns `Violated` by `boom(j)` over 4 states, never `Holds`. The blocked-delete subject: `{lock(k)}` is not an ample set, and the run returns `Violated` with `UndefinedEvaluation` after `unlink(j), cancel(k)` over 5 states. The navigation subject: `{disarm}` is not an ample set, and the run returns `Violated` by `point, boom` over 6 states, as the unreduced run does. | Test (TC-589) |

## Dependencies

- ADR-021 POR-5 to POR-9, PT-2 (POR column), §2 "POR and deadlocks".
- [FR-101](FR-101-explore-finite-models-with-canonical-order-and-the-qspec-sampler.md)
  (canonical transition order), [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md)
  (atoms), [FR-124](FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md)
  (`deadlocked`), [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (the product, enabled sets), [FR-155](FR-155-derive-and-enforce-read-and-write-footprints.md)
  (footprints, independence), [FR-157](FR-157-make-fairness-enabling-writes-visible.md)
  (fairness visibility), [FR-162](FR-162-offer-reductions-through-transition-system-hooks.md)
  (the reduced expansion).
- ADR-027 FT-1 to FT-5 (protocol footprints, enabling footprints and
  visibility).
- QSpec owns ample sets C0 to C3 and the breadth-first revisit proviso as
  normative semantics (ADR-021 QS-2).

## References

- QSpec half: QSpec FR-383 (Linear STD-134; ADR-021 §9 QS-2). Protocol transition system:
  ADR-027, Linear QSL-396. Owning ticket: Linear QSL-368.
