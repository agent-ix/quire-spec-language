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
2. Refused at S3 with `resource_exhausted`/`insufficient-next-charge`, naming
   the setting `model.family_steps`, the limit 2
   and the limits field that raises it; no checked package.

Until unit compile links dispatch (it does not yet reach the checked dispatch
bridge), the bridge's own test backs the charge record: it drives the
family walk over a built chain and asserts the code, cause and record of
step 2. The unit-level steps become the test once compile links dispatch.
