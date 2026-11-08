---
id: TC-760
title: "execute returns what CheckedPackage::call and evaluate return"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-279
    type: verifies
---
# TC-760: execute returns what CheckedPackage::call and evaluate return

## Description

Verify `execute` with the interpreter backend against S6a and FR-100's command.

Scope: FR-279-AC-1 to FR-279-AC-5.

## Test Procedure

1. Over FR-100-AC-4's unit, call `execute` and `CheckedPackage::call` for `lt(5, 3)`, `flag(1)`, `id(4)` and `id(12)`.
2. Over FR-109's ConfigVersion unit, for the FR-109-AC-1 healthy-parent and violating-parent requests, admit each request's snapshot for `ParentOrder` by FR-106 against the checked package; call `execute` with the interpreter and a clause selection naming `ParentOrder` with those `AdmittedObservations`, and `CheckedPackageEvaluation::evaluate_clause` with the same clause, observations and accounting limits; map `execute`'s `Evaluation` by FR-100's outcome mapping and run `qsl_replay::spine::run_clause` over the same request.
3. For FR-100-AC-1, AC-4 and AC-6's requests, compare the command's `outcome` member with the rendering of `execute`'s result.
4. Call `execute` choosing a supplied test `aot` backend whose `prepare` returns `PrepareFailure::Limit` naming `prepare.code_bytes` with configured value 1 and counter 2, then one whose `prepare` returns `PrepareFailure::Fault`.
5. Compile the `compile_fail` doctest that builds `ExecuteLimits` with `..Default::default()` and no `accounting`, beside a control stating `accounting` with `SuppliedLimits::default()` and `PrepareLimits::default()`. Over FR-100-AC-4's unit, call `execute` with the interpreter for `id(4)` under `supplied.admission_work_units` 0, and `CheckedPackageEvaluation::call_supplied` under a supplied-value run with the same `SuppliedLimits`; repeat `execute` under `SuppliedLimits::default()`.

Tag the tests `#[trace("TC-760", "<AC id>")]`.

## Expected Results

- Step 1: the results are equal pairwise, including `id(12)`'s `CallFailure::Input`.
- Step 2: for each request the two `Evaluation`s are equal; the mapped outcomes are `Completed(Boolean(true))` for healthy-parent and `Completed(Boolean(false))` for violating-parent, each equal to `run_clause`'s `Evaluate` disposition.
- Step 3: each pair is equal.
- Step 4: `CallFailure::Limit(PrepareLimitReached { setting: CodeBytes, bound: 1, actual: 2 })`, setting name `prepare.code_bytes`, category incomplete, exit 22, then `CallFailure::Fault` with the same `InternalFault`.
- Step 5: the doctest fails to compile and the control compiles; `execute` and `call_supplied` return the same `CallFailure::Supplied`; under the default `SuppliedLimits`, `execute` returns step 1's `Evaluation` for `id(4)`.
