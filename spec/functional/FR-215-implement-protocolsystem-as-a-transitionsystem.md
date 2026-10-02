---
id: FR-215
title: "Implement ProtocolSystem<M> as an FR-101 TransitionSystem"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-024
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-205
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-206
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-181
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-425
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-426
    type: depends_on
---
# FR-215: Implement ProtocolSystem<M> as an FR-101 TransitionSystem

## Description

Crate `qsl-eval`'s `simulation` module SHALL provide `ProtocolSystem<M: MemoryModel>`, beside
`ModelSystem`, implementing FR-101's `TransitionSystem` over a protocol
subject, with `initial` the subject's initial protocol states, `key` its
`ProtocolKey` and `successors` every enabled step of every live instance,
thread, registration, role, channel, environment and memory model (ADR-027
TS-1). Each step SHALL have QSpec FR-426's typed canonical transition identity
(ADR-027 TS-2), an attempt SHALL apply through the one FR-120 application function
`ModelSystem` uses (ADR-027 TS-3), and the protocol system SHALL reach
memory only through `MemoryModel`, whose identity is `Sc` (ADR-027 SE-1 to
SE-3). The `model_check` engine in crate `qsl-analyze` (ADR-029) SHALL check a protocol subject as FR-126
checks a model subject (ADR-027 TS-4 to TS-6).

## Use case

A verification operator runs the model checker over a protocol. The same
engine, product, fairness filter, limits and counterexample choice that
check a model check the protocol, so every verdict and limit means the same
thing, and an attempt means exactly what the operation's contract means in
`ModelSystem`.

## Inputs

- A `ProtocolSubject` (FR-205), the in-process checked package's checked
  protocol clause, the memory model `M` (`Sc` for a `parallel` with no
  memory clause) and the instance bound (FR-209).
- For `model_check`: FR-126's `ModelCheckRequest`, whose subject is a
  model subject or a protocol subject.

## Outputs

```rust
pub trait MemoryModel {
    type Component: Serialize;            // (a) member of the state key
    // (b) observe, (c) write, (d) internal steps with identity,
    // enabledness and effect, (e) gate on fork, join, send, receive and
    // attempt, (f) split and merge, (g) atom values, (h) fairness with a
    // `memory` origin
}

pub struct Sc;                            // the identity memory model

pub struct ProtocolSystem<'a, M: MemoryModel> { /* … */ }

impl<M: MemoryModel> TransitionSystem for ProtocolSystem<'_, M> {
    type State = ProtocolState<M>;
    type TransitionId = ProtocolTransition;
    type Key = ProtocolKey;
    type Finding = ModelFinding;
    // initial, key, successors
}
```

`ProtocolTransition` is owned by `quire-semantic-value` (SV), because the
compiler produces it and CG reads it in counterexamples; it serializes as
`{"type":"protocol-transition","step":<kind>,"thread":<path>,"node":<wire node id>,…}`.

## Behavior

### The system

- `initial` SHALL return FR-205's initial protocol states.
- `key` SHALL return FR-205's `ProtocolKey`.
- `successors` SHALL enumerate every step of FR-206 to FR-209 and every
  memory step enabled at the state, each with its successor state, in
  ascending JCS bytes of the transition identity.

### Transition identity

- A step's identity SHALL carry its kind, the owning thread's path and its
  wire node id, and the further members of QSpec FR-426's identity table:
  for `attempt` and `cattempt`, the operation's qualified name, the
  receiver reference and the argument vector; for `cattempt` and `cend`,
  the template's qualified name and the registration key; for `send`,
  `receive`, `event`, `finish` and `activate`, the bound record; for a step
  `by` a replicated role, the acting instance's reference; for `spawn` and
  `retire`, the role and object reference; for `duplicate` and `lose`, the
  channel's qualified name with its protocol instance ordinal, and the
  entry; for a `fence` and an access with an ordering, the ordering; for an
  access whose memory model chooses a message or slot, that choice; for a
  memory step, the form its model gives.
- Successors of one identity SHALL be told apart by their post-state's
  state-key digest, as ADR-018 CX-3 tells FR-120's apart.

### One application rule

- An `attempt` or `cattempt` SHALL call the same function `ModelSystem`
  uses to expand one application (effective precondition, frame
  post-states, postconditions, `check_frame`'s delta), passing the clauses
  its attempt selects and the observation the memory model gives. The
  protocol system SHALL have no second contract evaluator.
- Each expanded state SHALL carry FR-120's invariant findings for its model
  state. An attempt whose contract decision is refused or incomplete SHALL
  carry `ContractUndetermined`, and a model check that meets it SHALL return
  `Undecided(UndecidedSuccessor)`, which FR-127 settles as V-6. When a
  contract evaluates undefined, the model check SHALL return a violation
  with cause `UndefinedEvaluation{where, cause}` (QSpec FR-426), which
  FR-127 settles as refuted.

### The memory-model seam

- `Sc` SHALL have an empty component, observe and atom values that read the
  model state, a write that applies in place, no internal steps, gates that
  are true, a split and merge that do nothing, and no fairness.
- A memory step SHALL be a position, uncounted (FR-213), owned by no thread,
  moving no rest point, with the footprint its model gives (FR-216).

### Model checking a protocol subject

- Before any expansion, `model_check` SHALL classify every root of the
  subject: FR-120's `domains()` under the universes, the record type of
  every binder of `event`, `send`, `receive`, `finish`, `cattempt` and
  `activate` steps, and each replicated role's population universe. An
  unbounded root SHALL return FR-126's `RequiresBound`, and no state is
  explored.
- `model_check` SHALL explore the product of `ProtocolSystem` with the
  property automaton, keyed by (protocol state key, automaton state index),
  on FR-101's engine, with FR-126's edge retention, SCC phase, fairness
  filter over FR-212's classes, limits and canonical counterexample.
- A `ModelCheckRequest` whose subject is a protocol subject SHALL be
  answered with the same `ModelCheckOutcome` variants as for a model
  subject, plus FR-209's `InstanceBoundReached`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-215-AC-1 | `ProtocolSystem<Sc>` over ADR-027 §7's `Fill` explored by FR-101's engine reaches seven states and six transitions; at s1 `successors` returns `attempt(A)` then `attempt(B)` in ascending JCS bytes of their identities, and each identity serializes with `"type":"protocol-transition"`, its step kind, thread path, node id, operation name and argument vector. | Test (TC-660) |
| FR-215-AC-2 | For the attempt `bump()` of FR-206-AC-2, the successor model states and FR-120 transition identities `ProtocolSystem` gives are equal to those `ModelSystem` gives for the same application. A variant whose postcondition is undecided by the evaluator yields `ContractUndetermined`, and `model_check` settles V-6 `UndecidedSuccessor`. | Test (TC-660) |
| FR-215-AC-3 | `Sc`'s component serializes as an empty member of every key, `Sc` gives no internal step, and every gate is true: the states of `Fill` under `ProtocolSystem<Sc>` equal those of a test memory model that implements `Sc`'s eight members independently. | Test (TC-660) |
| FR-215-AC-4 | A protocol with an `event` node whose record type has an unbounded `Int` field returns `RequiresBound` naming that binder before any state is explored. `model_check` over the `Fill` deadlock-freedom item returns `Violated` with `kind: Deadlock`, and over the repaired `Fill` returns `Holds{Exhaustive}` over ten states; running each twice gives equal outcomes and byte-equal counterexamples. | Test (TC-660) |

## Dependencies

- ADR-027 §2.5 SE-1 to SE-4, §4 TS-1 to TS-6; ADR-011 §6.2 (`qsl-eval`'s
  `simulation`, as amended by ADR-027); ADR-029 (`model_check` in
  `qsl-analyze`).
- [FR-101](FR-101-explore-finite-models-with-canonical-order-and-the-qspec-sampler.md)
  (`TransitionSystem`, engine, canonical order),
  [FR-120](FR-120-simulate-a-checked-package-s-state-family.md) (the
  application function), [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (product, phases, limits), FR-205, FR-206, FR-209, FR-212, FR-213,
  FR-216.
- QSpec owns the protocol transition identity's typed canonical form
  (ADR-027 QS-4): QSpec FR-426, over FR-181.

## References

- Owning ticket: Linear QSL-396. QSpec half: QSpec FR-425, FR-426 (Linear STD-140).
