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
- A static role on a model alias, once FR-218's "Role on a model alias"
  admits one, SHALL be bound to the model subject by construction; the
  request SHALL NOT bind it. Its `roles` entry in the state key SHALL hold
  the null literal `{"type":"null"}` (QSpec FR-181) as its binding, since it
  binds no object. When the request supplies a binding for such a role, the
  subject builder SHALL refuse `invalid_runtime_input`/`wrong-role-mapping`,
  naming the role.
- When a static role on any other model has no binding, or a binding names a
  model the role does not admit, the subject builder SHALL refuse
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
  under `Sc`. The key SHALL also hold, as FR-425 states: for each
  compensation thread, the registration it compensates; for each protocol
  instance in which a step of a template's commit node has occurred, that
  node; and, for each compensation thread, its attempt record in a state if
  and only if the thread rests at its retry point, its attempt count is below
  the template's `attempts n`, the template's retry relation reads its
  `earlier` binder, and no step of the template's commit node has occurred in
  its instance (ADR-027 PS-8, ST-11).
- Thread, instance, registration and nesting counts SHALL have no ceiling
  of their own; the run's method bounds and resource limits (FR-209, FR-215) are
  the only bounds.
- The subject SHALL have QSpec FR-425's initial protocol states: under
  `on origin`, instance 0 settled from the start of `run` (FR-206); under
  `on each`, no instance (FR-209).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-205-AC-1 | ADR-027 §7's `Fill` subject (universe `{c}`, `c.v = 0`, `k` bound to `c`, and `w`, a role on the model alias `M`, bound to the model subject by construction with no request binding) builds. Two subjects that differ only in the `over` binding, or only in the clause's `terminal` member, have different obligation identities. Naming a protocol absent from the package refuses `invalid_runtime_input`/`invalid-value`; supplying a request binding for `w` refuses `invalid_runtime_input`/`wrong-role-mapping` naming `w`; in a variant that adds `role v on M::Cell`, leaving `v` unbound refuses `invalid_runtime_input`/`wrong-role-mapping` naming `v`; binding `k` to an object outside the universe refuses `invalid_runtime_input`/`invalid-value`. | Test (TC-650) |
| FR-205-AC-2 | The `Fill` subject has one initial state, whose key holds the root at `fork Both`, empty `observations`, `queues` and `bounds`, and `roles` holding `w`'s entry with binding `{"type":"null"}` (bound to the model subject by construction). After `fork(Both)` the key's `control` holds paths `(0)` at `join Both`, `(0, Both, left, 0)` at `A` running and `(0, Both, right, 0)` at `B` running; its maps are sorted by JCS bytes, and serializing the key twice gives equal bytes. Every reachable state of `Fill` has empty `observations`, because no node reads `a` or `b`; in a variant where a node after the join reads `a`, `observations` holds `a` in every state from `attempt(A)` to that node's step, under `(0, Both, left, 0)` until the join and under `(0)` after it, and in none after the node's step. In a variant where `left` is `sequence { attempt A; attempt A2 }`, `a` is held from `attempt(A)` to the join, and then by the root to the reading node. In a variant where `left` is `sequence { parallel R { branch x attempt X binding b } join all; attempt L2 }` and a node after `Both`'s join reads `b`, `x` keeps `b` through `join(R)`, `left` through `join(Both)`, and the root holds it until that node's step. | Test (TC-650) |
| FR-205-AC-3 | A `repeat` with maximum 2 around `parallel P { branch a attempt X …; branch b attempt Y …; } join any [a, b] outstanding continue`, where `X` and `Y` are always enabled: the second fork while instance 0's `b` is still `running` creates threads with ordinal 1; when `b` of instance 0 completes before the second fork, the second fork creates ordinal 0 again, so its threads' paths equal the first fork's. Two behaviours that reach the second fork's post-state with equal model state and counter, one through each completion order of the first instance's branches, reach one state. With `branch b sequence { parallel Q { branch x attempt X2; branch y attempt Y2 } join any [x, y] outstanding continue }` in place of `b`'s attempt, when instance 0's join has settled, `b` has completed and `y` of `Q` is still `running`, the second fork takes ordinal 1, because `y`'s path extends instance 0's branch path, and it takes ordinal 0 once `y` completes. With `maximum 3` and instances 0 and 1 both live, when instance 0 ends and is removed while instance 1 still runs, the third fork takes ordinal 0, the smallest ordinal no live sibling holds, and not 2. | Test (TC-650) |
| FR-205-AC-4 | A `Fill` subject with two initial snapshots (`c.v = 0` and `c.v = 1`) has two initial protocol states, each with the root at `fork Both`. An `on each` protocol's initial protocol state holds no instance. | Test (TC-650) |
| FR-205-AC-5 | A template `Refund` with two registrations, whose first activates and whose compensation thread `(0, Refund, 0)` is still live when the second activates: the second thread takes ordinal 1. When `cend` has removed `(0, Refund, 0)` before the second activates, the second thread takes ordinal 0. | Test (TC-650) |
| FR-205-AC-6 | Two protocol definitions in one package, each declaring `channel Pay`, give two subjects whose state keys hold `queues` entries, and whose `duplicate` and `lose` identities hold channel names, each in its own protocol's qualified name; no key or identity of one names the other's channel. | Test (TC-650) |
| FR-205-AC-7 | Inside a `repeat` of maximum 2: `parallel P { branch left attempt A1 binding a; branch right attempt B1 } join all`, then `parallel Q { branch u sequence { attempt U1; check Ck { a.ready } }; branch v attempt V1 } join any [u, v] outstanding continue`, where `A1` has two successors, binding `a` with `ready` true or false. After iteration 1's `join(P)` with `ready` true, the forking thread's binder environment holds `a` with `ready` true, and `fork(Q)` copies it into `u` and `v`. In the state where `V1` has been taken, the `join(Q)` step has run and `u` still rests at `U1`, the second fork of `P` takes ordinal 0, because `P`'s first instance is removed, while `Q`'s instance 0 is still live because `u` is `running`. In iteration 2 with `ready` false, `fork(Q)` takes ordinal 1 and gives its `u` an environment holding `a` with `ready` false: iteration 1's `u` still evaluates `Ck` over its own `a` (true) and iteration 2's `u` over its own (false, so it rests at `Ck`), and no state refuses. When `u` of iteration 1 completes, its entry is dropped, and the key's `observations` holds no `a` under its path. | Test (TC-650) |
| FR-205-AC-8 | Template `Refund` with `attempts 1` and `commit Committed`, whose forward `effect` node is reached twice in a `repeat` of maximum 2, giving registrations `r1` and `r2` that differ in a registration capture `id`, whose activation guard compares `id` with the `id` of the trigger record, and a `parallel Triggers` of two sibling branches each binding a trigger record with one of the two ids, followed by `commit Committed`, each compensation thread taking its `cattempt` and `cend` before the step of `Committed`. The state after `r1`'s trigger and then `r2`'s and the state after `r2`'s and then `r1`'s have different keys, because each compensation thread's entry names its registration: thread `(0, Refund, 0)` names `r1` in the first and `r2` in the second. The state after the step of `Committed` holds that node under its instance and the state before the step does not, so the two have different keys, and the entry is absent once the instance finishes. In a variant with `attempts 2 of M::RefundAttempt`, `RefundAttempt` with one field `ok: Boolean`, a template operation whose precondition is `true`, and the retry relation `not e.ok` over (`e`, `l`): after thread `(0, Refund, 0)`'s first `cattempt` with record `ok` false, `observations` holds that thread's attempt record with `ok` false and a retry `cattempt` is enabled; after its first `cattempt` with `ok` true, `observations` holds the attempt record with `ok` true and only `cend` is enabled, so the two post-states have different keys. The attempt record is absent after the thread's second `cattempt`, whose count reaches 2, after its `cend`, and in the post-state of the step of `Committed` taken while the thread rests at its retry point holding one. With the retry relation `l.ok`, which does not read `e`, no state holds an attempt record, and the two first `cattempt` steps, `ok` true and `ok` false, reach one state. | Test (TC-650) |

## Dependencies

- ADR-027 §1 PS-1 to PS-8 (PS-2 ordinals, PS-7 channel names), §2.1 FO-3, §2.2a AE-2.
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
