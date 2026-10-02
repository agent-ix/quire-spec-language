---
id: TC-795
title: "Unit compile refuses a dispatch family past the caller's family_steps limit"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-302
    type: verifies
---
# TC-795: Unit compile refuses a dispatch family past the caller's family_steps limit

## Description

Verify the `family_steps` limit at compile. Scope: FR-302-AC-5.

## Test Procedure

1. Compile a unit whose called operation's family has `n = 3` `redefines`
   edges with `SpineLimits` setting `family_steps = 3`.
2. Compile the same unit with `family_steps = 2`.

## Expected Results

1. The compile succeeds.
2. Refused at S3 with the resource-exhaustion cause `family-steps`, naming
   `family_steps`, the value 2
   and the limits field that raises it; no checked package.
