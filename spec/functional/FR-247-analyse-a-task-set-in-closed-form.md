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

An embedded engineer gets each task's exact worst-case response time, or the response-time iterates that prove a miss, from an analysis that
terminates on its own and needs no iteration cap.

## Inputs

- A checked `TaskSet` and `ScheduleClaim` (FR-246), and
  `ScheduleLimits { max_demand_points: u64 }`, default 1_048_576 (2^20),
  and FR-276's `Cancel` handle.

## Outputs

```rust
pub enum ClosedFormOutcome {
    Schedulable(ClosedFormEvidence),
    Miss(MissEvidence),
    SufficientTestFailed(ClosedFormEvidence),
    Stopped(IncompleteCause, ScheduleLimit),
}
// ClosedFormEvidence: FixedPoints(Vec<(task, R)>) | Demand { busy_period, qpa_sequence }
// MissEvidence: FixedPriority { task, job: u64, iterates: Vec<ExactRational> } | Edf(DemandWitness)
//             | Utilization(ExactRational)
```

## Behavior

- **Analyses.** EN-7 SHALL implement QSpec FR-419's fixed-priority
  response-time analysis with the busy-period extension, its EDF
  processor-demand test with Quick Processor-demand Analysis, and its
  AMC-rtb test, in exact rationals. A fixed-priority iteration SHALL stop
  at its fixpoint or as soon as the response time exceeds the deadline.
- **Miss evidence.** For a fixed-priority miss, EN-7 SHALL return the task,
  the job `q` of its level-`i` busy period that misses (0 when `D_i <= T_i`
  and the busy period ends with the first job), and that job's RT-4
  iterates up to the first whose response time `R + J_i − q · T_i` exceeds
  `D_i`, so the evidence covers jitter, blocking and `D_i > T_i`
  (ADR-026 RT-4, RT-7).
- **EDF evidence.** EN-7 SHALL return `Miss(Utilization(U))` when the
  utilization exceeds 1, a `t` with `dbf(t) > t` for a demand miss, and
  otherwise the busy period and QPA's check sequence.
- EN-7 SHALL return `Miss` on a low-mode miss, and `SufficientTestFailed`
  when a high-mode criterion fails.
- **Response claims.** For a `response S.τ <= d` claim, EN-7 SHALL compare
  the computed response time of `τ` with `d`.
- EN-7 SHALL count demand points checked and stop with
  `Stopped(LimitReached, MaxDemandPoints)` when the count would exceed
  `max_demand_points`, and `Stopped(Cancelled, …)` when the `Cancel` handle
  is cancelled.
- EN-7 SHALL compute every value exactly, as a function of the task set,
  the claim and the limits.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-247-AC-1 | `Ctl` under fixed priority returns `Schedulable` with fixpoints `(1, 3, 12)`, the third iterating `5, 9, 12, 12`; under EDF, utilization exactly 1, `Schedulable` with its busy period and QPA sequence. `response Ctl.c <= 12 ms` holds and `<= 11 ms` misses. | Test (TC-702) |
| FR-247-AC-2 | `Ctl` with the third WCET 6 returns, under fixed priority, `Miss` for the third task, job 0, iterates `6, 10, 13`. A set of `a` (`C = 1`, `T = D = 4`, priority 1) and `b` (`C = 2`, `T = D = 6`, jitter 1, blocking 3, priority 2) returns `Miss` for `b`, job 0, iterates `5, 7` (response `8 > 6`), and `check_closed_form` accepts that evidence; under EDF, `Miss(Utilization(13/12))`. | Test (TC-702) |
| FR-247-AC-3 | AMC set `Mc`: `a` `LO`, `C = 1`, `T = D = 4`, priority 1; `b` `HI`, `C(LO) = 1`, `C(HI) = 3`, `T = D = 6`, priority 2. It returns `Schedulable` with low-mode response 2 for `b` and high-mode response 4. With `C(HI) = 6` it returns `SufficientTestFailed`; with `C(LO) = C(HI) = 5` it returns `Miss` in low mode. | Test (TC-702) |
| FR-247-AC-4 | An EDF set with utilization below 1 whose QPA check sequence under the default limit has more than 3 points, run with `max_demand_points` 3, returns `Stopped(LimitReached, MaxDemandPoints)` naming the value 3. | Test (TC-702) |

## Dependencies

- ADR-026 §12 RT-4 to RT-6; ADR-014 B-5; QSpec FR-419 (the analyses).
- [FR-246](FR-246-check-task-sets-and-schedulability-claims.md).

## References

- M. Joseph and P. Pandya, 1986; N. Audsley et al., 1993; J. Lehoczky,
  1990; S. Baruah, L. Rosier and R. Howell, 1990; F. Zhang and A. Burns,
  2009; S. Baruah, A. Burns and R. Davis, 2011 (ADR-026 References).
