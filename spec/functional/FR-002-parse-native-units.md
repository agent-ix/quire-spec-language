---
id: FR-002
title: "Parse native source with the lossless CST parser"
type: FR
relationships:
  - target: "ix://agent-ix/quire-spec-language/US-001"
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-275
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-286
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-287
    type: references
---
# FR-002: Parse native source with the lossless CST parser

## Description

When a source unit is parsed, QSL SHALL construct located syntax for every
admitted declaration and expression with the complete-V1 lossless CST parser,
`qsl_cst::parse` (S1), under the `qsl_cst::Limits` the FR-255 settings
`s1.input_bytes`, `s1.tokens`, `s1.nodes` and `s1.work_units` set. The
`parse` lifecycle operation (FR-275, `qsl_replay::spine::parse`) runs S1 and
then S2 over the result, and its outcome serializes through
`OutcomeDocument::from_parse` (FR-286).

A CLI frontend reaches this operation through the driver's `parse` verb
(FR-287, ADR-029 CB-1). QSL's part is the library contract: the `parse`
operation, its outcome document and FR-285's exit function. The frontend's
arguments, including how it takes `s1.*` settings, and its refusal of a bad
argument are the driver's (FR-287, Overlap).

## Inputs

- Source bytes and their `SourceIdentity`.
- S1 limits: `qsl_cst::Limits`, each field set by its FR-255 setting, through
  the limits type's builder methods or FR-255's settings operation.

## Outputs

- From `qsl_cst::parse`: a `ParsedSource` holding the source's exact bytes,
  its lossless CST and its located diagnostics, or a fatal source or
  resource diagnostic.
- From the `parse` operation: `Staged<ParsedSource>`, or a typed
  `StageFailure`.
- From `OutcomeDocument::from_parse` over that result: one
  `quire-outcome/1` document, whose category gives FR-285's exit code.

## Behavior

- Token recognition SHALL be declarative.
- Expression precedence SHALL follow the complete-V1 grammar: `implies` is
  right-associative and a comparison operator takes no comparison operand
  (comparisons do not chain).
- A grouped expression and each name and identity literal SHALL retain its
  exact source span.
- Parsing SHALL establish no model linkage.
- When the source holds a syntax error, `qsl_cst::parse` SHALL return a
  recovered `ParsedSource` that retains every source byte, holds the located
  diagnostic and is not admissible; the `parse` operation SHALL refuse it as
  a typed refusal at S1.
- The `parse` operation SHALL run S1 under exactly the `qsl_cst::Limits` its
  caller gives it.
- `OutcomeDocument::from_parse` SHALL serialize the `parse` operation's
  result as one `parse` document whose category's FR-285 exit code is the
  code a frontend exits with.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-002-AC-1 | The `parse` operation over `tests/fixtures/spine-compile.native` returns a `Staged<ParsedSource>` whose `ParsedSource` is admissible, has no diagnostic and holds the fixture's exact bytes. | Test (TC-012) |
| FR-002-AC-2 | `qsl_cst::parse` over a unit holding each of `a < b < c`, `a = b != c` and `a < b + c = d` returns a `ParsedSource` that is not admissible and holds an `invalid_syntax` diagnostic at the second comparison operator. | Test (TC-012) |
| FR-002-AC-4 | `qsl_cst::parse` over a source whose syntax nodes exceed `s1.nodes` stops with `stage_limit_exceeded` naming `s1.nodes`, its configured value, the count reached and setting `s1.nodes` (FR-255). | Test (TC-012) |
| FR-002-AC-5 | The CST node of a parenthesized expression spans from its `(` to its `)`, both included. | Test (TC-012) |
| FR-002-AC-6 | The bounded malformed/truncation corpus returns a `ParsedSource` or a located diagnostic from `qsl_cst::parse` without panic; any admissible variant retains its exact submitted bytes. | Test (TC-019) |
| FR-002-AC-7 | `OutcomeDocument::from_parse` over the `parse` operation's result for `tests/fixtures/spine-compile.native` is the document with operation `parse`, `last_stage` `S2`, category `success`, empty `items`, `diagnostics` and `artifacts`, and `result` `null`, and its category's FR-285 exit code is 0. Over a unit holding `a < b < c` it is the document with category `refusal`, `last_stage` `S1` and one `invalid_syntax` diagnostic at the second `<`, and its exit code is 20. `make ci-clean-build` runs, in its no-default-features build, a test that calls the `parse` operation over `tests/fixtures/spine-compile.native` and requires category `success`. | Test (TC-012) |
| FR-002-AC-8 | The `parse` operation over `tests/fixtures/spine-compile.native`, under the `qsl_cst::Limits` FR-255's settings operation returns for `s1.tokens=<n>`, with `n` one less than the fixture's token count, stops with `stage_limit_exceeded` naming `s1.tokens`, bound `n` and the count reached, and its document's exit code is 22; with `n` equal to the token count it succeeds. Those limits hold `s1.tokens` at `n` and every other S1 field at its FR-255 default. | Test (TC-750) |

## Dependencies

- [US-001](../usecase/US-001-author-native-source.md) supplies the user need.
- [FR-255](FR-255-name-the-setting-that-raises-a-reached-limit.md): the `s1.*` settings and the settings operation.
- [FR-275](FR-275-take-a-typed-request-limits-and-cancel-on-every-lifecycle-operation.md): the `parse` operation.
- [FR-285](FR-285-map-every-outcome-category-to-one-exit-code.md), [FR-286](FR-286-serialize-every-outcome-as-one-json-outcome-document.md): exit code and outcome document.
- [FR-287](FR-287-reach-qsl-through-the-driver-cli.md): the driver's CLI, which owns the frontend's arguments.
- [The CST parser](../../qsl-cst/src/parser.rs) supplies the scoped context.
