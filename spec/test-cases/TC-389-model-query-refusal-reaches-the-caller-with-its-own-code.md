---
id: TC-389
title: "A refused model query reaches the caller with the ModelRefusal's own catalog code, not a kernel refusal"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: verifies
---
# TC-389: A refused model query reaches the caller with the ModelRefusal's own catalog code, not a kernel refusal

## Description

Verify FR-090-AC-8. Suppose `model::population` refuses an `allInstances<T>(p)`
query with a `ModelRefusal`. The caller then receives
`FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause))` whose
`cause.catalog_code()` is that refusal's `ModelRefusal::catalog_code()`, the
method O-17 requires and which does not exist today. The result is not a
kernel `Outcome::Refused` inside `FamilyOutcome::Evaluated`, not a dispatch
`FamilyOutcome::Refused` and not a panic. The carried refusal holds no
native-v1 `qsl_foundation::diagnostic::Code` (ADR-013 R-09), and
`FamilyRefusal` has no variant naming `ModelRefusal` (ADR-013 O-16).
Scope: FR-090-AC-8.

The fixture is the above-maximum refusal of `all_instances`
(`src/model/population.rs`, `Code::CardinalityOutOfBound` with
`ModelRefusalCause::AboveMaximum`, cause tag `above-maximum`).

This catches three faults: carrying the refusal as QSL's
`Refusal::Model(ModelQueryRefusal)` kernel-copy variant, which T-6 removes;
keeping `ModelQueryRefusal`'s `code: Code` field; and reporting the model
refusal as kernel `Refusal::CardinalityOutOfBound`, which would conflate a
model-query refusal with a kernel collection-bound refusal.

## Test Procedure

1. Admit a closed population `p: Population<A>[1]` whose binding has two
   members of type `A`.
2. Evaluate the checked expression `allInstances<A>(p)` through
   `CheckedPackage::evaluate`, with an unlimited meter.
3. Match the result as
   `Ok(FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause)))` and
   take `cause.catalog_code()`.
4. Inspect the definition of `ModelRefusal` and of the `check` core's
   `FamilyRefusal`.

Tag the test `#[trace("FR-090-AC-8", "TC-389")]`.

## Expected Results

- Step 3's code is
  `CatalogCode::new("cardinality_out_of_bound", "above-maximum")`, and it
  equals `ModelRefusal::catalog_code()` of the refusal `all_instances`
  returns for the same binding.
- Step 2 does not panic, and its result matches step 3's pattern; it is not
  `Ok(FamilyOutcome::Evaluated(Outcome::Refused(_)))` and not
  `Ok(FamilyOutcome::Refused(_))`.
- Step 4 finds no `ModelRefusal` field of type
  `qsl_foundation::diagnostic::Code`, and no `FamilyRefusal` variant whose
  payload is `ModelRefusal`.

## Status

Planned; no test backs this case.
