---
id: TC-383
title: "The S6a Relation arm refuses FamilyNotNativelyEvaluable without evaluating or charging"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: verifies
---
# TC-383: The S6a Relation arm refuses FamilyNotNativelyEvaluable without evaluating or charging

## Description

Verify FR-090-AC-2. S6a's `Relation` arm returns
`Ok(FamilyOutcome::Refused(FamilyRefusal::FamilyNotNativelyEvaluable))`,
calls no `evaluate` hook and charges nothing to the meter (ADR-012 §2, §3;
ADR-013 O-16, Q210-3). Scope: FR-090-AC-2.

This catches three faults. The first routes `Relation` through a `_` arm into
a different family's evaluator. The second reports the non-evaluable family
as a kernel `Outcome::Refused(Refusal::CheckedInvariant)`, which puts a
family-dispatch cause in the kernel. The third charges work before refusing.

## Test Procedure

1. Build a `quire_exact::Meter` with finite limits, and record its usage
   counters.
2. Call the S6a seam with `FamilyKind::Relation`, a declaration identity and
   the meter.
3. Read the meter's usage counters again.

Tag the test `#[trace("FR-090-AC-2", "TC-383")]`.

## Expected Results

- Step 2 returns exactly
  `Ok(FamilyOutcome::Refused(FamilyRefusal::FamilyNotNativelyEvaluable))`.
- The step-3 counters equal the step-1 counters.
- The result is not `Ok(FamilyOutcome::Evaluated(_))` and not `Err(_)`.
