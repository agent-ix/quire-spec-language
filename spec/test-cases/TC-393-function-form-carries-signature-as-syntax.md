---
id: TC-393
title: "A function form carries its name, using alias, signature type references as syntax, measure and body"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-393: A function form carries its name, using alias, signature type references as syntax, measure and body

## Description

Verify that the `Value` function form carries every part of a
`FunctionDeclaration` CST node, and carries type references as syntax with
their declared bounds as written (ADR-011 §2.2 E2 row).

This catches a production that resolves or types a bound at S2, one that
drops the `using` alias or the `decreases` measure, and one that needs a
`NodeKey` to represent a named parameter type.

Scope: FR-091-AC-2.

## Test Procedure

1. Build an admissible unit whose only declaration is
   `function inc using v(x: Int[0, 9], y: Digit): Int[0, 10] pure decreases(x) { x + 1 }`.
   `Digit` is deliberately undeclared: S2 does not resolve names.
2. Run the S2 entry and take the one function form.
3. Read the name, the `using` alias, each parameter's name and type
   reference, the result type reference, the measure and the body.
4. Add a `compile_fail` doctest that calls an accessor returning `ValueType`
   or `NodeKey` on the function form.

Tag the test `#[trace("FR-091-AC-2", "TC-393")]`.

## Expected Results

- Name `inc`; `using` alias `v`.
- Parameter `x`: type reference with constructor `Int` and bounds spelled
  `0` and `9`. Parameter `y`: type reference with qualified name `Digit`.
- Result: constructor `Int` with bounds spelled `0` and `10`.
- Measure `Name("x")`; body `Binary{Add, Name("x"), Integer(1)}`.
- Step 4 fails to compile.

## Status

Planned; no test backs this case.
