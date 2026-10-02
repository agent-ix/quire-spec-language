---
id: FR-205
title: "Define the protocol subject and its canonical state key"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-024
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-161
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-181
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-425
    type: depends_on
---
# FR-205: Define the protocol subject and its canonical state key

## Description

QSL SHALL build a temporal claim's **protocol subject**, a model subject
(FR-125) together with one checked protocol clause of its package and the
clause's bindings, as QSpec FR-425 defines it, and SHALL key each protocol
state by QSpec FR-425's state key, so that two states with equal keys are
one state (ADR-027 PS-1 to PS-8, FO-3, AE-2).

## Use case

A verification operator asks whether a claim holds over a protocol that
forks two branches over one cell. They name the model subject, the
protocol, the object its `over` parameter binds and the model each static
role plays. Every engine and replay reads one definition of what a protocol
state is, so a counterexample's state digests mean the same thing to the
model checker and to replay, and a loop that forks again reaches a state it
has already seen instead of a new one.

## Inputs

- `ProtocolSubject` (`qsl_analyze::model_check`, ADR-029): a `ModelSubject` (FR-125);
  `protocol`, the declared identity of one protocol clause of the subject's
  checked package; `over`, the object reference bound to the clause's `over`
  parameter; `roles`, a binding for each static role; and the resolved
  memory model of each outermost `parallel` (FR-219, ADR-027 SE-4): the
  request's selection when present, else the source default, `Sc` when the
  clause declares none. Each replicated role's population universe is the subject's
  universe for that population (ADR-027 RR-1).
- The checked protocol clause S3 produces (FR-218).

## Outputs

- `ProtocolSubject` and, for a protocol state, its `ProtocolKey`: QSpec
  FR-425's form of FR-181's `{"type":"simulation-state","semantic":…,"control":…,"queues":…,"roles":…,"observations":…,"bounds":…}`
  with `queues` and `roles` keyed by protocol instance ordinal.
- The initial protocol states of the subject.

## Behavior

### Subject

- The subject builder SHALL build `ProtocolSubject` from the model subject,
  the protocol clause, the `over` binding, the static role bindings, the
  replicated role universes and the resolved memory models, as QSpec FR-425
  states. The protocol clause's `terminal` member (FR-211) and its
  `scheduling` member (FR-212) are part of the checked package, so the
  subject's obligation identity (ADR-013 O-09) binds them. The identity
  binds the resolved memory model of each outermost `parallel` (FR-219),
  and the source `memory` clause only through it.
- When the request names a protocol that is not a protocol clause of the
  package, the subject builder SHALL refuse `invalid_runtime_input`/
  `invalid-value`, naming the protocol.
- When a static role has no binding, or a binding names a model the role
  does not admit, the subject builder SHALL refuse
  `invalid_runtime_input`/`wrong-role-mapping`, naming the role.
- When the `over` object is not an object of its declared type's population
  universe, the subject builder SHALL refuse `invalid_runtime_input`/
  `invalid-value`, naming the parameter and the object.

### State and key

- `ProtocolSystem` (FR-215) SHALL hold each protocol state as QSpec FR-425
  states it, with threads, paths and ordinals, parallel instances, binders,
  iteration counters, queues, role instances and the memory component, and
  SHALL remove parallel instances, binders and finished protocol instances
  in the steps QSpec FR-425 and ADR-027 PS-4, PS-5 and AE-3 name.
- `ProtocolKey` SHALL be QSpec FR-425's state key, with `queues` and
  `roles` keyed by protocol instance ordinal, each map sorted by JCS bytes,
  and the memory component as the member the memory model supplies, empty
  under `Sc`.
- Thread, instance, registration and nesting counts SHALL have no ceiling
  of their own; the run's method bounds and resource limits (FR-209, FR-215) are
  the only bounds.
- The subject SHALL have QSpec FR-425's initial protocol states: under
  `on origin`, instance 0 settled from the start of `run` (FR-206); under
  `on each`, no instance (FR-209).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-205-AC-1 | ADR-027 §7's `Fill` subject (universe `{c}`, `c.v = 0`, `k` bound to `c`, `w` bound to the model) builds. Two subjects that differ only in the `over` binding, or only in the clause's `terminal` member, have different obligation identities. Naming a protocol absent from the package refuses `invalid_runtime_input`/`invalid-value`; leaving `w` unbound refuses `invalid_runtime_input`/`wrong-role-mapping`; binding `k` to an object outside the universe refuses `invalid_runtime_input`/`invalid-value`. | Test (TC-650) |
| FR-205-AC-2 | The `Fill` subject has one initial state, whose key holds the root at `fork Both`, empty `observations`, `queues` and `bounds`, and `roles` binding `w`. After `fork(Both)` the key's `control` holds paths `(0)` at `join Both`, `(0, Both, left, 0)` at `A` running and `(0, Both, right, 0)` at `B` running; its maps are sorted by JCS bytes, and serializing the key twice gives equal bytes. Every reachable state of `Fill` has empty `observations`, because no node reads `a` or `b`; in a variant where a node after the join reads `a`, the binder keyed `(A, (0, Both, left, 0))` is in every state from `attempt(A)` to that node's step and in none after it. | Test (TC-650) |
| FR-205-AC-3 | A `repeat` with maximum 2 around `parallel P { branch a attempt X …; branch b attempt Y …; } join any [a, b] outstanding continue`, where `X` and `Y` are always enabled: the second fork while instance 0's `b` is still `running` creates threads with ordinal 1; when `b` of instance 0 completes before the second fork, the second fork creates ordinal 0 again, so its threads' paths equal the first fork's. Two behaviours that reach the second fork's post-state with equal model state and counter, one through each completion order of the first instance's branches, reach one state. | Test (TC-650) |
| FR-205-AC-4 | A `Fill` subject with two initial snapshots (`c.v = 0` and `c.v = 1`) has two initial protocol states, each with the root at `fork Both`. An `on each` protocol's initial protocol state holds no instance. | Test (TC-650) |

## Dependencies

- ADR-027 §1 PS-1 to PS-8, §2.1 FO-3, §2.2a AE-2.
- [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md)
  (model subject), [FR-120](FR-120-simulate-a-checked-package-s-state-family.md)
  (`semantic` member and the state-key form),
  [FR-101](FR-101-explore-finite-models-with-canonical-order-and-the-qspec-sampler.md)
  (state keys and coalescing), FR-218 (the checked protocol clause and
  scopes).
- QSpec owns the protocol subject and the content of the state key's
  members (ADR-027 QS-1, QS-3); QSL builds them.

## References

- Owning ticket: Linear QSL-396. QSpec half: QSpec FR-425 (Linear STD-140).
