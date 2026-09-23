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
outcome as `Ok(e)` with `e.outcome` equal to `FamilyOutcome::Evaluated(o)`,
and each `o` is a fixed, literal expected outcome. The hook's
`Ok(EvalOutcome::Kernel(Outcome::Incomplete(i)))` becomes
`Evaluated(Outcome::Incomplete(i))`. Scope: FR-090-AC-1.

The fixture uses an IEEE-to-rational conversion because the checker's
definedness pass puts no obligation on it: `qsl-semantics/src/check/facts.rs` raises
obligations only for `Nonzero`, `Presence`, integer `Range`, `RationalRange`
and `NonemptyReduction`. The same conversion already runs through a checked
package in `qsl-eval/tests/it/total_functions.rs`
(`p10_stable_paths_ieee_conversion_references_duplicates_and_node_limits`),
giving `Undefined(IeeeNotFinite)` for NaN, `Refused(IeeeRationalOutOfDomain)`
for 20.0 and `Completed(1/2)` for 0.5. A zero divisor or an out-of-domain
integer result would be refused at check time and never reach S6a.

This catches three faults. The first is a wrapper that turns a kernel
`Refused` into a `FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(_))`,
which conflates a kernel refusal with a family refusal. The second reports the hook's meter-budget
`Incomplete` as a refusal or as an `InternalFault`. The third rewrites a
kernel payload on the way out.

## Test Procedure

1. Check a package declaring
   `f(x: Float[binary64]): Rational[-9..9 / 1..9] = convert(x)` and link it.
   Confirm the check admits it.
2. Call S6a for `f` with `x = 0.5` (`0x3FE0_0000_0000_0000`) and an unlimited
   meter.
3. Call S6a for `f` with `x = NaN` (`0x7FF8_0000_0000_0000`) and an unlimited
   meter.
4. Call S6a for `f` with `x = 20.0` (`0x4034_0000_0000_0000`) and an
   unlimited meter.
5. Call S6a for `f` with `x = 0.5` and a meter whose work limit is zero, so
   the hook's `ChargePoint::FunctionCall` charge is denied.

Tag the test `#[trace("FR-090-AC-1", "TC-382")]`.

## Expected Results

Each result is compared with a fixed literal outcome, not with another run
of the hook. Each step returns `Ok(e)`. `FamilyOutcome` has no `Eq`, so the
test matches `e.outcome`'s `Evaluated` arm and compares the kernel `Outcome`
it holds with `assert_eq!`.

- Step 2: `e.outcome` is `FamilyOutcome::Evaluated(Outcome::Completed(Value::Rational(1/2)))`, `e.location` is `None`, and `e.losses` is empty.
- Step 3: `e.outcome` is `FamilyOutcome::Evaluated(Outcome::Undefined(Undefined::IeeeNotFinite))`, `e.location` is `Some`, and `e.losses` is empty.
- Step 4: `e.outcome` is `FamilyOutcome::Evaluated(Outcome::Refused(Refusal::IeeeRationalOutOfDomain))`, `e.location` is `Some`, and `e.losses` is empty.
- Step 5: `e.outcome` is `FamilyOutcome::Evaluated(Outcome::Incomplete(i))`,
  where `i`'s charge point is `ChargePoint::FunctionCall`.
- No step returns a `FamilyOutcome::FamilyEvaluated` or `Err(_)`.

## Status

`✅ Passed locally`. Backed: `s6a_returns_kernel_outcomes_unchanged_in_evaluated`,
in `qsl-eval/tests/it/total_functions.rs`, through `CheckedPackage::call`. It also
calls a rounding decimal division and asserts the one loss `call` returns.
