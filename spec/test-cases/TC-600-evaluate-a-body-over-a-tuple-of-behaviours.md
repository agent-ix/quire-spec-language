---
id: TC-600
title: "The evaluator reads a hyper body over a tuple of lassos in lockstep"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-175
    type: verifies
---
# TC-600: The evaluator reads a hyper body over a tuple of lassos in lockstep

## Description

Verify lockstep tuple evaluation: indexed atoms per component, the joint stem and period, past operators across loop copies, terminal stutter and interval counting.

Scope: FR-175-AC-1 to FR-175-AC-3.

## Test Procedure

Fixtures: ADR-023 §8.1's vault traces; a pair of lassos with loop lengths 2 and 3; a pair where `a` ends at a terminal state.

1. Evaluate `always holds(v.l @ a = v.l @ b)` over the leaky and secure pairs.
2. Evaluate over the 2-and-3 loop pair; evaluate a past operator under `always` at a loop position and its unrolled copy.
3. Evaluate the terminal pair's step labels and atoms; evaluate `eventually[0,2] holds(v.l @ a = 1)`.

Tag the tests `#[trace("TC-600", "FR-175-AC-n")]`.

## Expected Results

- Step 1: `false` at joint position 1; `true`.
- Step 2: `false` at the fifth position of the joint period of 6, with `trace_position` naming it; equal values at both positions.
- Step 3: `stutter` labels and terminal-state reads for `a`; the interval counts joint positions.
