---
id: FR-230
title: "Check a state model's time declaration, clocks, clock constraints, time invariants and urgency"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-026
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-124
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-142
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-205
    type: depends_on
---
# FR-230: Check a state model's time declaration, clocks, clock constraints, time invariants and urgency

## Description

QSL's S3 checker SHALL admit the timed members of a state model (ADR-026
CK-1 to CK-9): one `time` member naming the model's time source, `Clock`
fields under a dense source, clock resets in postconditions, atomic clock
constraints in guards, time invariants and urgency declarations. It records
them on the checked state model so that the timed subject (FR-231), the zone
engine (FR-239) and replay (FR-237) read one checked form. S3 confines every
clock read to an atomic clock constraint, so no real value reaches the value
kernel.

## Use case

A verification operator models a client that times out after 3 ms and a
server that replies between 1 ms and 3 ms after a request. They declare
`time dense unit ms`, give the call object a clock, reset it on `send`,
guard `reply` with `x >= 1 ms` and bound waiting with time invariants. A
mistake such as copying the clock into an integer field, or comparing two
clocks, is refused at check time with a span.

## Inputs

- A parsed state model with its members, in the spelling QSpec's shared
  grammar fixes for the `time` member, `Clock` fields, resets, clock
  constraints, `time invariant` and `urgent` (QSpec half, References).

## Outputs

On the checked state model:

```rust
pub enum TimeSource {
    Dense { unit: TimeUnit },                                   // QSpec FR-142
    Tick { operation: OperationId, period: ExactQuantity, unit: TimeUnit },
}

pub struct TimedModel {
    pub source: TimeSource,
    pub clocks: Vec<ClockField>,            // (object type, field), declaration order
    pub invariants: Vec<TimeInvariant>,     // when P { x <= c, ... }
    pub urgency: Vec<Urgency>,              // When(P) | Operation(OperationId)
}

pub struct ClockConstraint { pub clock: ClockRef, pub op: ClockOp, pub bound: ExactRational }
pub enum ClockOp { Lt, Le, Eq, Ge, Gt }
```

`CheckedStateModel::time: Option<TimedModel>`, each operation's checked
guard as data predicates and atomic clock constraints, and each operation's
resets as `(ClockRef, ExactRational)` pairs. A typed `CheckRefusal` with a
span on refusal.

## Behavior

### The `time` member

- The checker SHALL admit at most one `time` member per state model; a
  second SHALL refuse `invalid_timed_model`/`duplicate-time-source` at
  its span, naming the first (QSpec FR-415).
- The checker SHALL resolve the unit `u` as a unit of QSpec FR-142's time
  dimension, and SHALL refuse a unit of another dimension with
  `ill_typed`/`type-mismatch` at the unit's span.
- Under `time tick O period p unit u`, the checker SHALL resolve `O` to an
  operation of the model (FR-103), refusing `missing_declaration`/
  `missing-name` otherwise, and SHALL require `p` to be a positive exact
  rational quantity of the time dimension (QSpec FR-205), refusing zero or a
  negative period with `invalid_model_binding`/`malformed-declaration`.
- A model with a `time` member is a timed model. The `TimedModel` SHALL be
  part of the checked package, so two models that differ only in it have
  different package identities.

### Clocks and resets

- The checker SHALL admit a field of type `Clock` only in a model whose
  source is `Dense`, and SHALL refuse it elsewhere with
  `invalid_timed_model`/`clock-without-dense-time` at the field's span.
- A postcondition conjunct `self.x = c` on a clock `x` of the operation's
  frame, with `c` an exact non-negative rational time quantity, SHALL check
  as a reset to `c` converted exactly to `u`. Any other write to a clock
  SHALL refuse `invalid_timed_model`/`clock-write`.
- A clock in the frame that the operation does not reset keeps its value;
  the checker records no write for it.

### Clock constraints and guards

- The checker SHALL admit `x ~ c`, with `~` one of `<`, `<=`, `=`, `>=`,
  `>` and `c` an exact non-negative rational time quantity, as an atomic
  clock constraint, converting `c` exactly to `u` (QSpec FR-142, FR-205).
  A negative constant SHALL refuse `invalid_timed_model`/
  `negative-clock-constant` at the constant's span.
- The checker SHALL admit a clock read only as the clock operand of an
  atomic clock constraint. A clock compared with another clock SHALL refuse
  `invalid_timed_model`/`clock-comparison`, and a clock read anywhere else,
  including a clock in arithmetic or a clock assigned to a data field,
  SHALL refuse `invalid_timed_model`/`clock-read-outside-constraint`, each
  at the read's span.
- An operation's precondition SHALL check as a Boolean combination of data
  predicates and atomic clock constraints. The checker SHALL record the
  guard in disjunctive form over convex clock parts, each part a conjunction
  of atomic clock constraints with its data predicate.

### Time invariants and urgency

- `time invariant when P { … }` SHALL check `P` as a state predicate
  through the clause checker invariants use (FR-104), and its body as a
  conjunction of atomic clock constraints whose operator is `<` or `<=`.
  A lower bound or an equality in the body SHALL refuse
  `invalid_timed_model`/`time-invariant-shape`.
- `urgent when P` SHALL check `P` as a state predicate (FR-104).
- `urgent O` SHALL resolve `O` to an operation of the model, and SHALL
  refuse an `O` whose guard reads a clock with `invalid_timed_model`/
  `urgent-guard-reads-clock` at the declaration's span, naming the clock
  read.
- A time invariant or urgency declaration in a model with no `time` member
  SHALL refuse `missing_declaration`/`missing-name`, naming the missing
  `time` member.

### Limits

- The checker SHALL accept any number of clocks, any constant and any
  period that the exact number types represent. The work of checking timed
  members is charged to the existing S3 work meter.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-230-AC-1 | ADR-026 §11's `Rpc` unit checks to `TimeSource::Dense{ms}` with one clock field `Call.x`, two time invariants (`inflight` with `x <= 3`, `Waiting` with `x <= T`), `send`'s reset of `x` to 0, and `reply`'s guard with the one convex part `x >= 1`. The same unit with `time dense unit s` checks with `reply`'s bound converted to `1/1000`, and its package identity differs from the `ms` unit's. | Test (TC-685) |
| FR-230-AC-2 | Refusals, each at its span: a second `time` member (`invalid_timed_model`/`duplicate-time-source`); `time dense unit m` (`ill_typed`/`type-mismatch`); `self.n = self.x` with `n: Int` (`invalid_timed_model`/`clock-read-outside-constraint`); `x <= self.y` with both clocks (`invalid_timed_model`/`clock-comparison`); `x >= -1 ms` (`invalid_timed_model`/`negative-clock-constant`); a `Clock` field under `time tick` (`invalid_timed_model`/`clock-without-dense-time`); `self.x = self.x + 1` (`invalid_timed_model`/`clock-write`); a time invariant body `x >= 1` (`invalid_timed_model`/`time-invariant-shape`); `urgent reply` where `reply` guards on `x` (`invalid_timed_model`/`urgent-guard-reads-clock`); `urgent when P` in an untimed model (`missing_declaration`/`missing-name`). | Test (TC-685) |
| FR-230-AC-3 | `time tick tick period 1/2 ms unit ms` checks to `TimeSource::Tick` with period `1/2`; `period 0 ms` refuses `invalid_model_binding`/`malformed-declaration`; a tick operation name that resolves to nothing refuses `missing_declaration`/`missing-name`. | Test (TC-685) |
| FR-230-AC-4 | A guard `(x < 1 or x > 2) and self.ready` checks to two convex parts, `x < 1` and `x > 2`, each with the data predicate `self.ready`. A model with 64 clock fields over a universe of 100 objects and a constant of `10^40 ms` checks with no refusal. | Test (TC-685) |

## Dependencies

- ADR-026 §2 CK-1 to CK-9.
- [FR-103](FR-103-admit-model-operations-and-frames-on-the-spine.md)
  (operations and frames), [FR-104](FR-104-check-state-clauses.md) (state
  predicates), [FR-124](FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md)
  (the sibling `terminal` member).
- QSpec FR-142 (the time dimension and exact unit conversion) and QSpec
  FR-205 (exact quantities).

## References

- QSpec half: QSpec FR-415 (Linear STD-139) owns the shared grammar of the
  `time` member, `Clock` fields, resets, clock constraints, time invariants
  and `urgent` (ADR-026 OV-7).
