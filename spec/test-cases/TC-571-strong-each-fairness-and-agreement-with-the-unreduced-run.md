---
id: TC-571
title: "Strong each fairness refines on the quotient, and every verdict equals the unreduced verdict"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-153
    type: verifies
---
# TC-571: Strong each fairness refines on the quotient, and every verdict equals the unreduced verdict

## Description

Verify strong `each` fairness and its refinement on the annotated quotient, agreement with unreduced runs, determinism and the meter stop.

Scope: FR-153-AC-3 to FR-153-AC-4.

## Test Procedure

Fixtures: The `Toggle` unit of FR-153-AC-3; ADR-021 §7.1's subject.

1. Run the `Toggle` claim under `fair weak each flip` with `fair weak each fire`, and with `fair strong each fire`, reduced and unreduced.
2. Run FR-153-AC-1 and AC-2 unreduced; run every reduced request twice; run AC-2 with the meter below its orbit-closure cost.

Tag the tests `#[trace("TC-571", "FR-153-AC-n")]`.

## Expected Results

- Step 1: `Violated` (alternating `flip` loop) and `Holds`; in the strong run, the alternating SCC fails strong `each` and the refinement removes its `fire`-enabling states; reduced verdicts equal unreduced verdicts.
- Step 2: equal verdicts; equal generators on repeat; `Stopped(ResourceExhausted, …)` naming the meter.
