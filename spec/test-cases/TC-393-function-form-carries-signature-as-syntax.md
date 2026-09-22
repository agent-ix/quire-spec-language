---
id: TC-393
title: "The forms FunctionDeclaration carries its name, using alias, type forms, measure and body"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-393: The forms FunctionDeclaration carries its name, using alias, type forms, measure and body

## Description

Verify that the `forms` `FunctionDeclaration` built at S2 carries every part
of a `FunctionDeclaration` CST node, and carries each type reference as a
type form with its declared bounds as spelled (ADR-011 §2.2 E2 row).

This catches three faults: a production that resolves or types a bound at
S2, one that drops the `using` alias or the `decreases` measure, and one
that needs a `NodeKey` to represent a named parameter type.

Scope: FR-091-AC-2.

## Test Procedure

Every fixture unit below starts with the complete-V1 header
(`language "ix:native" edition "1-draft";`) and one profile selection whose
alias is `v`.

1. Build an admissible unit whose only declaration is
   `function inc using v(x: Int[0, 9], y: Digit): Int[0, 10] pure decreases(x) { x + 1 }`.
   `Digit` is deliberately undeclared, because S2 does not resolve names.
2. Run the S2 entry and take the `FunctionDeclaration`.
3. Read the name, the `using` alias, each parameter's name and type form,
   the result type form, the measure and the body.
4. With `syn`, list the field types of `FunctionDeclaration` and of the type
   form, and the return types of every public accessor on both.

Tag the test `#[trace("FR-091-AC-2", "TC-393")]`.

## Expected Results

- Name `inc`; the `using` field holds alias `v` and the span of that `v`.
- Parameter `x`: head `Int`, bounds spelled `0` and `9`. Parameter `y`:
  qualified-name head `Digit`.
- Result: head `Int`, bounds spelled `0` and `10`.
- Measure `Name("x")`; body `Binary{Add, Name("x"), Integer(1)}`.
- Step 4 finds no `ValueType` and no `NodeKey`, including inside `Option`,
  `Vec` or `Box` wrappers.

## Status

Planned; no test backs this case.
