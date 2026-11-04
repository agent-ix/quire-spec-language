---
id: TC-642
title: "A delay no distribution gives is decided by its minimum or maximum under a workload"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-204
    type: verifies
---
# TC-642: A delay no distribution gives is decided by its minimum or maximum under a workload

## Description

Verify that under a workload EN-5 leaves the delay of an operation with no
`delay` declaration to the scheduler, assumes no distribution for it, and reads a
`>= θ` bound on the minimum and a `<= θ` bound on the maximum over that
choice.

Scope: FR-204-AC-5, FR-204-AC-6 and FR-204-AC-7.

## Test Procedure

Fixture: §15.5's `Retx`, whose `send` has no `delay` declaration, and the
workload `One` with weight 1 on `send`.

1. Decide `probability >= 0.99 [eventually[0 ms, 4 ms] holds(m.delivered)]`
   under `One`.
2. Decide `probability <= 0.999` over the same event under `One`, and read
   the witness scheduler.

3. Over `Race` with `B` guarded by `x >= 1 ms`, decide
   `probability >= 0.5 [eventually[0 ms, 5 ms] holds(r.b)]` and
   `probability <= 0.5` under `Both`, and read each witness scheduler's choice.
4. With `B` guarded by `x >= 2 ms`, decide `probability <= 0.5` and
   `probability <= 0.4` under `Both`, and the same claim under every scheduler.

Tag the tests `#[trace("TC-642", "<AC id>")]`.

## Expected Results

- Step 1: `proved`, `ExactValue{99/100}`, the entry marked a minimum.
- Step 2: `refuted` on the maximum `9999/10000`, the entry marked a maximum;
  the witness scheduler sends at `x = 1`.
- Step 3: `refuted` on the minimum `ExactValue{0}` with `WorkloadResolve{B, 3}`;
  `refuted` on the maximum `ExactValue{1}` with `WorkloadResolve{B, 1}`.
- Step 4: `proved`, `ExactValue{1/2}`, marked a maximum; `refuted` on that
  maximum with `WorkloadResolve{B, 2}`; `unsupported`, `DelayDistribution` naming
  `A` under every scheduler.
