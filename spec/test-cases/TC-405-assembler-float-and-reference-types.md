---
id: TC-405
title: "The assembler refuses floating types and unresolved model references"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-405: The assembler refuses floating types and unresolved model references

## Description

Verify the explicit resolution arms for `Float32`/`Float64` and
`Reference<Q>`. A floating type refuses rather than dropping its rounding
mode (ADR-013 R-07), and a floating type written without a mode is
admitted by S1 and read as mode `exact` (QSpec FR-148). A model reference with no admitted domain package is
an unresolved name.

Scope: FR-091-AC-19, FR-091-AC-23.

## Test Procedure

Every fixture unit below starts with the complete-V1 header
(`language "ix:native" edition "1-draft";`) and one profile selection whose
alias is `v`.

1. Assemble a unit with `function f using v(x: Float64[nearest-even]): Boolean pure { true }`.
2. Parse and assemble a unit with
   `function h using v(x: Float64): Boolean pure { true }`. Confirm
   `is_admissible()` after S1, and read the parameter's type form after S2.
3. Assemble a unit, with no `model` selection and no admitted domain
   package, holding `function g using v(r: Reference<M::T>): Boolean pure { true }`.

Tag the test `#[trace("FR-091-AC-19", "FR-091-AC-23", "TC-405")]`.

## Expected Results

- Step 1 gives a floating-type error, code
  `unknown_required_feature`/`unsupported-feature`, that names `Float64`,
  `nearest-even`, the profile selection `v` and the type form's span.
- Step 2's source is admissible, its type form has head `Float64` and no
  mode, and the assembler gives the same floating-type error naming
  `exact`.
- Step 3 gives an unresolved-type-name error naming `M::T`.
- No step returns a `PackageDeclarations` value.

## Status

Steps 1 and 3 are backed by `qsl-semantics` `check::assemble` tests: `floating_and_reference_types_are_refused`. Step 2 is not: `qsl-cst` still requires `[mode]` on `Float32` and `Float64` (FR-091-AC-23).
