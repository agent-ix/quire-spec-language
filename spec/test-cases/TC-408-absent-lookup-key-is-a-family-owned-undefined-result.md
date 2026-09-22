---
id: TC-408
title: "An absent lookup key reaches the caller as a StateModel undefined result, and an absent-refused lookup as a refusal"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: verifies
---
# TC-408: An absent lookup key reaches the caller as a StateModel undefined result, and an absent-refused lookup as a refusal

## Description

Verify FR-090-AC-12. Suppose a `lookup<T>(p, r) absent undefined` query
names a reference whose key is not a member of the population bound to `p`.
The caller then receives an `Evaluation` whose `outcome` is
`FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause))` and whose
`cause.undefined_record()` has reason `absent-key` and the catalog payload,
category `undefined`. The same query with `absent refused` receives
`FamilyResult::Refused` with code `invalid_runtime_input`/`absent-key`,
category `refusal` (QSpec TC-198 L03). The kernel `Undefined` has no
`AbsentKey` variant (ADR-013 O-13, O-16). Scope: FR-090-AC-12.

The fixture is `tests/it/model_reference_queries.rs`'s
`l14_lookup_expression_undefined_mode` scenario (QSpec TC-198 L03): a
population argument `p: Population<A>[3]` and a reference `c9` of type `A`
that is not a member. That test asserts `Outcome::Undefined(
Undefined::AbsentKey)`, which is the kernel carrier ADR-013 O-16 removes.

This catches four faults: keeping `absent-key` as a kernel `Undefined`
reason with no payload; carrying it in `FamilyResult::Refused`, which
collapses category `undefined` into `refusal`; dropping the binding or key
from the payload; and returning the undefined result for an `absent refused`
query, which erases the refusal that query asks for.

## Test Procedure

1. Reuse the `l14_lookup_expression_undefined_mode` package and population
   argument.
2. Evaluate the checked expression `lookup<A>(p, r) absent undefined` with
   `r` naming `c9` through `CheckedPackage::evaluate`, with an unlimited
   meter.
3. Match the result as `Ok(e)` and `e.outcome` as
   `FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause))`, and take
   `record = cause.undefined_record()`.
4. Evaluate `lookup<A>(p, r) absent refused` with the same arguments through
   `CheckedPackage::evaluate`, match `Ok(e)` and `e.outcome` as
   `FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause))`, and take
   `cause.catalog_code()`.
5. Inspect `quire-exact/src/outcome.rs`'s `Undefined` enum.

Tag the test `#[trace("FR-090-AC-12", "TC-408")]`.

## Expected Results

- Step 3's `record.reason` is `UndefinedReason` `absent-key`, and
  `record.fields` name the population binding bound to `p` and the requested
  key `c9`.
- Step 2 does not panic, and its result matches step 3's pattern;
  `e.outcome` is not `FamilyOutcome::Evaluated(Outcome::Undefined(_))` and
  not `FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(_))`.
- Step 4's code is
  `CatalogCode::new("invalid_runtime_input", "absent-key")`.
- Step 5 finds no `AbsentKey` variant.

## Status

Planned; no test backs this case.
