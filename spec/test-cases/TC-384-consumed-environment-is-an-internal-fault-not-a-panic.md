---
id: TC-384
title: "A consumed evaluation environment is an InternalFault, not a panic or a refusal"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: verifies
---
# TC-384: A consumed evaluation environment is an InternalFault, not a panic or a refusal

## Description

Verify FR-090-AC-3. When an S6a invariant breaks, S6a returns
`Err(InternalFault)`, which names the stage and the invariant. The fault's
category is `internal-failure` and its catalog code is
`runtime_invariant`/`established-invariant-broken` (ADR-013 T-4). The
invariant used here is the consumed evaluation environment. Scope:
FR-090-AC-3.

This catches three faults. The first keeps an `unreachable!`/`panic!` on
this path; today `CheckedPackage::call` ends its
`EnvironmentAlreadyConsumed` arm in `unreachable!`. The second reports the
invariant as `FamilyOutcome::Refused`. The third reports it as a kernel
`Outcome::Refused(Refusal::CheckedInvariant)` inside `Evaluated`.

## Test Procedure

1. Check a package declaring `id(x: Integer[0,10]): Integer[0,10] = x`.
2. Build one `Value` evaluation environment that carries the arguments
   `[3]`.
3. Call S6a with `id`'s identity and that environment. This consumes the
   arguments.
4. Call S6a a second time with the same environment. The test harness
   treats a panic as a failure.

Tag the test `#[trace("FR-090-AC-3", "TC-384")]`.

## Expected Results

- Step 3 returns `Ok(FamilyOutcome::Evaluated(Outcome::Completed(3)))`.
- Step 4 returns `Err(fault)` and does not panic. For that `fault`:
  - `fault.stage()` names S6a;
  - `fault.invariant()` is the stable identifier the implementation gives
    the consumed-environment invariant. The test compares it with a literal
    string, not with a rendered message;
  - `fault.category() == Category::InternalFailure`;
  - `fault.catalog_code() == CatalogCode::new("runtime_invariant", "established-invariant-broken")`.
- Step 4's result is neither `Ok(FamilyOutcome::Refused(_))` nor
  `Ok(FamilyOutcome::Evaluated(Outcome::Refused(_)))`.
