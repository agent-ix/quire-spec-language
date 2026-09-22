---
id: TC-295
title: "The QSL layer admits a Value::Population identity under ValueType::Population by its resolved binding's declared maximum"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-089
    type: verifies
---
# TC-295: The QSL layer admits a Value::Population identity under ValueType::Population by its resolved binding's declared maximum

## Description

Verify FR-089-AC-5: the QSL layer treats `ValueType::Population(maximum)` as
admitting a `Value::Population(population_id)` exactly when the binding that
`population_id` resolves to, through `model`'s recorded correspondence, has a
declared maximum equal to `maximum` -- the pairing
`src/value/composite.rs:104` performs between
`ValueType::Population(maximum)` and `Value::Population(binding)`. The kernel
`ValueType::admits` refuses every population pair (FR-089-AC-6, TC-297); this
comparison is the QSL layer's. Scope: FR-089-AC-5.

Remaining work: QSL-131's other half (QSL `model` minting a `PopulationId`
and the evaluator's resolution step), which this test case needs.

Catches an implementation that has `ValueType::Population(maximum)` admit
every `Value::Population(_)` unconditionally (ignoring the resolved binding's
declared maximum entirely, silently widening the type), and one that compares
`maximum` against something other than the resolved binding's own
`declared_maximum` (for example the population's member count at the moment
of the check, which can differ from its declared bound).

## Test Procedure

1. Admit a `PopulationBinding` with declared maximum `5`, producing
   `Value::Population(population_id)`.
2. Check the QSL-layer admission of `Value::Population(population_id)`
   under `ValueType::Population(5)`, resolving `population_id` through the
   recorded correspondence.
3. Check the same QSL-layer admission under `ValueType::Population(6)`.

## Expected Results

Step 2 returns `true`. Step 3 returns `false`. A mutant that admits every
`Value::Population(_)` regardless of `maximum` passes step 2 but also
(incorrectly) passes step 3, failing this test's negative assertion.
