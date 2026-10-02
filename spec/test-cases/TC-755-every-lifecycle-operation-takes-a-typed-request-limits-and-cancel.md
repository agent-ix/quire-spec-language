---
id: TC-755
title: "Every lifecycle operation takes a typed request, limits and Cancel, and rejects wrong-stage input at compile time"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-275
    type: verifies
---
# TC-755: Every lifecycle operation takes a typed request, limits and Cancel, and rejects wrong-stage input at compile time

## Description

Verify the eleven QSL lifecycle operations share the FR-275 call shape: each takes its typed request, its limits value and `&Cancel`; each consuming operation takes its predecessor's own type; and each is a function of its request and limits.

Scope: FR-275-AC-1 to FR-275-AC-3, FR-275-AC-5.

## Test Procedure

1. Call `parse` on `tests/fixtures/spine-compile.native` with default limits and `Cancel::new()`, then `format` over the `ParsedSource`, `select` with no domain packages, `check` and `package`.
2. Compare `package`'s bytes with `qsl_replay::spine::compile`'s for the same source.
3. Call `execute` on the `CheckedPackage` selecting `seven` with no arguments and the interpreter backend.
4. Call `inspect` with the package view selector, then `render` the view as text.
5. Compile the `compile_fail` doctests that pass raw bytes to `check`, a `ParsedSource` to `package`, package bytes to `execute` and to `analyze`, and a `ParsedSource` to `monitor`, each beside a compiling control with the correct type.
6. Call each of the eleven operations twice over its step 1 to 4 input (for `analyze`, `monitor` and `replay`, TC-762's, TC-765's and FR-098's fixtures) and compare the outcomes.
7. Read the per-stage work counters from the accounting of step 1's `check`, and of `package`, `execute` and `monitor` (TC-765's trace) over step 1's `CheckedPackage`.

Tag the tests `#[trace("TC-755", "<AC id>")]`.

## Expected Results

- Step 1: each call returns `Ok(Staged<_>)`.
- Step 2: the bytes are equal.
- Step 3: the call completes with the integer 7.
- Step 4: both return `Ok(Staged<_>)`.
- Step 5: every doctest fails to compile and every control compiles.
- Step 6: each pair of outcomes is equal.
- Step 7: `check` reports zero S1 and S2 work; `package`, `execute` and `monitor` each report zero work for S1 to S4.
