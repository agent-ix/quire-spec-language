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

Planned; no test backs this case.
