---
id: TC-407
title: "A false dispatched precondition reaches the caller as a family-owned undefined result, not a kernel Undefined"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: verifies
---
# TC-407: A false dispatched precondition reaches the caller as a family-owned undefined result, not a kernel Undefined

## Description

Verify FR-090-AC-11. Suppose an FR-151 dispatched call's selected method has
an effective precondition that evaluates to `false`. The caller then receives
`FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause))` whose
`cause.undefined_reason()` is `precondition-false`, category `undefined`. The
result is not a family refusal, not a kernel `Outcome::Undefined` inside
`FamilyOutcome::Evaluated`, not a dispatch `FamilyOutcome::Refused` and not a
panic. The kernel `Undefined` has no `PreconditionFalse` variant (ADR-013
O-13, O-16). Scope: FR-090-AC-11.

The fixture is `tests/it/dispatch_calls.rs`'s D06 scenario (QSpec TC-196
D06): an `A`-typed receiver whose operation `size` has precondition `false`.
That test asserts `Outcome::Undefined(Undefined::PreconditionFalse(_))` on
QSL's kernel copy, which is the shape ADR-013 O-16 removes.

This catches three faults: keeping `PreconditionFalse` as a kernel
`Undefined` reason; carrying it in `FamilyResult::Refused`, which collapses
category `undefined` into `refusal`; and carrying it as a `FamilyRefusal`,
which reads as "the family did not run".

## Test Procedure

1. Reuse `tests/it/dispatch_calls.rs`'s D06 package: an `A`-typed receiver
   `a1` whose operation `size` has precondition `false` and no redefinition
   ancestor.
2. Evaluate the dispatched call through the public S6a entry point, with an
   unlimited meter.
3. Match the result as
   `Ok(FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause)))` and
   take `cause.undefined_reason()`.
4. Read the cause's payload through its owning `value::expression` type.
5. Inspect `quire-exact/src/outcome.rs`'s `Undefined` enum.

Tag the test `#[trace("FR-090-AC-11", "TC-407")]`.

## Expected Results

- Step 3's reason is `UndefinedReason` `precondition-false`.
- Step 4's payload names operation `size`, selected method `model.A.size`
  and receiver `a1`.
- Step 2 does not panic, and its result matches step 3's pattern; it is not
  `Ok(FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(_)))`, not
  `Ok(FamilyOutcome::Evaluated(Outcome::Undefined(_)))` and not
  `Ok(FamilyOutcome::Refused(_))`.
- Step 5 finds no `PreconditionFalse` variant.

## Status

Planned; no test backs this case.
