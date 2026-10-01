---
id: FR-247
title: "Analyse a task set in closed form (EN-7)"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-026
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-246
    type: depends_on
---
# FR-247: Analyse a task set in closed form (EN-7)

## Description

EN-7 SHALL decide schedulability and response claims in exact rational
arithmetic by fixed-priority response-time analysis, the EDF
processor-demand test with Quick Processor-demand Analysis, and AMC-rtb
for dual-criticality task sets (ADR-026 RT-4 to RT-6). Each analysis
returns its evidence for FR-248.

## Use case

An embedded engineer gets each task's exact worst-case response time, or
the scheduling points where demand exceeds time, from an analysis that
terminates on its own and needs no iteration cap.

## Inputs

- A checked `TaskSet` and `ScheduleClaim` (FR-246), and
  `ScheduleLimits { max_demand_points: u64 }`, default 1_048_576 (2^20),
  with a cancellation poll.

## Outputs

```rust
pub enum ClosedFormOutcome {
    Schedulable(ClosedFormEvidence),
    Miss(MissEvidence),
    SufficientTestFailed(ClosedFormEvidence),
    Stopped(IncompleteCause, ScheduleLimit),
}
// ClosedFormEvidence: FixedPoints(Vec<(task, R)>) | Demand { busy_period, qpa_sequence }
// MissEvidence: FixedPriority { task, points: Vec<(t, demand)> } | Edf(DemandWitness)
//             | Utilization(ExactRational)
```

## Behavior

- **Fixed priority.** EN-7 SHALL compute each task's response time as the
  least fixpoint of `R = C_i + B_i + Σ_{j ∈ hp(i)} ⌈(R + J_j) / T_j⌉ · C_j`,
  iterated from `C_i + B_i` in exact rationals, with response time
  `R + J_i`. It SHALL stop at the fixpoint, or as soon as `R + J_i` exceeds
  `D_i`. When `D_i > T_i`, it SHALL use the busy-period extension.
- **Miss evidence.** For a fixed-priority miss, EN-7 SHALL return the task
  and its scheduling points up to `D_i` (the multiples of higher-priority
  periods and `D_i`), with the demand at each.
- **EDF.** EN-7 SHALL return `Miss(Utilization(U))` when the utilization `U`
  exceeds 1. Otherwise it SHALL check `dbf(t) = Σ_i max(0, ⌊(t − D_i) /
  T_i⌋ + 1) · C_i <= t` at absolute deadlines up to the synchronous
  busy-period length, by Quick Processor-demand Analysis, and return the
  busy period and QPA's check sequence, or a `t` with `dbf(t) > t`.
- **AMC.** EN-7 SHALL run AMC-rtb: low-mode response times with `C(LO)`,
  high-mode response times with `C(HI)` over higher-priority `HI` tasks
  plus higher-priority `LO` tasks charged up to the low-mode response time.
- EN-7 SHALL return `Miss` on a low-mode miss, and `SufficientTestFailed`
  when a high-mode criterion fails.
- **Response claims.** For a `response S.τ <= d` claim, EN-7 SHALL compare
  the computed response time of `τ` with `d`.
- EN-7 SHALL count demand points checked and stop with
  `Stopped(ResourceExhausted, MaxDemandPoints)` when the count would exceed
  `max_demand_points`, and `Stopped(Cancelled, …)` when the poll returns
  `true`.
- EN-7 SHALL compute every value exactly, as a function of the task set,
  the claim and the limits.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-247-AC-1 | `Ctl` under fixed priority returns `Schedulable` with fixpoints `(1, 3, 12)`, the third iterating `5, 9, 12, 12`; under EDF, utilization exactly 1, `Schedulable` with its busy period and QPA sequence. `response Ctl.c <= 12 ms` holds and `<= 11 ms` misses. | Test (TC-702) |
| FR-247-AC-2 | `Ctl` with the third WCET 6 returns, under fixed priority, `Miss` for the third task with points `4, 6, 8, 12` and demands `9, 10, 12, 13`; under EDF, `Miss(Utilization(13/12))`. | Test (TC-702) |
| FR-247-AC-3 | AMC set `Mc`: `a` `LO`, `C = 1`, `T = D = 4`, priority 1; `b` `HI`, `C(LO) = 1`, `C(HI) = 3`, `T = D = 6`, priority 2. It returns `Schedulable` with low-mode response 2 for `b` and high-mode response 4. With `C(HI) = 6` it returns `SufficientTestFailed`; with `C(LO) = C(HI) = 5` it returns `Miss` in low mode. | Test (TC-702) |
| FR-247-AC-4 | An EDF set with utilization below 1 whose QPA check sequence under the default limit has more than 3 points, run with `max_demand_points` 3, returns `Stopped(ResourceExhausted, MaxDemandPoints)` naming the value 3. | Test (TC-702) |

## Dependencies

- ADR-026 §12 RT-4 to RT-6; ADR-014 B-5.
- [FR-246](FR-246-check-task-sets-and-schedulability-claims.md).

## References

- M. Joseph and P. Pandya, 1986; N. Audsley et al., 1993; J. Lehoczky,
  1990; S. Baruah, L. Rosier and R. Howell, 1990; F. Zhang and A. Burns,
  2009; S. Baruah, A. Burns and R. Davis, 2011 (ADR-026 References).
