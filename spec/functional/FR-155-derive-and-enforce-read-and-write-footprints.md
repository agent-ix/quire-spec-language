---
id: FR-155
title: "Derive and enforce read and write footprints of model transitions"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-019
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-021
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-154
    type: depends_on
---
# FR-155: Derive and enforce read and write footprints of model transitions

## Description

Partial-order reduction needs to know which locations each transition reads
and writes. S3 SHALL derive each operation's read footprint from its checked
`pre` and `post` clause bodies, with the receiver and parameters symbolic,
and its write footprint from its frame (ADR-021 POR-1). `ModelSystem` SHALL
instantiate both per transition identity, and SHALL enforce them on every
step it takes for the model checker: writes through `check_frame` at entry
scope (FR-154), reads by evaluating each clause over an observation
restricted to the read footprint (POR-2). A footprint is therefore a checked
fact about the model. The locations, their meets and the derivation of
`W(t)`, `R(t)` and the enabling footprint are QSpec FR-383's; this
requirement implements them and does not restate them. Two transitions are
independent when neither writes a location the other reads or writes
(POR-4).

## Use case

A verification operator selects partial-order reduction. The engine treats
`bump` on counter `a` and `bump` on counter `b` as independent because their
footprints are disjoint, and that independence cannot be wrong, because a
step that read or wrote outside its footprint would stop the run (US-019).

## Inputs

- The checked `pre` and `post` clauses of each operation (FR-104) and its
  `OperationEffect` with scoped entries (FR-154).
- A transition identity (FR-120): operation, receiver and arguments.

## Outputs

`Location`, `Footprint` and `independent` are owned by `qsl-eval`
(`qsl_eval::simulation`), beside FR-101's `TransitionSystem`, whose
`footprint` hook returns a `Footprint`.

```rust
pub enum Location {
    Field { object: ObjectReference, field: DeclarationKey },
    AnyField { field: DeclarationKey },          // the field on any object
    Membership { population: DeclarationKey, object: ObjectReference },
    AnyMembership { population: DeclarationKey }, // membership of any object
    Protocol(ProtocolLocation),                  // ADR-027 FT-1
}

pub struct Footprint {
    pub reads: BTreeSet<Location>,
    pub writes: BTreeSet<Location>,
    pub enabling: BTreeSet<Location>,            // = reads for a model transition; ADR-027 FT-3's for a protocol step
}

pub fn independent(t: &Footprint, u: &Footprint) -> bool;
```

The checked package records, per operation, its **symbolic footprint**: the
same sets with `self.f` as (`Receiver`, `f`), `p.f` as (`Param(p)`, `f`), the
receiver's and a reference parameter's membership as `Membership` over
`Receiver` and `Param(p)`, and `AnyField`, `AnyMembership` as above.

## Behavior

- **Derivation.** S3 SHALL compute each operation's symbolic read, write
  and enabling footprints exactly by QSpec FR-383's "Locations", "Write
  footprint", "Read footprint" and "Enabling footprint", with `self` as
  `Receiver` and a reference parameter `p` as `Param(p)`. `Field`,
  `AnyField`, `Membership` and `AnyMembership` encode FR-383's four model
  location forms, and the enabling footprint of a model transition SHALL
  equal its read footprint. A `deletes` entry's scope is read from the
  frame (FR-154): `deletes self` and `deletes p` write the receiver's or the
  argument's `Membership`, and a type-wide `deletes T` writes
  `AnyMembership`.
- **Instantiation.** `ModelSystem::footprint(transition)` SHALL replace
  `Receiver` with the transition's receiver and `Param(p)` with the
  argument bound to `p`; a `Param(p)` whose argument is absent SHALL give no
  location.
- **Meeting.** Two locations SHALL meet exactly when QSpec FR-383 says they
  do; protocol locations meet as ADR-027 FT-1 states. A protocol step's
  enabling footprint SHALL be ADR-027 FT-3's set, which holds the binders
  its arguments, constraint, guard or retry relation read.
- **Independence.** `independent(t, u)` SHALL be true exactly when no
  location of `t.writes` meets one of `u.reads ∪ u.writes` and no location
  of `u.writes` meets one of `t.reads`.
- **Write enforcement.** Every step the model checker takes under
  partial-order reduction SHALL pass `check_frame` with its receiver
  (FR-154), as every FR-120 step does.
- **Read enforcement.** Under partial-order reduction, `ModelSystem` SHALL
  evaluate each `pre` and `post` clause of a transition over an observation
  that exposes only the transition's read footprint (and, for a `post`
  clause, its write footprint in the post-state). A read of any other
  location SHALL stop the expansion with `runtime_invariant`/
  `established-invariant-broken`, naming the transition and the location,
  and the item SHALL settle `failed` as an internal failure (ADR-013 O-16),
  never with a value.
- **Determinism.** Footprints SHALL be functions of the checked package and
  the transition identity.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-155-AC-1 | ADR-021 §7.2's `Counter` unit (`v: Int[0, 2]`, `bump()` framed `modifies self.v`, pre `self.v < 2`, post `self.v = pre(self.v) + 1`): the symbolic footprint of `bump` has reads `{(Receiver, v), Membership{counters, Receiver}}`, writes `{(Receiver, v)}`, and enabling equal to reads; `bump(a)` instantiates to `Field{a, v}` and `Membership{counters, a}`; `bump(a)` and `bump(b)` are independent and `bump(a)` with itself is not. | Test (TC-574) |
| FR-155-AC-2 | With `bump()` framed `modifies v` (every-object), its write footprint is `{AnyField{v}}` and `bump(a)`, `bump(b)` are dependent. An operation `copy()` framed `modifies self.v` with post `self.v = self.peer.v` reads `(Receiver, peer)` and `AnyField{v}` and is dependent with every `bump`; an operation whose pre is `forall o in counters: o.v < 2` reads `AnyField{v}` and `AnyMembership{counters}`. An operation `give(to: Counter)` framed `modifies self.v` with pre `to.v < 2` has an enabling footprint holding `(Param(to), v)`, `Membership{counters, Receiver}` and `Membership{counters, Param(to)}`, instantiating to `Field{b, v}`, `Membership{counters, a}` and `Membership{counters, b}` for `give(a, b)`, so `give(a, b)` is dependent with `bump(b)` and independent of `bump(c)`. An operation `spawn()` framed `creates Counter` writes `AnyMembership{counters}` and is dependent with every `bump`; an operation whose pre quantifies over `counters` reads `AnyMembership{counters}`. | Test (TC-574) |
| FR-155-AC-3 | Under partial-order reduction, evaluating `bump(a)`'s post clause over an observation restricted to a footprint with `Field{a, v}` removed stops the expansion with `runtime_invariant`/`established-invariant-broken` naming `bump(a)` and `Field{a, v}`, and the item settles `failed`, category internal failure; with the derived footprint it evaluates to a value. | Test (TC-575) |
| FR-155-AC-4 | Every step of AC-1's reduced run passes `check_frame` with its receiver; a hand-built successor of `bump(a)` that also changes `b.v` is `frame_violation`. Deriving footprints twice from the same package gives equal footprints. | Test (TC-575) |
| FR-155-AC-5 | ADR-021 §7.6's guard-in-postcondition subject: `boom`'s enabling footprint holds `(Receiver, armed)` and `AnyField{v}` from `pre(self.cell.v)`, so `set(a)` meets it. The deleting-the-receiver subject: `cancel()` deleting its receiver writes `Membership{jobs, Receiver}`, so `cancel(j)` is dependent with `boom(j)`, which reads `Membership{jobs, j}`. The type-wide-delete subject: `cancel()` framed `deletes Job` writes `AnyMembership{jobs}`, so `cancel(m)` is dependent with `boom(j)`. The blocked-delete subject: `cancel()` framed `deletes self` reads `AnyField{peer}`, which `unlink(j)` writes. The navigation subject: `boom`'s read footprint holds `(Receiver, cell)`, which `point()` writes. | Test (TC-589) |
| FR-155-AC-6 | ADR-021 §7.6's result subject: `pick(): Reference<Cell>` with post `self.alarm and result.v = 1` has an enabling footprint holding `AnyField{v}` and `AnyMembership{cells}`, so `set(a)` meets it, and the reduced run returns `Violated` by `set, pick` over 6 states, as the unreduced run does. | Test (TC-589) |

## Dependencies

- ADR-021 POR-1, POR-2, POR-4, EI-1 (c), EI-4.
- [FR-104](FR-104-check-state-clauses.md) (checked clause bodies),
  [FR-107](FR-107-evaluate-state-clauses-at-s6a.md) (the evaluator),
  [FR-120](FR-120-simulate-a-checked-package-s-state-family.md)
  (`ModelSystem`, observations), [FR-154](FR-154-scope-a-modifies-entry-to-the-receiver.md)
  (scoped frames and `check_frame`).
- ADR-027 FT-1 to FT-4 give the protocol locations and `ProtocolSystem`'s
  footprints.
- QSpec FR-383 owns locations, meets and footprint derivation as normative
  semantics (ADR-021 QS-4); QSpec FR-013 owns scoped `deletes`.

## References

- QSpec half: QSpec FR-383 (Linear STD-134; ADR-021 §9 QS-4). Protocol transition system:
  ADR-027, Linear QSL-396. Owning ticket: Linear QSL-368.
