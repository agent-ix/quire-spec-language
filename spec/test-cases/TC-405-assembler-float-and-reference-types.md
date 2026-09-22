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
mode (ADR-013 R-07). A model reference with no admitted domain package is
an unresolved name.

Scope: FR-091-AC-19.

## Test Procedure

Every fixture unit below starts with the complete-V1 header
(`language "ix:native" edition "1-draft";`) and one profile selection whose
alias is `v`.

1. Assemble a unit with `function f using v(x: Float64[nearest-even]): Boolean pure { true }`.
2. Assemble a unit, with no `model` selection and no admitted domain
   package, holding `function g using v(r: Reference<M::T>): Boolean pure { true }`.

Tag the test `#[trace("FR-091-AC-19", "TC-405")]`.

## Expected Results

- Step 1 gives a floating-type error that names `nearest-even` and the type
  form's span.
- Step 2 gives an unresolved-type-name error naming `M::T`.
- Neither step returns a `PackageDeclarations` value.

## Status

Planned; no test backs this case.
