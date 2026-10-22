---
id: TC-1172
title: "The choice partition proof charges the S3 work budget"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: verifies
---
# TC-1172: The choice partition proof charges the S3 work budget

## Description

Verify that the partition proof of a `choice` charges one `s3.work_units` unit per valuation, stops with a named limit outcome at the budget, and is raised by the setting at every entry point, with no ceiling of its own.

Scope: FR-218-AC-30.

## Test Procedure

Fixture: TC-1056's fixture P with a protocol whose `choice` lists the Boolean facts `a`, `b`, `c` in its `visible` entry; a variant over 24 visible Boolean facts `a1` to `a24`.

1. Check the three-atom choice with guards `a and b and c` and `not (a and b and c)` at the default limits and read the work spent, `w`.
2. Check it with `s3.work_units` at `w`, then at `w - 1`.
3. Check the 24-atom variant at the default limits, then with `s3.work_units` at 33554432 through the library builder, a replay request's `stage_limits` entry and FR-255's settings operation.
4. Check the three-atom choice with guards `a and b` and `not a`, which leave `a and not b` uncovered.

Tag the tests `#[trace("TC-1172", "FR-218-AC-30")]`.

## Expected Results

- Step 1: checks; the choice's proof charges 8 units.
- Step 2: checks at `w`; at `w - 1`, `stage_limit_exceeded`/`work-budget-exceeded` at the choice's span naming bound `w - 1`, count `w` and `s3.work_units`, with no `unproved-exhaustiveness`.
- Step 3: the default run stops with the same outcome naming `s3.work_units`; each raised run checks.
- Step 4: `undefined_expression`/`unproved-exhaustiveness` with `failed` `[missing-arm]`.
