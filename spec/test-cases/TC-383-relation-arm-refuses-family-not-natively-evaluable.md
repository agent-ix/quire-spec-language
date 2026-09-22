---
id: TC-383
title: "The S6a Relation arm refuses FamilyNotNativelyEvaluable without calling an evaluate hook"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: verifies
---
# TC-383: The S6a Relation arm refuses FamilyNotNativelyEvaluable without calling an evaluate hook

## Description

Verify FR-090-AC-2, the precise form of FR-062-AC-6. S6a's `Relation` arm
returns `Ok(FamilyOutcome::Refused(FamilyRefusal::FamilyNotNativelyEvaluable))`
without calling any `evaluate` hook (ADR-012 §2, §3; ADR-013 O-16, Q210-3).
Scope: FR-090-AC-2.

This catches two faults. The first routes `Relation` through a `_` arm into
a different family's evaluator. The second reports the non-evaluable family
as a kernel `Outcome::Refused(Refusal::CheckedInvariant)`, which puts a
family-dispatch cause in the kernel.

## Test Procedure

1. Call the S6a seam with `FamilyKind::Relation`, a declaration identity and
   a meter.
2. Separately, confirm that the `Relation` arm references no
   `ReferenceEvaluation::evaluate` implementation. `Relation` implements
   none (ADR-012 §2), so an arm that called one would not compile.

Tag the test `#[trace("FR-090-AC-2", "TC-383")]`.

## Expected Results

- Step 1 returns exactly
  `Ok(FamilyOutcome::Refused(FamilyRefusal::FamilyNotNativelyEvaluable))`.
- The result is not `Ok(FamilyOutcome::Evaluated(_))` and not `Err(_)`.

## Status

Planned; no test backs this case.
