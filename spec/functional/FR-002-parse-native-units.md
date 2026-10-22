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

The `quire-spec parse` command is a frontend over that operation: it reads
the file its operands name, calls the `parse` operation, writes the
operation's outcome document and exits with FR-285's code for the
document's category.

## Inputs

- Source bytes and their `SourceIdentity`.
- S1 limits: `qsl_cst::Limits`, each field set by its FR-255 setting.
- For `quire-spec parse`: the source operands FR-010 lists and zero or more
  `--limit s1.<name>=<n>` operands.

## Outputs

- From `qsl_cst::parse`: a `ParsedSource` holding the source's exact bytes,
  its lossless CST and its located diagnostics, or a fatal source or
  resource diagnostic.
- From the `parse` operation: `Staged<ParsedSource>`, or a typed
  `StageFailure`.
- From `quire-spec parse`: one `quire-outcome/1` document on stdout and
  FR-285's exit code.

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
- The `quire-spec parse` command SHALL take S1 limits only as
  `--limit s1.<name>=<n>` operands, which it passes to FR-255's settings
  operation.
- When no `--limit` operand is given, the `quire-spec parse` command SHALL
  parse under the FR-255 defaults.
- If a `--limit` operand names a setting outside `s1.*` or is refused by the
  settings operation, or another limit flag is given, the `quire-spec parse`
  command SHALL refuse it as a usage error with exit 20 before it reads the
  source file.
- The `quire-spec parse` command SHALL write exactly the bytes of
  `OutcomeDocument::from_parse` over the operation's result to stdout.
- The `quire-spec parse` command SHALL exit with FR-285's code for that
  document's category.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-002-AC-1 | The `parse` operation over `tests/fixtures/spine-compile.native` returns a `Staged<ParsedSource>` whose `ParsedSource` is admissible, has no diagnostic and holds the fixture's exact bytes. | Test (TC-012) |
| FR-002-AC-2 | `qsl_cst::parse` over a unit holding each of `a < b < c`, `a = b != c` and `a < b + c = d` returns a `ParsedSource` that is not admissible and holds an `invalid_syntax` diagnostic at the second comparison operator. | Test (TC-012) |
| FR-002-AC-4 | `qsl_cst::parse` over a source whose syntax nodes exceed `s1.nodes` stops with `stage_limit_exceeded` naming `s1.nodes`, its configured value, the count reached and setting `s1.nodes` (FR-255). | Test (TC-012) |
| FR-002-AC-5 | The CST node of a parenthesized expression spans from its `(` to its `)`, both included. | Test (TC-012) |
| FR-002-AC-6 | The bounded malformed/truncation corpus returns a `ParsedSource` or a located diagnostic from `qsl_cst::parse` without panic; any admissible variant retains its exact submitted bytes. | Test (TC-019) |
| FR-002-AC-7 | `quire-spec parse` with the source operands FR-010 lists over `tests/fixtures/spine-compile.native` writes to stdout exactly the bytes of `OutcomeDocument::from_parse` over the `parse` operation's result for that file (operation `parse`, `last_stage` `S2`, category `success`, empty `items`, `diagnostics` and `artifacts`, `result` `null`) and exits 0. Over a unit holding `a < b < c` it writes the document with category `refusal`, `last_stage` `S1` and one `invalid_syntax` diagnostic at that operator, and exits 20. `make ci-clean-build`'s parse step is this command over `tests/fixtures/spine-compile.native`. | Test (TC-012) |
| FR-002-AC-8 | `quire-spec parse` with `--limit s1.tokens=<n>` parses under `s1.tokens` at `n` and every other S1 limit at its default. With `--limit s3.nodes=1`, `--limit s1.tokens=ten`, `--source-bytes 10`, `--tokens 10` or `--nodes 10`, it exits 20 with a usage refusal naming the operand and writes no outcome document. | Test (TC-750) |

## Dependencies

- [US-001](../usecase/US-001-author-native-source.md) supplies the user need.
- [FR-255](FR-255-name-the-setting-that-raises-a-reached-limit.md): the `s1.*` settings and the settings operation.
- [FR-275](FR-275-take-a-typed-request-limits-and-cancel-on-every-lifecycle-operation.md): the `parse` operation.
- [FR-285](FR-285-map-every-outcome-category-to-one-exit-code.md), [FR-286](FR-286-serialize-every-outcome-as-one-json-outcome-document.md): exit code and outcome document.
- [The CST parser](../../qsl-cst/src/parser.rs) supplies the scoped context.
