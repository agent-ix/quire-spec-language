---
id: FR-035
title: "Parse composed native units under an explicit edition"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-001
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-002
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-032
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-033
    type: depends_on
---
# FR-035: Parse composed native units under an explicit edition

## Description

When a source selects an admitted composed edition, the compiler SHALL construct located syntax for its predicate, state, temporal and choreography declarations using that edition's shared grammar.

## Inputs

Exact source bytes and identity, the source header and existing caller-configured
source/token/node limits. The `ix:native` / `1-draft` grammar is the
[shared grammar](https://github.com/agent-ix/quire-specification/blob/main/proposals/quire-v1/shared-grammar.md)
landed in [standard PR #15](https://github.com/agent-ix/quire-specification/pull/15).

## Outputs

A complete source-owned syntax unit with typed declaration variants and located
expression/control nodes, or a located typed diagnostic. A parsed unit establishes
no model, profile, binding, execution or backend capability admission.

## Behavior

The compiler SHALL classify native keywords according to the selected edition.

The compiler SHALL retain the original spelling and half-open source-byte span of every declaration, reference, binder and literal.

The parser SHALL consume each family body through the shared grammar productions.

The parser SHALL distinguish ordinary value expressions, temporal formulas and protocol control nodes in its syntax output.

If source is malformed or remains after the last complete declaration, then the parser SHALL return a located syntax diagnostic without a successful partial unit.

If a selected edition is unavailable, then the parser SHALL return the typed unknown-edition diagnostic at the header selection.

If a syntax budget is exhausted, then the parser SHALL return `stage_limit_exceeded` with the budget's `<kind>-exceeded` cause (`quire.native.diagnostics/v1`; the token budget is `token-count-exceeded`) without an admitted unit.

The compiler SHALL preserve the historical `0-draft` grammar and profile behavior for historical selections.

Qualified model/member positions accept edition-reserved ASCII words under the
shared grammar; native binder positions do not. The parser does not scan a second
expression string inside `holds`, guards, captures, queries or choreography
nodes. It retains explicit grouping, precedence, the temporal-constant versus
`holds(true)` distinction and all authored selectors. Parsing a valid form is
separate from admitting its selected profile; balanced text is never a
substitute for a family syntax tree.

The result makes the selected edition and declaration kinds inspectable without
downcasting opaque strings or inferring kinds from diagnostic messages.

**QSL-499 disposition:** the dedicated composed-unit parser is deleted under ADR-011, ADR-033 and FR-499. Source is parsed by the lossless CST parser (FR-002), which builds the located declaration, expression and protocol control forms (FR-308, FR-257) that the spine checks; the retired criteria below are retained as migration history and are not active acceptance obligations. FR-035-AC-7 stays active: it constrains the `1-draft` identifier spellings the spine parser and checker accept.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-035-AC-1 | **RETIRED in QSL-499; historical only.** A complete unit containing a reusable predicate, state clause, temporal clause and protocol declaration produces four inspectable declaration kinds, with their original profile selectors and source spans. | Test (TC-113) |
| FR-035-AC-2 | **RETIRED in QSL-499; historical only.** Precedence, nonchained comparisons and temporal relations match the shared grammar; temporal syntax in a value argument, malformed family bodies and trailing garbage refuse without a successful unit. | Test (TC-113) |
| FR-035-AC-3 | **RETIRED in QSL-499; historical only.** A model member spelled `result` or `retry` parses in its qualified position; the same reserved spelling as a native binder refuses in the composed edition. An identifier newly reserved there retains its historical `0-draft` treatment. | Test (TC-113) |
| FR-035-AC-4 | **RETIRED in QSL-499; historical only.** CRLF, comments and escaped multibyte text retain exact original bytes and correct byte spans; an escaped string's decoded offset cannot replace its source location. | Test (TC-113) |
| FR-035-AC-5 | **RETIRED in QSL-499; historical only.** Exact syntax limits admit the otherwise valid unit; the next required token/node/source-byte step above a lowered limit returns `stage_limit_exceeded`/`<kind>-exceeded` (a token is `token-count-exceeded`), with no recovery path bypass. | Test (TC-113) |
| FR-035-AC-6 | **RETIRED in QSL-499; historical only.** Historical source, formatted output, refusal codes and package identities retain their unchanged historical expectations; a composed selector is never silently replaced with the historical profile. | Test (TC-113) |
| FR-035-AC-7 | Under `1-draft`, the exact spellings `helper`, `rec`, `cast` and `Tuple` are ordinary identifiers in every native declaration and binder name position and in every reference to such a name (QSpec FR-339-AC-5; the QSpec shared grammar does not reserve them, [QSL-1649](https://linear.app/agent-ix/issue/QSL-1649)): for each spelling `w`, `function w using v(x: Integer): Integer pure { x }` parses and checks, and `function c using v(): Integer pure { w(1) }` evaluates `c()` to `1`; `w` as the `using` profile alias, a parameter, a `let` binder, a record name and its field, an enum member, a type-alias name and a model selection alias (in a unit that also declares a function) each parses and checks with no `invalid_syntax` and no `unsupported_construct`; and with `profile w = "quire.value.complete/v1";`, `function f using w(x: Integer): Integer pure { x }` is admitted with its profile alias resolved to the profile `w` selects. The recognizer's unsupported-spelling class does not apply to them under `1-draft`. Lowercase `tuple`, a complete-V1 reserved word, still refuses `invalid_syntax`/`unexpected-token` as a record name. Under `0-draft` the four exact spellings keep their historical `unsupported_construct` classification unchanged. | Test (TC-113) |

## Dependencies

[FR-002](FR-002-parse-native-units.md) owns the existing syntax path;
[FR-001](FR-001-read-exact-source.md) owns exact source intake.
[Compiler #35](https://github.com/agent-ix/quire-spec-language/issues/35) owns this
L2 implementation slice. Predicate evaluation and family execution remain their
existing roadmap tickets; this requirement does not claim them from parsing.
