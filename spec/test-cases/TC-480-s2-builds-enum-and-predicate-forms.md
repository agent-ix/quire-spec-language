---
id: TC-480
title: "S2 builds enum and predicate forms"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-480: S2 builds enum and predicate forms

## Description

Verify the `enum`, `ordered` and `predicate` dispatch entries and the forms
they build: an enum form with its ordered flag, its members in source order
and each member's display string, and a predicate as the `forms`
`FunctionDeclaration` of kind `Predicate`.

This catches an S2 production that sorts an unordered enum's members (the
assembler sorts them for the preimage, S2 keeps source order), drops a
display string or its span, or builds a predicate with no result type form
or with a measure.

Scope: FR-091-AC-25, FR-091-AC-26.

## Test Procedure

Every fixture unit below starts with the complete-V1 header
(`language "ix:native" edition "1-draft";`) and one profile selection whose
alias is `v`.

1. Run S1 and S2 on a unit declaring
   `ordered enum Level { LOW, HIGH = "High", }`.
2. Run S1 and S2 on a unit declaring `enum Color { RED, BLUE }`.
3. Run S1 and S2 on a unit declaring
   `predicate Positive using v(x: Int[0, 9]): Boolean { x > 0 }`.
4. Run S1 and S2 on a unit declaring
   `function inc using v(x: Int[0, 9]): Int[0, 10] pure { x + 1 }`.

Tag the tests `#[trace("FR-091-AC-25", "TC-480")]` and
`#[trace("FR-091-AC-26", "TC-480")]`.

## Expected Results

- Step 1 gives one enum form: name `Level` with the span of `Level`,
  ordered, members `LOW` (no display string) and then `HIGH` (display
  string `"High"`, with the span of that literal), each with the span of its
  case name.
- Step 2 gives one enum form that is not ordered, with members `RED` and
  then `BLUE`.
- Step 3 gives one `FunctionDeclaration` of kind `Predicate`: name
  `Positive`; `using` alias `v` with the span of that `v`; parameter `x`
  with a type form of head `Int` and bounds spelled `0` and `9`; a result
  type form of head `Boolean` with the span of the `Boolean` token; no
  measure; body `Binary{Greater, Name("x"), Integer(0)}`.
- Step 4 gives a `FunctionDeclaration` of kind `Function`.

## Status

Not implemented. QSL-275 adds the entries and the forms; today S2 refuses
steps 1 to 3 with `NoDispatchEntry`.
