---
id: TC-434
title: "Model limits have finite defaults, and normalization work stops at the limit"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-012
    type: verifies
---
# TC-434: Model limits have finite defaults, and normalization work stops at the limit

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
   - at `effective_declarations` 6, counting member identities;
   - at `derivation_facts` `2 + n + 3`, counting inherited members.
4. Normalize a lattice of two types per level, each generalizing both types
   of the level above, 10 and 18 levels deep, at `derivation_facts` 200, and
   the 18-level lattice again at `derivation_facts` 103.
5. Without `test-support`, charge a model `Meter` and an `AdmissionMeter`
   100000 times, and check that neither they nor their charges have drop
   glue.
6. Normalize a 120-deep chain whose root declares 20 fields at the defaults,
   and read the retained facts and each inherited member's inherit fact.
7. Normalize a 10-deep chain at `ancestor_steps` 5, first with
   `declaration_records` 10, then with it unlimited.
8. For each `N` in 1,000, 4,000 and 16,000, construct one selected unit model
   whose `package.records` contains, in order, `N` record-value declarations
   followed by `N` field records. Give each record-value declaration exactly
   one field record that names it as owner. Use no inheritance, subsets or
   redefinitions. Admit the unit at the default limits, read total
   `WorkUnits` from `SelectedModels::consumed(LimitKind::WorkUnits)`, and
   record the three counts and both adjacent ratios on the QSL-1406 tracker
   item.
9. For a one-record-value/one-field input, first normalize the exact admitted
   `DomainPackage` with the same other limits and a high work ceiling; record
   the completed normalization's consumed `WorkUnits` as `W`. Admit the same
   input through unit intake with those limits unchanged except for the work
   ceiling set to `W`; observe the selected-row projection's first denied
   charge.

## Expected Results

1. The defaults equal NFR-012's values, and each view records the limits it
   ran under.
2. Every fixture's view identity and declarations, or its refusals, are the
   same at the defaults as without a limit.
3. Both sizes are incomplete at the same charge point and counter. The
   declaration run admits 4 member identities and the fact run takes 4
   steps, for either `n`.
4. Both lattices complete at 200 with one fact per edge, 56 and 104
   retained facts (two qualify facts per level, four edges per level
   boundary), although the deeper one has 2^17 paths from its bottom type. At 103 the 18-level
   lattice is incomplete on `derivation_facts`.
5. The admission count equals the number of charges, and no meter or charge
   type owns heap memory.
6. The chain normalizes with 2,661 retained facts (241 for the types, 2,420
   for the members: 20 qualify facts at the root and 120 x 20 inherit facts), and every inherited member holds exactly one inherit
   fact and no qualify fact whose parent is the same member's identity in the preceding type.
7. The first run is incomplete on `declaration_records` at
   `normalize.record`; the second is incomplete on `ancestor_steps` naming
   bound 5.
8. All three unit intakes complete. If their consumed work counts are
   `W1000`, `W4000` and `W16000`, then `W4000 <= 5 * W1000` and
   `W16000 <= 5 * W4000`; the three counts and ratios are recorded on
   QSL-1406.
9. Unit intake returns an error whose cause is the `WorkUnits` limit's
   incomplete result, with bound `W` and denied counter `W + 1` at the
   selected-row projection charge. It returns no `SelectedModels` value and
   exposes no partial selected-model result.
