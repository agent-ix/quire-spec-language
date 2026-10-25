---
id: TC-1172
title: "Choice and repeat proofs charge the S3 work budget"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: verifies
---
# TC-1172: Choice and repeat proofs charge the S3 work budget

## Description

Verify that choice partition and nested-repeat feasibility proofs use the
independent-atom abstraction, charge one `s3.work_units` unit per valuation,
stop with a named limit outcome at the construct being proved and are raised
by the setting at every entry point, with no proof-specific ceiling.

Scope: FR-218-AC-30, FR-218-AC-32, FR-096-AC-22.

## Test Procedure

Fixture: TC-1056's fixture P with a protocol whose `choice` lists the Boolean facts `a`, `b`, `c` in its `visible` entry; a variant over 24 visible Boolean facts `a1` to `a24`.

1. Check the choice with guards `a and b and c` and `not (a and b and c)`, and the same unit with guards `a and a and a` and `not (a and a and a)`, at the default limits; read each unit's measured `s3.work_units` spend.
2. Check the 24-atom variant, guards `a1 and ... and a24` and `not (a1 and ... and a24)`, at the default limits; read the refusal, its `StageLimitCause` and `DeclarationRegions::refusal_region`.
3. Check the 24-atom variant with `s3.work_units` at 33554432 through the library builder, a replay request's `stage_limits` entry and FR-255's settings operation.
4. Check the choice with guards `a and b and c` and `not a`.
5. In an outer repeat whose only body is a nested repeat with an event in its
   body, compare `repeat max 1 while a` with
   `repeat max 1 while (a or not a or b or not b)`; list `a` and `b` as
   individually visible facts. Measure the latter's successful spend, rerun
   at that exact `s3.work_units` limit and one unit less, and raise the
   successful limit through the library builder, replay `stage_limits` and
   FR-255's settings operation.

Tag the tests `#[trace("TC-1172", "FR-218-AC-30")]`,
`#[trace("TC-1172", "FR-218-AC-32")]` and
`#[trace("TC-1172", "FR-096-AC-22")]`.

## Expected Results

- Step 1: both check; the spends differ by exactly 6.
- Step 2: `CheckRefusal`/`stage_limit_exceeded`, cause `work-budget-exceeded`, naming bound 16777216 and `s3.work_units`, `region: None` on the cause and `refusal_region` the choice's span; no `unproved-exhaustiveness`.
- Step 3: each raised run checks.
- Step 4: `undefined_expression`/`unproved-exhaustiveness` with `failed` `[missing-arm]`.
- Step 5: with `while a`, the feasible false valuation retains the skip edge
  and the outer repeat refuses `unsupported_construct`/`declaration-form`
  for missing progress. With the tautological guard, false is infeasible,
  the skip edge is absent and the outer repeat checks from the nested repeat's
  entered-body progress. That proof checks at the measured exact limit. One
  unit less stops at the nested repeat's span with `CheckRefusal`/
  `stage_limit_exceeded`, cause `work-budget-exceeded`, naming the bound, the
  denied charge count and `s3.work_units`; it does not report an enclosing
  progress refusal. Each raised entry point checks.
