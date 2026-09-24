---
id: TC-433
title: "Model limits have finite defaults, and normalization work stops at the limit"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-012
    type: verifies
---
# TC-433: Model limits have finite defaults, and normalization work stops at the limit

## Description

Verify NFR-012:

- `ModelNormalizationLimits` and `PopulationAdmissionLimits` have finite
  defaults equal to NFR-012's values.
- A completed view records the limits it was normalized under.
- Every normalization fixture has the same outcome at the defaults as
  without a limit.
- A refused normalization's work is bounded by the limit, not the package.
- A production meter allocates nothing per charge.

## Test Procedure

1. Compare `ModelNormalizationLimits::default()` and
   `PopulationAdmissionLimits::default()` with NFR-012's table. Normalize a
   small package at the defaults and at a raised work ceiling, and read
   `EffectiveView::effective_limits`.
2. Normalize every fixture in `qsl-semantics/tests/it/model_normalization.rs`
   at the defaults and without a limit, and compare the outcomes.
3. Normalize type `A` with `n` fields and type `B` generalizing `A`, for
   `n` = 100 and 1000:
   - at `effective_declarations` 6, counting member identity hashes;
   - at `derivation_facts` `2 + n + 3`, counting inherited members.
4. Normalize a lattice of two types per level, each generalizing both types
   of the level above, 10 and 18 levels deep, at `derivation_facts` 200,
   counting ancestor-walk steps.
5. Without `test-support`, charge a model `Meter` and an `AdmissionMeter`
   100000 times, and check that neither they nor their charges have drop
   glue.

## Expected Results

1. The defaults equal NFR-012's values, and each view records the limits it
   ran under.
2. Every fixture's view identity and declarations, or its refusals, are the
   same at the defaults as without a limit.
3. Both sizes are incomplete at the same charge point and counter. The
   declaration run hashes 4 member identities and the fact run takes 4
   steps, for either `n`.
4. Both lattices are incomplete on `derivation_facts` after at most 200
   walk steps, although the deeper one has 2^18 paths.
5. The admission count equals the number of charges, and no meter or charge
   type owns heap memory.
