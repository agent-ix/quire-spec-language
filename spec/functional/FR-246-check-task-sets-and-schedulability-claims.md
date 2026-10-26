---
id: FR-246
title: "Check task sets and schedulability claims"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-026
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-142
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-290
    type: depends_on
---
# FR-246: Check task sets and schedulability claims

## Description

QSL's S3 checker SHALL admit a package's task sets for one preemptive
uniprocessor and the schedulability and response claims over them, and
route each claim to the closed-form engine EN-7, module `schedulability`
in layer A, crate `qsl-analyze`,
under the QSpec FR-290 capability kind `schedulability` (ADR-026 RT-1 to
RT-3). WCETs, blocking terms and jitter are stated premises of the claim:
they are part of the task set, so part of the subject and its obligation
identity.

## Use case

An embedded engineer declares three periodic control tasks with periods,
deadlines, WCETs and priorities, and asks whether the set is schedulable
under fixed priority. The verdict holds for exactly the WCETs they stated;
changing one gives a different obligation.

## Inputs

- A parsed `taskset S { task τ … }` declaration and `schedulable N on S
  under P;` and `response N on S.τ <= d under fixed_priority;` claims, `N`
  the claim's name, `S` the task set and `P` one of `fixed_priority`, `edf`
  and `amc`, in the spelling QSpec's shared grammar fixes. The grammar admits
  a task set with no task, a negative time value and a `lo` task with
  `wcet_hi`, so that S3 refuses them below.

## Outputs

```rust
pub struct TaskSet { pub name: QualifiedName, pub unit: Option<UnitId>, pub tasks: Vec<Task> }  // unit: below
pub struct Task {
    pub name: Name,
    pub arrival: Arrival,                   // Periodic { period } | Sporadic { min_interarrival }
    pub deadline: ExactRational,
    pub wcet: Wcet,                         // Single(c) | Dual { lo, hi }
    pub priority: Option<u64>,
    pub criticality: Criticality,           // Lo | Hi
    pub jitter: ExactRational,              // default 0
    pub blocking: ExactRational,            // default 0
}
pub enum SchedulingPolicy { FixedPriority, Edf, Amc }
pub enum ScheduleClaim { Schedulable(SchedulingPolicy), Response { task: Name, bound: ExactRational } }
```

## Behavior

- Every time value SHALL be an exact rational time quantity (QSpec FR-142,
  FR-230's time units); a value of another dimension SHALL refuse
  `ill_typed`/`type-mismatch` at its span.
- **The task set's unit.** The checker SHALL order the tasks by task name in
  Unicode scalar value order (the order FR-482 writes them), then take the
  unit of the first task's period or minimum inter-arrival time as the task
  set's unit. It SHALL convert every time value of the task set, and the
  bound `d` of every `response` claim over it, exactly into that unit (QSpec
  FR-142). Reordering task declarations SHALL NOT change the chosen unit or
  any checked value. EN-7's
  evidence (FR-247) and the settled record (FR-248) state their values in
  that unit, and the checked-package node holds them in it (FR-454). A task
  set written entirely in `ms` keeps `ms`, so `Ctl`'s fixpoints are
  `(1, 3, 12)`; the same task set with its first value written `4000 us`
  states them in `us`. A task set that declares no task has no time
  quantity and so no unit (`unit` is `None`); it is refused, and a
  `response` claim over it keeps its bound in the bound's own unit while it
  is checked.
- The checker SHALL refuse, each with `invalid_timed_model`/
  `invalid-task-set` at the span, naming the task (the task set when it
  declares none): a task set that declares
  no task; a period or minimum inter-arrival time that is not positive; a
  deadline that is not positive; a WCET that is not positive; a jitter or
  blocking term that is negative; a `HI` task whose `C(LO)` exceeds `C(HI)`;
  a `LO` task with dual budgets. QSpec FR-419 states the code and every
  one of these conditions, the claim-dependent ones below included, so that
  one invalid task set refuses under one code.
- Two tasks with one name SHALL refuse `conflicting_declaration`/
  `duplicate-declaration`.
- When a `fixed_priority`, `amc` or `response` claim is over a task set where
  two tasks share a priority, the checker SHALL refuse `invalid_timed_model`/
  `invalid-task-set`, naming both tasks; when a task has no priority, the
  checker SHALL refuse the same way, naming that task. For an `edf` claim the
  checker SHALL ignore priorities (QSpec FR-419).
- When an `edf` claim is over a task with a jitter or blocking term other than
  zero, or an `amc` claim is over a task whose deadline exceeds its period,
  the checker SHALL refuse `invalid_timed_model`/`invalid-task-set`, naming
  the task, because the analysis does not read the term or does not cover
  that deadline.
- A `response` claim SHALL name a task of the set, refusing
  `missing_declaration`/`missing-name` otherwise.
- **Several defects.** The checker SHALL report every defect, none hiding
  another: first the task set's own defects (the conditions above that need
  no claim, and duplicate names), in source order of their spans; then, for
  each claim over the task set in source order, that claim's defects (the
  priority, `edf`, `amc` and `response` conditions), in source order of
  their spans. A task set refused for its own defects is still checked
  against each claim.
- The task set, including every WCET, blocking term and jitter, SHALL be
  part of the claim's subject and obligation identity (ADR-013 O-09): S4
  emits each checked claim as one QSpec FR-482 `claim`/`schedulability`
  node holding the task set (FR-454), and the identity is keyed from that
  node.
- The request writer SHALL give each claim the requirement
  (`schedulability`, policy) and route it by negotiation (FR-075), where
  EN-7 registers a provider manifest for that kind.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-246-AC-1 | ADR-026 §12's `Ctl` (periodic, `(C, T = D)` of `(1, 4)`, `(2, 6)` and `(5, 12)` ms, priorities by period) checks; `schedulable CtlFp on Ctl under fixed_priority;` and `schedulable CtlEdf on Ctl under edf;` route to EN-7 under `schedulability`. The `Ctl` with the third WCET 6 gives a different obligation identity. | Test (TC-701) |
| FR-246-AC-2 | Each of these refuses `invalid_timed_model`/`invalid-task-set` naming the task (the task set for the empty one), and both tasks for a clash: a task set with no task; a period of 0 ms; a deadline of 0 ms; a WCET of 0 ms; a jitter of -1 ms; a blocking term of -1 ms; a sporadic task with minimum inter-arrival time 0 ms; a `HI` task with `C(LO)` 3 and `C(HI)` 2; a `LO` task with dual budgets; a task with no priority under `fixed_priority`; two tasks with priority 1 under `fixed_priority`; a task with blocking 1 ms under `edf`; a task with jitter 1 ms under `edf`; a task with no priority and two tasks with priority 1 under `amc`; a task with no priority under `response R on Ctl.c <= 12 ms under fixed_priority;`; two tasks with priority 1 under `response R on Ctl.c <= 12 ms under fixed_priority;`; a task with deadline above its period under `amc`. Two tasks named `a` refuse `conflicting_declaration`/`duplicate-declaration`; `response R on Ctl.zz <= 5 ms under fixed_priority;` refuses `missing_declaration`/`missing-name`; a deadline of `3 m` refuses `ill_typed`/`type-mismatch`. The same priority clash and a task with no priority under `edf` check. | Test (TC-701) |
| FR-246-AC-3 | `Ctl` written in `ms` has unit `ms`, and FR-248 settles `CtlFp` with fixpoints `(1, 3, 12)`; `Ctl` with task `a`'s period written `4000 us` has unit `us`, every value and `response R on Ctl.c <= 12 ms under fixed_priority;`'s bound converted exactly (`12 ms` is `12000`), and fixpoints `(1000, 3000, 12000)`. Reordering those same tasks so `c` is first in source, while task `a` still has period `4000 us`, keeps unit `us`, all checked values and claim identity unchanged. `Ctl` with the first task's jitter `-1 ms` and the second's blocking `-1 ms`, under `CtlEdf`, reports four `invalid_timed_model`/`invalid-task-set` refusals in this order: the first task, the second task (the task set's own), then the first task and the second task (`edf`'s non-zero terms); with tasks declared in the order `a` (valid), `a` (valid) and `z` (period 0 ms), it reports `conflicting_declaration`/`duplicate-declaration` at the second `a`, then `invalid_timed_model`/`invalid-task-set` at `z`, in that order. `taskset Empty { }` with `response R on Empty.c <= 12 ms under fixed_priority;` has no unit, refuses `invalid-task-set` naming `Empty` and `missing_declaration`/`missing-name` at `c`, and keeps the bound `12 ms`. | Test (TC-701) |

## Dependencies

- ADR-026 §12 RT-1 to RT-3; ADR-013 O-09.
- [FR-075](FR-075-compute-candidates-from-registered-backends.md),
  [FR-230](FR-230-check-time-declarations-clocks-and-clock-constraints.md)
  (time units shared with timed models).
- QSpec FR-142 (units), QSpec FR-290 (capability kinds).

## References

- QSpec half: QSpec FR-419 (Linear STD-139) owns the `taskset` grammar,
  the `schedulable` and `response` claims and the FR-290 capability kind
  `schedulability` (ADR-026 OV-10).
