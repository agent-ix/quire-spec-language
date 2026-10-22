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

- A parsed `taskset S { task τ … }` declaration and `schedulable S under
  P` and `response S.τ <= d under fixed-priority` claims, in the spelling
  QSpec's shared grammar fixes.

## Outputs

```rust
pub struct TaskSet { pub name: QualifiedName, pub tasks: Vec<Task> }
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

- Every time value SHALL be an exact rational time quantity converted to
  one unit per task set (QSpec FR-142); a value of another dimension SHALL
  refuse `ill_typed`/`type-mismatch` at its span.
- The checker SHALL refuse, each with `invalid_timed_model`/
  `invalid-task-set` at the span, naming the task (the task set when it
  declares none): a task set that declares
  no task; a period or minimum inter-arrival time that is not positive; a
  deadline that is not positive; a WCET that is not positive; a jitter or
  blocking term that is negative; a `HI` task whose `C(LO)` exceeds `C(HI)`;
  a `LO` task with dual budgets. QSpec FR-419 states the code and the
  conditions on `T`, `D`, `C`, the budgets and the priorities; the empty
  set, the negative terms and the two claim-dependent conditions below are
  QSL's own, refused under the same code so that one invalid task set
  refuses under one code.
- Two tasks with one name SHALL refuse `conflicting_declaration`/
  `duplicate-declaration`.
- When a `fixed-priority`, `amc` or `response` claim is over a task set where
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
- The task set, including every WCET, blocking term and jitter, SHALL be
  part of the claim's subject and obligation identity (ADR-013 O-09).
- The request writer SHALL give each claim the requirement
  (`schedulability`, policy) and route it by negotiation (FR-075), where
  EN-7 registers a provider manifest for that kind.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-246-AC-1 | ADR-026 §12's `Ctl` (periodic, `(C, T = D)` of `(1, 4)`, `(2, 6)` and `(5, 12)` ms, priorities by period) checks; `schedulable Ctl under fixed-priority` and `under edf` route to EN-7 under `schedulability`. The `Ctl` with the third WCET 6 gives a different obligation identity. | Test (TC-701) |
| FR-246-AC-2 | Each of these refuses `invalid_timed_model`/`invalid-task-set` naming the task (the task set for the empty one), and both tasks for a clash: a task set with no task; a period of 0 ms; a deadline of 0 ms; a WCET of 0 ms; a jitter of -1 ms; a blocking term of -1 ms; a sporadic task with minimum inter-arrival time 0 ms; a `HI` task with `C(LO)` 3 and `C(HI)` 2; a `LO` task with dual budgets; a task with no priority under `fixed-priority`; two tasks with priority 1 under `fixed-priority`; a task with blocking 1 ms under `edf`; a task with jitter 1 ms under `edf`; a task with no priority and two tasks with priority 1 under `amc`; a task with no priority under `response Ctl.c <= 12 ms`; two tasks with priority 1 under `response Ctl.c <= 12 ms`; a task with deadline above its period under `amc`. Two tasks named `a` refuse `conflicting_declaration`/`duplicate-declaration`; `response Ctl.zz <= 5 ms` refuses `missing_declaration`/`missing-name`; a deadline of `3 m` refuses `ill_typed`/`type-mismatch`. The same priority clash and a task with no priority under `edf` check. | Test (TC-701) |

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
