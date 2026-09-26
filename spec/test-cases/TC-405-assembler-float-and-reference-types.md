---
id: TC-405
title: "The assembler admits floating types and refuses unresolved model references"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-405: The assembler admits floating types and refuses unresolved model references

## Description

Verify the explicit resolution arms for `Float32`/`Float64` and
`Reference<Q>`. A floating type is admitted and carries its rounding mode
(FR-091-OQ-4, ADR-013 R-07), and a floating type written without a mode is
admitted by S1 and read as mode `exact` (QSpec FR-148). A model reference with no admitted domain package is
an unresolved name.

Scope: FR-091-AC-19, FR-091-AC-23.

## Test Procedure

Every fixture unit below starts with the complete-V1 header
(`language "ix:native" edition "1-draft";`) and one profile selection whose
alias is `v`.

1. Assemble a unit with `function f using v(x: Float64[nearest-even]): Boolean pure { true }`, and evaluate `f + g` over `Float64[mode]` parameters under each of the six modes.
2. Parse and assemble a unit with
   `function h using v(x: Float64): Boolean pure { true }`. Confirm
   `is_admissible()` after S1, and read the parameter's type form after S2.
3. Assemble a unit, with no `model` selection and no admitted domain
   package, holding `function g using v(r: Reference<M::T>): Boolean pure { true }`.
4. Spine-compile, with no library supplied, a unit that declares
   `import "test/units" version "2" digest "<64 lowercase hex>" as u;`
   (FR-091-AC-24).

Tag the tests `#[trace("FR-091-AC-19", "FR-091-AC-23", "TC-405")]`, and step 4's `#[trace("TC-405", "FR-091-AC-24")]`.

## Expected Results

- Step 1 assembles; the parameter's type resolves to a `ValueType::Float`
  of width `Float64` and rounding mode `nearest-even`, and a `Float64[mode]`
  `+` is evaluated under that mode.
- Step 2's source is admissible, its type form has head `Float64` and no
  mode, and it assembles and resolves to rounding mode `exact`.
- Step 3 gives an unresolved-type-name error naming `M::T`.
- Step 4 refuses at stage `intake`, before assembly, with
  `missing_import`/`missing-selection` naming `test/units`, located at the
  import's identity string.
- Step 3 returns no `PackageDeclarations` value.

## Status

Step 4 is backed by `qsl-replay` `spine::tests::an_import_no_dependency_input_supplies_refuses` (QSL-255 part b). Steps 1 and 3 are backed by `qsl-semantics` `check::assemble` tests: `floating_types_are_admitted_and_reference_types_are_refused`; resolution of the mode is `check::type_form` `float_types_carry_their_rounding_mode`, and evaluation under each mode is `qsl-eval` `float_rounding::the_evaluator_applies_the_rounding_mode_the_float_type_carries` (QSL-280). Step 2 is backed by those and by `qsl-forms` `a_bare_float_type_is_admitted_and_builds_with_no_rounding_mode` (bare `Float64` and `Float32`, no rounding mode on the type form).
