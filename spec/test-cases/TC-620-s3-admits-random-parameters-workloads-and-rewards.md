---
id: TC-620
title: "S3 admits random parameters, workloads and rewards and refuses malformed ones"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-185
    type: verifies
---
# TC-620: S3 admits random parameters, workloads and rewards and refuses malformed ones

## Description

Verify S3's checks of random parameters, workloads and rewards on ADR-024 §7.2's `Service` model and its malformed variants.

Scope: FR-185-AC-1 to FR-185-AC-4.

## Test Procedure

Fixtures: ADR-024 §7.2's `Service` model and the variants FR-185-AC-2 to FR-185-AC-4 list.

1. Check `Service` and read back `attempt`'s random parameters, the `duration` reward and the workload `Steady`.
2. Check each random-parameter variant of FR-185-AC-2.
3. Check each workload variant of FR-185-AC-3.
4. Check each reward variant of FR-185-AC-4, and `reward cost = 2` on `request`.

Tag the tests `#[trace("TC-620", "FR-185-AC-n")]`.

## Expected Results

- Step 1: supports and probabilities `9/10`, `7/100`, `3/100` and `49/50`, `1/50`; `duration: Quantity<Time>`; weight 1 per operation.
- Step 2: each variant refused with a diagnostic naming the parameter.
- Step 3: each variant refused naming the workload and the operation.
- Step 4: the three variants refused naming the reward; `cost` checks.
