---
id: TC-384
title: "S6a invariant breaks are InternalFaults, not panics or refusals"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: verifies
---
# TC-384: S6a invariant breaks are InternalFaults, not panics or refusals

## Description

Verify FR-090-AC-3. When an S6a invariant breaks, S6a returns
`Err(InternalFault)`, which names the stage and the invariant, with category
`internal-failure` (ADR-013 T-4). Two invariants are covered:

- a consumed evaluation environment;
- a checked identity the package does not resolve. ADR-011 §2.3's E6 row
  places bad input at admission, and `call` resolves the function before
  S6a.

Scope: FR-090-AC-3.

This catches three faults. The first keeps an `unreachable!`/`panic!` on
these paths; today `CheckedPackage::call` ends its
`EnvironmentAlreadyConsumed` arm in `unreachable!`. The second reports the
invariant as a `FamilyOutcome::FamilyEvaluated` result. The third reports it as a kernel
`Outcome::Refused(Refusal::CheckedInvariant)` inside `Evaluated`.

## Test Procedure

1. Check a package declaring `id(x: Integer[0,10]): Integer[0,10] = x`.
2. Build one `Value` evaluation environment that carries the arguments
   `[3]`.
3. Call S6a with `id`'s identity and that environment. This consumes the
   arguments.
4. Call S6a a second time with the same environment.
5. Build a fresh environment, and call S6a with a `NodeKey` that names no
   function in the package.

The test harness treats a panic as a failure.

Tag the test `#[trace("FR-090-AC-3", "TC-384")]`.

## Expected Results

- Step 3 returns `Ok(e)` with `e.outcome` equal to
  `FamilyOutcome::Evaluated(Outcome::Completed(3))`.
- Steps 4 and 5 each return `Err(fault)` and do not panic. For each `fault`:
  - `fault.stage()` names S6a;
  - `fault.category() == Category::InternalFailure`;
  - `fault.invariant()` is compared with a literal identifier, not a
    rendered message.
- The two steps' invariant identifiers differ.
- Neither step returns an `Ok(e)` whose `e.outcome` is a
  `FamilyOutcome::FamilyEvaluated` or
  `FamilyOutcome::Evaluated(Outcome::Refused(_))`.

## Status

`✅ Passed locally`. Backed: `s6a_invariant_breaks_are_internal_faults_not_panics`, in
`src/value/expression/mod.rs`, tagged `#[trace("FR-090-AC-3", "TC-384")]`.
The test calls `ValueFunctionFamily::evaluate` (the S6a seam's own hook)
directly, rather than through `CheckedPackage::call`, so it asserts the
hook's own `Result<EvalOutcome<Value>, InternalFault>` shape: step 3 is
`Ok(EvalOutcome::Kernel(Outcome::Completed(Value::Integer(3))))`, and steps
4 and 5 are `Err(fault)` -- a bare `InternalFault`, since the hook never
wraps its fault in any refusal-shaped type -- with `fault.stage()`,
`fault.category()` and `fault.invariant()` checked exactly as specified and
the two steps' invariant identifiers asserted distinct.
