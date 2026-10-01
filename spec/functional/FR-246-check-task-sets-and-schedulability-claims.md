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
route each claim to the closed-form engine EN-7, module `schedulability`,
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
- The checker SHALL refuse, each with `invalid_model_binding`/
  `malformed-declaration` at the span: a period or minimum inter-arrival
  time that is not positive; a deadline that is not positive; a WCET that is
  not positive; a `HI` task whose `C(LO)` exceeds `C(HI)`; a `LO` task with
  dual budgets.
- Two tasks with one name SHALL refuse `conflicting_declaration`/
  `duplicate-declaration`.
- A `fixed-priority` or `amc` claim over a task set where two tasks share a
  priority, or a task has none, SHALL refuse `invalid_model_binding`/
  `malformed-declaration`, naming the tasks. An `edf` claim SHALL ignore
  priorities.
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
| FR-246-AC-2 | Refusals at the span: a period of 0 ms; a `HI` task with `C(LO)` 3 and `C(HI)` 2; two tasks named `a`; two tasks with priority 1 under `fixed-priority`; `response Ctl.zz <= 5 ms`; a deadline of `3 m`. The same priority clash under `edf` checks. | Test (TC-701) |

## Dependencies

- ADR-026 §12 RT-1 to RT-3; ADR-013 O-09.
- [FR-075](FR-075-compute-candidates-from-registered-backends.md),
  [FR-230](FR-230-check-time-declarations-clocks-and-clock-constraints.md)
  (time units shared with timed models).
- QSpec FR-142 (units), QSpec FR-290 (capability kinds).

## References

- QSpec half: Linear STD-139 owns the `taskset` grammar, the `schedulable`
  and `response` claims and the FR-290 capability kind `schedulability`
  (ADR-026 OV-10); this requirement cites it until those QSpec FRs merge.
