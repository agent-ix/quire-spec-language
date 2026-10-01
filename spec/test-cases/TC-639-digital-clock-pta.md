---
id: TC-639
title: "Closed probabilistic timed automata are decided exactly through digital clocks, with the workload resolving delays"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-204
    type: verifies
---
# TC-639: Closed probabilistic timed automata are decided exactly through digital clocks, with the workload resolving delays

## Description

Verify the digital MDP for §15.5, its deadline and expected-time results, the workload-resolved DTMC with discrete delays, and every unsupported cause of the route.

Scope: FR-204-AC-1 to FR-204-AC-4.

## Test Procedure

Fixtures: §15.5's `Retx` with `Deadline` and `MeanTime`; its strict, `discrete`-delay, `exponential`-delay and zero-delay-cycle variants; an `always[0 ms, 4 ms]` claim.

1. Decide `Deadline` at 0.99 and 0.995 over every scheduler; replay the refutation.
2. Decide `MeanTime`; decide both with the strict guard.
3. Decide the deadline claim on the `discrete`-delay variant under `One` and over every scheduler; on the `exponential` variant under `One`.
4. Decide the zero-delay-cycle variant and the `always[0 ms, 4 ms]` claim.

Tag the tests `#[trace("TC-639", "FR-204-AC-n")]`.

## Expected Results

- Step 1: clock ranges 0 to 2 and 0 to 5; `ExactValue{99/100}`; `refuted` with probability `1/100`, replay checking the 4 ms horizon.
- Step 2: `ExactValue{20/9 ms}`; `StrictClockConstraint` naming the guard.
- Step 3: `ExactValue{158509/160000}`; `DelayDistribution` naming `send`; `DelayDistribution`.
- Step 4: `ZeroDelayCycle` naming the cycle; `TimedFormShape`.
