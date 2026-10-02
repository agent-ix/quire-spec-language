---
id: TC-702
title: "EN-7 computes fixed-priority, EDF and AMC verdicts in exact arithmetic"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-247
    type: verifies
---
# TC-702: EN-7 computes fixed-priority, EDF and AMC verdicts in exact arithmetic

## Description

Verify response-time analysis, the EDF demand test, AMC-rtb, response claims and the demand-point budget.

Scope: FR-247-AC-1 to FR-247-AC-4.

## Test Procedure

Fixtures: `Ctl`, its WCET-6 variant, the jitter-and-blocking set of FR-247-AC-2, the AMC set `Mc` and its variants, an EDF set with a QPA sequence longer than 3 points.

1. Analyse `Ctl` under fixed priority and EDF, and its two response claims.
2. Analyse the WCET-6 variant under both policies, and the jitter-and-blocking set under fixed priority, rechecking its evidence.
3. Analyse `Mc`, its `C(HI) = 6` variant and its `C(LO) = C(HI) = 5` variant.
4. Analyse the EDF set with `max_demand_points` 3.

Tag the tests `#[trace("TC-702", "FR-247-AC-n")]`.

## Expected Results

- Step 1: fixpoints `(1, 3, 12)` with iteration `5, 9, 12, 12`; EDF schedulable; `<= 12 ms` holds, `<= 11 ms` misses.
- Step 2: fixed-priority miss for the third task, job 0, iterates `6, 10, 13`; the jitter-and-blocking set misses for `b`, job 0, iterates `5, 7`, and its evidence rechecks; `Miss(Utilization(13/12))`.
- Step 3: `Schedulable`, low-mode 2, high-mode 4; `SufficientTestFailed`; `Miss`.
- Step 4: `Stopped(LimitReached, MaxDemandPoints)` naming 3.
