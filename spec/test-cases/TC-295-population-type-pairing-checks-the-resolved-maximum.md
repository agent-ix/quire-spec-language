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
`CheckedPackage::call`/`evaluate`'s own argument-admission `validate`
(`src/value/expression/mod.rs`) performs between
`ValueType::Population(maximum)` and `Value::Population(binding)`. The kernel
`ValueType::admits` refuses every population pair (FR-089-AC-6, TC-297); this
comparison is the QSL layer's. Scope: FR-089-AC-5.

Implemented (QSL-131 V5): `ValueType::admits` is `quire_exact`'s own kernel
function now, which has no access to the recorded correspondence and refuses
every `(Population, Population)` pair outright (FR-089-AC-6), so `validate`
special-cases `ValueType::Population`/`Value::Population` ahead of its
generic `admits()` call rather than after it, performing the whole
comparison this criterion names itself: it does receive the environment,
resolves `population_id` through it, and refuses
(`InputRefusal::WrongValueKind`) when the resolved binding's declared
maximum differs from the checked parameter's `maximum` -- whether or not
the checked body actually consumes the parameter (PR #326 review finding
F1). The evaluator's own `Machine::resolve_population`
(`src/value/expression/evaluate.rs`'s `AllInstances`/`Lookup` sites)
performs the identical comparison and is exercised by this test case's own
consumed-parameter procedure; it is unreachable through either public entry
point for a checked program once `validate` already refuses first, and is
kept only as defence in depth.

Catches an implementation that has `ValueType::Population(maximum)` admit
every `Value::Population(_)` unconditionally (ignoring the resolved binding's
declared maximum entirely, silently widening the type), one that compares
`maximum` against something other than the resolved binding's own
`declared_maximum` (for example the population's member count at the moment
of the check, which can differ from its declared bound), and one that only
performs the comparison when the checked body actually consumes the
parameter.

## Test Procedure

1. Admit a `PopulationBinding` with declared maximum `5`, producing
   `Value::Population(population_id)`.
2. Evaluate a checked call whose parameter is declared
   `ValueType::Population(5)`, bound to `population_id`, consuming it
   (`tc_295_population_type_pairing_checks_the_resolved_maximum`).
3. Evaluate the same call shape with the parameter declared
   `ValueType::Population(6)` instead, both consuming it and, separately, a
   variant whose body never reads the parameter at all
   (`tc_295_population_maximum_mismatch_refuses_even_when_unconsumed`).

## Expected Results

Step 2 completes. Step 3 refuses at argument admission
(`InputRefusal::WrongValueKind`) in both the consuming and non-consuming
variants. A mutant that admits every `Value::Population(_)` regardless of
`maximum` passes step 2 but also (incorrectly) completes step 3, failing
this test's negative assertion; a mutant that checks the maximum only when
the body consumes the parameter fails the "never reads it at all" variant.
