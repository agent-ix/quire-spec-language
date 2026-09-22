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
query with a `ModelRefusal`. The caller then receives a refusal whose catalog
code equals that `ModelRefusal`'s code and cause. The result is neither a
kernel `Outcome::Refused` inside `FamilyOutcome::Evaluated` nor a panic. The
carried refusal holds no native-v1 `qsl_foundation::diagnostic::Code`.
Scope: FR-090-AC-8.

The fixture is the above-maximum refusal of `all_instances`
(`src/model/population.rs`, `Code::CardinalityOutOfBound` with
`ModelRefusalCause::AboveMaximum`, cause tag `above-maximum`).

This catches three faults: carrying the refusal as QSL's
`Refusal::Model(ModelQueryRefusal)` kernel-copy variant, which T-6 removes;
keeping `ModelQueryRefusal`'s `code: Code` field; and reporting the model
refusal as kernel `Refusal::CardinalityOutOfBound`, which would conflate a
model-query refusal with a kernel collection-bound refusal.

FR-090-OQ-1 decides the carrier the caller reads. The assertions below hold
whichever carrier the ruling picks.

## Test Procedure

1. Admit a closed population `p: Population<A>[1]` whose binding has two
   members of type `A`.
2. Evaluate `allInstances<A>(p)` through the public S6a entry point, with
   an unlimited meter.
3. Read the refusal from the carrier FR-090-OQ-1 rules, and take its
   `catalog_code()`.
4. Inspect the definition of the carried refusal type.

Tag the test `#[trace("FR-090-AC-8", "TC-389")]`.

## Expected Results

- Step 3's code is
  `CatalogCode::new("cardinality_out_of_bound", "above-maximum")`.
- Step 2 does not panic, and its result is not
  `Ok(FamilyOutcome::Evaluated(Outcome::Refused(_)))`.
- Step 4 finds no field of type `qsl_foundation::diagnostic::Code`.
