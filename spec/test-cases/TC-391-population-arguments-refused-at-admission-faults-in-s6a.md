---
id: TC-391
title: "An unresolved or mismatched population argument is refused at admission, and is an InternalFault inside S6a"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: verifies
---
# TC-391: An unresolved or mismatched population argument is refused at admission, and is an InternalFault inside S6a

## Description

Verify FR-090-AC-10. `CheckedPackage::call` admits arguments before S6a and
refuses a bad `Population<T>[N]` argument as `CallFailure::Input(_)`
(ADR-011 §2.3 E6 row; ADR-013 T-4). The same condition reached inside S6a is
an invariant break, so it returns `Err(InternalFault)` rather than a kernel
or family refusal. Scope: FR-090-AC-10.

Today the admission half is FR-089-AC-4 and AC-5 (`validate`, TC-294 and
TC-295), which refuse with `InputRefusal::WrongValueKind`. The evaluator
half is today's `Refusal::UnresolvedPopulation` and
`Refusal::PopulationMaximumMismatch`, neither of which is a `quire_exact`
variant.

This catches two faults: an S6a path that reports an unresolvable
population identity as a kernel refusal, and a `call` that lets such an
argument reach S6a.

## Test Procedure

1. Check a package declaring a `Value` function with a parameter
   `p: Population<A>[3]` whose body reads `size(allInstances<A>(p))`.
2. Call `CheckedPackage::call` with a `Value::Population(id)` whose `id` was
   minted in a separate evaluation and is not recorded in this object
   environment.
3. Call `CheckedPackage::call` with a population admitted with declared
   maximum 2.
4. Repeat step 2's argument, and then step 3's, passing each directly to the
   S6a seam without admission.

Tag the test `#[trace("FR-090-AC-10", "TC-391")]`.

## Expected Results

- Steps 2 and 3 each return `Err(CallFailure::Input(_))`, and the meter
  records no charge.
- Step 4 returns `Err(fault)` for both arguments, with
  `fault.category() == Category::InternalFailure`. It never returns
  `Ok(FamilyOutcome::Evaluated(Outcome::Refused(_)))` or
  `Ok(FamilyOutcome::Refused(_))`, and it does not panic.

## Status

Backed, in two parts (`FamilyOutcome`/`FamilyRefusal` are not yet built --
FR-090-OQ-2, OQ-3 remain open -- so the S6a-internal half asserts today's
real `EvaluateFailure`/`InternalFault` types rather than the literal
`FamilyOutcome`/kernel-`Refused` spelling above):

- Steps 1-3 (admission, `CheckedPackage::call`):
  `tc_391_call_refuses_an_unresolved_population_id_at_admission` and
  `tc_391_call_refuses_a_population_maximum_mismatch_at_admission`, in
  `tests/it/model_reference_queries.rs`, each tagged
  `#[trace("TC-391", "FR-090-AC-10")]`, asserting `Err(CallFailure::
  Input(InputRefusal::WrongValueKind { .. }))` and an empty meter.
- Step 4 (bypassing admission, called directly against the S6a seam,
  `ValueFunctionFamily::evaluate`, rather than `Machine::
  resolve_population` in isolation):
  `evaluate_bypassing_admission_with_an_unresolved_population_id_is_an_internal_fault`
  and
  `evaluate_bypassing_admission_with_a_population_maximum_mismatch_is_an_internal_fault`,
  in `src/value/expression/evaluate.rs`, each tagged
  `#[trace("FR-090-AC-10", "TC-391")]`, asserting `Err(crate::family::
  EvaluateFailure::Fault(fault))` with `fault.category() ==
  Category::InternalFailure` and the two conditions' own distinct invariant
  identifiers, never a panic. Proven by mutation (PR #334 review round 2,
  finding F2/N1): deleting the interception that keeps a fault out of the
  shared `Stop`/`Outcome` path makes both tests fail -- by a compile error
  once the fault can no longer be represented as a `Stop` at all, not by a
  panic.
