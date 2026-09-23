---
id: TC-385
title: "S6a's input type admits no Relation, and FamilyOutcome has exactly two arms"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: verifies
---
# TC-385: S6a's input type admits no Relation, and FamilyOutcome has exactly two arms

## Description

Verify FR-090-AC-4, the precise form of FR-062-AC-6. The family kind S6a
dispatches over has no `Relation` variant, so no S6a call can name a
`Relation` declaration. `FamilyOutcome` has exactly the two variants
`Evaluated` and `FamilyEvaluated`, so no family-dispatch refusal is
representable (ADR-012 §2, §5.1 S1; ADR-013 O-16). Scope: FR-090-AC-4.

This catches two faults: a `Relation` variant added to S6a's input type,
which lets a `Relation` declaration reach S6a with nothing to evaluate it;
and a third `FamilyOutcome` arm added for a result nothing produces.

## Test Procedure

1. In the test body, write an exhaustive `match` with no `_` arm over the
   S6a family kind, the family kind S6a dispatches on. It has one arm per
   variant, one per family that implements `ReferenceEvaluation`, and no
   `Relation` arm.
2. In the test body, write an exhaustive `match` with no `_` arm over a
   `FamilyOutcome<Value>`, with the two arms `Evaluated(_)` and
   `FamilyEvaluated(_)`.
3. For each variant step 1 names, pass it to the S6a seam's dispatch with a
   checked package that declares no item of that family, and a meter.

Tag the test `#[trace("FR-090-AC-4", "TC-385")]`.

## Expected Results

- The test compiles. Steps 1 and 2 compile only while S6a's family kind has
  no variant beyond the arms listed and `FamilyOutcome` has no third
  variant: adding either variant fails the build with `E0004`.
- Step 1 has no `Relation` arm, so a build that passes shows S6a's family
  kind has no `Relation` variant.
- Step 3 compiles, which shows the seam's family parameter is that type.
  Each call returns `Err(fault)` for the unresolved identity, and none
  panics.

## Status

`✅ Passed locally`. Steps 1 to 3:
`s6a_family_kind_admits_no_relation_and_family_outcome_has_two_arms`, in
`src/value/expression/mod.rs`. Both `FamilyOutcome` arms through the seam
from `CheckedPackage::call`:
`both_family_outcome_arms_reach_a_caller_through_the_s6a_seam`, in
`tests/it/model_reference_queries.rs`. The seam-probe list names
`evaluate_declaration`, `S6aFamilyKind::family` and
`ValueFunctionFamily::evaluate`, and `S6aFamilyKind` and `FamilyOutcome`
each have a `#[cfg(seam_probe)]` variant. A compile-time check in
`src/value/expression/s6a.rs` rejects an S6a family kind that maps to
`FamilyKind::Relation` or shares a `FamilyKind` with another.
