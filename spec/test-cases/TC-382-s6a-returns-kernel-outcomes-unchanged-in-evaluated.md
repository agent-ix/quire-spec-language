---
id: TC-382
title: "S6a returns each kernel outcome unchanged in FamilyOutcome::Evaluated"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: verifies
---
# TC-382: S6a returns each kernel outcome unchanged in FamilyOutcome::Evaluated

## Description

Verify FR-090-AC-1. For a checked `Value` function, S6a returns every kernel
outcome it evaluates to in `Ok(FamilyOutcome::Evaluated(o))`, with `o` equal in
variant and payload to the `quire_exact::Outcome` the family's `evaluate`
hook produced. Scope: FR-090-AC-1.

This catches three faults. The first is a wrapper that turns a kernel
`Refused` into a `FamilyOutcome::Refused`, which conflates a kernel refusal
with a family refusal. The second reports a meter-budget `Incomplete` as a
refusal or as an `InternalFault`. The third rewrites a kernel payload on the
way out, for example by dropping `IeeeNotExact`'s flags or a
`CardinalityOutOfBound`'s count.

## Test Procedure

1. Check a package declaring four `Value` functions:
   - `id(x: Integer[0,10]): Integer[0,10] = x`;
   - `div(x: Integer[0,10], y: Integer[0,10]): Rational[..] = x / y`;
   - `inc(x: Integer[0,10]): Integer[0,10] = x + 1`;
   - any one-call function, for the meter case.
2. Evaluate `id(3)` through S6a with an unlimited meter.
3. Evaluate `div(1, 0)` through S6a with an unlimited meter.
4. Evaluate `inc(10)` through S6a with an unlimited meter. The result `11`
   is outside the declared `Integer[0,10]` domain.
5. Evaluate the one-call function through S6a with a meter whose work limit
   is zero, so the call's first charge is denied.
6. For each step, run the same evaluation directly through
   `ValueFunctionFamily::evaluate` and keep the kernel outcome it produces.
   Compare that outcome with the S6a result.

Tag the test `#[trace("FR-090-AC-1", "TC-382")]`.

## Expected Results

- Step 2 returns `Ok(FamilyOutcome::Evaluated(Outcome::Completed(3)))`.
- Step 3 returns `Ok(FamilyOutcome::Evaluated(Outcome::Undefined(Undefined::DivisionByZero)))`.
- Step 4 returns `Ok(FamilyOutcome::Evaluated(Outcome::Refused(r)))`, where
  `r` is the kernel cause (`Refusal::IntegerOutOfDomain`).
- Step 5 returns `Ok(FamilyOutcome::Evaluated(Outcome::Incomplete(i)))`,
  where `i` names the denied charge point and limit.
- In every step, the payload inside `Evaluated` equals the step-6 kernel
  outcome under `PartialEq`.
- No step returns `Ok(FamilyOutcome::Refused(_))` or `Err(_)`.
