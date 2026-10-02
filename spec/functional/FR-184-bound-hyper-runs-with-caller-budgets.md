---
id: FR-184
title: "Bound hyper runs with caller-set budgets: max_witness_set and max_relation_tuples"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-021
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-023
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
---
# FR-184: Bound hyper runs with caller-set budgets: max_witness_set and max_relation_tuples

## Description

`ModelCheckLimits` SHALL gain two budgets for hyper items (ADR-023 HC-10):
`max_witness_set`, the members one HP-3 witness set may reach, and
`max_relation_tuples`, the tuples one HP-1 run may evaluate. Like every
EN-1 limit they are ADR-014 B-5 budgets of QSL's own provider that the
request sets, each with a published default that applies only when the
request sets none. They are resource budgets, never modelling limits: no
form, formula size, arity or depth is capped. Reaching one stops the run
and settles V-7, naming the limit, its value and the request member that
raises it.

## Use case

A verification operator runs a `∀∃` check whose witness sets grow large.
The run stops at the default budget and says which budget it reached, its
value, and that `max_witness_set` raises it. The operator raises it and
reruns, and the run completes.

## Inputs

```rust
pub struct ModelCheckLimits {
    // FR-126's members, plus:
    pub max_witness_set: u64,       // default 65_536 (2^16)
    pub max_relation_tuples: u64,   // default 16_777_216 (2^24)
}

pub enum ModelCheckLimit {
    // FR-126's members, plus:
    MaxWitnessSet,
    MaxRelationTuples,
}
```

## Outputs

- `Stopped(ResourceExhausted, MaxWitnessSet)` or
  `Stopped(ResourceExhausted, MaxRelationTuples)`, each with the limit's
  value and the count reached.

## Behavior

- A request that sets neither member SHALL run under the published
  defaults: `max_witness_set` 65,536 and `max_relation_tuples` 16,777,216.
- The engine SHALL count against `max_witness_set` the members of one HP-3
  witness set, during the check (FR-177) and during replay (FR-183).
- When an `X` would exceed `max_witness_set`, the engine SHALL stop the run.
- The engine SHALL count against `max_relation_tuples` the tuples an HP-1
  run has evaluated (FR-179).
- When the next tuple would pass `max_relation_tuples`, the engine SHALL
  stop the run.
- The engine SHALL count each budget with checked arithmetic.
- The engine SHALL never continue a run past a reached budget.
- For hyper items, `max_states` SHALL count product states,
  `max_transitions` joint steps or moves, and `max_automaton_states`
  automaton states, as FR-176 and FR-178 state.
- Every terminal record SHALL state the value of `max_witness_set` and
  `max_relation_tuples` the run used, and whether the request set it or the
  published default applied, whatever the verdict (FR-182).
- A stopped record SHALL name the limit, its value and the request member
  that raises it, and SHALL carry no counterexample.
- `max_depth` SHALL stay a method parameter that settles V-5, never V-7.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-184-AC-1 | A request that sets neither member runs with `max_witness_set` 65,536 and `max_relation_tuples` 16,777,216, and its terminal record reports those values, marked as published defaults, for every run: a proved run, a refuted run and a stopped run alike. | Test (TC-609) |
| FR-184-AC-2 | ADR-023 §8.2's leaky `Opaque` with `max_witness_set` 0 stops `Stopped(ResourceExhausted, MaxWitnessSet)` with value 0, count 1 and no counterexample; with `max_witness_set` 1 it completes with FR-177-AC-1's outcome. | Test (TC-609) |
| FR-184-AC-3 | FR-179-AC-1's `Det` with `max_relation_tuples` 63 stops `Stopped(ResourceExhausted, MaxRelationTuples)` with value 63 and count 63; with 64 it completes `Holds`. | Test (TC-609) |

## Dependencies

- ADR-023 §4 HC-10; ADR-014 B-5 as amended by ADR-018; ADR-018 IV-6.
- [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (`ModelCheckLimits`, `ModelCheckLimit`).
- FR-177, FR-179 and FR-183 count the budgets; FR-182 settles them V-7.
- QSpec owns the two budgets with their defaults and their V-7 settlement
  (ADR-023 QS-5).

## References

- ADR-023. QSpec half: QSpec FR-399 (Linear STD-136; ADR-023 QS-5).
