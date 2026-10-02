---
id: FR-209
title: "Start a protocol instance per trigger under the max_live_instances bound"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-024
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-205
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-206
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-171
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-331
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-428
    type: depends_on
---
# FR-209: Start a protocol instance per trigger under the max_live_instances bound

## Description

For a protocol clause with `activation on each (x: T) when {g}`, the
protocol system SHALL start one protocol instance per `activate(T)` step as
QSpec FR-428 states, and under every activation kind SHALL remove an
instance in the step that finishes it (ADR-027 AE-1 to AE-3, ST-13).

The model checker SHALL limit the live instances of a run to the
`max_live_instances` method bound, a bound on the explored state space and
not a resource limit, which the request sets and whose published
default is 3 (ADR-027 AE-4). When a run that
reached the bound finds no counterexample, the model checker SHALL return
`Undecided(InstanceBoundReached)`, which FR-127 settles `inconclusive`, naming
`max_live_instances` as the request member that raises it. The model checker SHALL
state the bound used, and whether it was reached, in every verdict over an
`on each` protocol.

## Use case

A verification operator models a service that starts one handling protocol
per incoming request. Requests can keep arriving, so the number of live
instances has no natural end. The operator checks the protocol with up to
three concurrent instances by default, or raises the bound, and the result
says which bound it ran with and whether any state hit it, so that a pass
is never mistaken for a proof over every instance count.

## Inputs

- A protocol state (FR-205) and the clause's activation: trigger type `T`,
  guard `g` and captures.
- `ModelCheckRequest.max_live_instances: Option<u64>` (FR-126), a request
  member beside `max_depth`;
  `None` means the published default.

## Outputs

- `activate(T)` steps; instances with their ordinals in the key.
- `ModelCheckOutcome::Undecided(InstanceBoundReached { bound, states })`
  (FR-126), settled by FR-127 as V-6.
- In every FR-331 terminal record of an item over an `on each` protocol:
  the method's `max_live_instances` value and whether it was reached.

## Behavior

### Instances

- The protocol system SHALL give one `activate(T)` step per record of `T`
  that satisfies `g`, as an environment step independent of the model's
  operations, starting one instance with QSpec FR-428's instance ordinal,
  captures and settled root thread.
- Under every activation kind, the step that finishes an instance (its
  `finish` step, FR-206) SHALL remove it with its binders, registrations,
  role instances and queues.
- Under `on origin` the subject SHALL have one instance, ordinal 0, started
  in the initial state.

### The bound

- `max_live_instances` SHALL be a method bound with no ceiling, outside
  the subject and its obligation identity. A request that leaves it unset
  SHALL run with 3.
- At a state with `max_live_instances` live instances, the model checker
  SHALL leave each enabled `activate` step unexpanded and SHALL record the
  state as **instance-limited**. ADR-018 FA-2 enabledness SHALL read `activate` as
  enabled there, and an instance-limited state SHALL be neither terminal nor
  a deadlock.
- A run that finds a counterexample SHALL return `Violated`, whatever the
  bound; FR-127 settles it V-4.
- A run that completes with no counterexample and no instance-limited state
  SHALL return `Holds`, which FR-127 settles V-1.
- A run that completes with no counterexample and at least one
  instance-limited state SHALL return `Undecided(InstanceBoundReached{bound,
  states})`, which FR-127 settles V-6, `inconclusive`, with `bound` naming the request
  member that raises it, `max_live_instances`, and its value, and `states`
  the count of instance-limited states.
- When more than one under-approximation cause applies, the model checker
  SHALL return the first of `ConstraintReached` (ADR-021),
  `MemoryBoundReached` (ADR-025), `InstanceBoundReached`, `BoundReached`
  (the completed `max_depth` search horizon).
- Every terminal record of an item over an `on each` protocol, V-1 included,
  SHALL state in its method the `max_live_instances` value used and whether
  any state was instance-limited.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-209-AC-1 | An `on each (x: Tick) when {true}` protocol over a `Cell` (`v: Int[0, 1]`), whose body is one attempt `set` (postcondition `self.v = 1`, always enabled), and the TP-1 claim `always holds(k.v <= 1)`: with `max_live_instances` 1 the run settles V-6, `InstanceBoundReached{bound: max_live_instances 1, states}` with `states` at least 1, naming `max_live_instances` as the member that raises the bound; with 2 it settles V-6 with `bound: max_live_instances 2`; with the field unset the record states `max_live_instances` 3, reached. | Test (TC-654) |
| FR-209-AC-2 | The same protocol with guard `{false}`: no `activate` step exists, the run settles V-1, and its record states `max_live_instances` 3, not reached. With the guard back at `{true}`, the claim `always holds(k.v = 0)` and `max_live_instances` 1, the run settles V-4 with a counterexample `activate(Tick)`, `attempt(set)`. | Test (TC-654) |
| FR-209-AC-3 | With `max_live_instances` 2: after two `activate` steps the instances hold ordinals 0 and 1; once instance 0 finishes and is removed, the next `activate` creates ordinal 0. An instance-limited state is not reported as a deadlock by the deadlock-freedom item. | Test (TC-654) |
| FR-209-AC-4 | A run where both a memory bound (ADR-025) and the instance bound limited a state, and no counterexample was found, settles `MemoryBoundReached`; a run where the instance bound limited a state and `BoundReached` (the completed `max_depth` search horizon) also applies settles `InstanceBoundReached`. Two requests that differ only in `max_live_instances` carry equal obligation identities. | Test (TC-654) |

## Dependencies

- ADR-027 §2.2 ST-13, §2.2a AE-1 to AE-4, §3.2 PD-5; ADR-014 B-5 (run
  bounds); ADR-018 V-1, V-4, V-6.
- [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (`ModelCheckRequest`, outcomes),
  [FR-127](FR-127-settle-a-model-check-verdict-as-a-terminal-record.md)
  (the V-6 row and the terminal record), FR-205, FR-206.
- QSpec owns activation on each, its instance identity and the bound with
  its default, V-6 cause and terminal-record statement (ADR-027 QS-11):
  QSpec FR-171, FR-331.

## References

- Owning ticket: Linear QSL-396. QSpec half: QSpec FR-428 (Linear STD-140).
