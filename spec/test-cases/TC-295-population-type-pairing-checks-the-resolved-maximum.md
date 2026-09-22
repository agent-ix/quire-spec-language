---
id: TC-295
title: "ValueType::Population admits a Value::Population identity by its resolved binding's declared maximum"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-089
    type: verifies
---
# TC-295: ValueType::Population admits a Value::Population identity by its resolved binding's declared maximum

## Description

Verify FR-089-AC-5: `ValueType::admits` treats `ValueType::Population(maximum)`
as admitting a `Value::Population(population_id)` exactly when the resolved
binding's own declared maximum equals `maximum`, restoring the pairing
`src/value/composite.rs:104` performs today between
`ValueType::Population(maximum)` and `Value::Population(binding)`. Scope:
FR-089-AC-5.

Known gap: today `ValueType::Population(u64)` admits no `Value` variant at
all in the kernel (`quire-exact/src/value.rs:115-116`); the pairing this
test checks exists only in the QSL-layer `value::composite.rs:104` copy, not
in the kernel `ValueType::admits`. This test case fails against current
kernel code. Remaining work: QSL-131 Slice B.

Catches an implementation that has `ValueType::Population(maximum)` admit
every `Value::Population(_)` unconditionally (ignoring the resolved binding's
declared maximum entirely, silently widening the type), and one that compares
`maximum` against something other than the resolved binding's own
`declared_maximum` (for example the population's member count at the moment
of the check, which can differ from its declared bound).

## Test Procedure

1. Admit a `PopulationBinding` with declared maximum `5`, producing
   `Value::Population(population_id)`.
2. Check `ValueType::Population(5).admits(&Value::Population(population_id))`.
3. Check `ValueType::Population(6).admits(&Value::Population(population_id))`.

## Expected Results

Step 2 returns `true`. Step 3 returns `false`. A mutant that admits every
`Value::Population(_)` regardless of `maximum` passes step 2 but also
(incorrectly) passes step 3, failing this test's negative assertion.
