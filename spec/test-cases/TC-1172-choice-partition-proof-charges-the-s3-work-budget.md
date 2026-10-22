---
id: TC-1172
title: "The choice partition proof charges the S3 work budget"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: verifies
---
# TC-1172: The choice partition proof charges the S3 work budget

## Description

Verify that the partition proof of a `choice` charges one `s3.work_units` unit per valuation and nothing for comparing atoms, stops with a named limit outcome located at the choice, and is raised by the setting at every entry point, with no ceiling of its own.

Scope: FR-218-AC-30, FR-096-AC-22.

## Test Procedure

Fixture: TC-1056's fixture P with a protocol whose `choice` lists the Boolean facts `a`, `b`, `c` in its `visible` entry; a variant over 24 visible Boolean facts `a1` to `a24`.

1. Check the choice with guards `a and b and c` and `not (a and b and c)`, and the same unit with guards `a and a and a` and `not (a and a and a)`, at the default limits; read each unit's measured `s3.work_units` spend.
2. Check the 24-atom variant, guards `a1 and ... and a24` and `not (a1 and ... and a24)`, at the default limits; read the refusal, its `StageLimitCause` and `DeclarationRegions::refusal_region`.
3. Check the 24-atom variant with `s3.work_units` at 33554432 through the library builder, a replay request's `stage_limits` entry and FR-255's settings operation.
4. Check the choice with guards `a and b and c` and `not a`.

Tag the tests `#[trace("TC-1172", "FR-218-AC-30")]` and `#[trace("TC-1172", "FR-096-AC-22")]`.

## Expected Results

- Step 1: both check; the spends differ by exactly 6.
- Step 2: `CheckRefusal`/`stage_limit_exceeded`, cause `work-budget-exceeded`, naming bound 16777216 and `s3.work_units`, `region: None` on the cause and `refusal_region` the choice's span; no `unproved-exhaustiveness`.
- Step 3: each raised run checks.
- Step 4: `undefined_expression`/`unproved-exhaustiveness` with `failed` `[missing-arm]`.
