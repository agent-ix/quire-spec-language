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

Exact source bytes and identity, the source header and existing caller-lowered
source/token/node/nesting limits. The `ix:native` / `1-draft` grammar is the
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

If a syntax budget is exhausted, then the parser SHALL return a stage limit, reported as `stage_limit_exceeded`, without an admitted unit. A syntax budget is an S1 stage limit, not a caller work budget (`quire.native.diagnostics/v1` `stage_limit_exceeded` row, revision `1-draft.6`; [FR-096](FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md)).

The compiler SHALL preserve the historical `0-draft` grammar and profile behavior for historical selections.

Extend the current Logos token recognizer and structured/Pratt parser. Qualified
model/member positions accept edition-reserved ASCII words under the shared
grammar; native binder positions do not. Do not scan a second expression string
inside `holds`, guards, captures, queries or choreography nodes. Retain explicit
grouping, precedence, temporal-constant versus `holds(true)` distinction and all
authored selectors. Parsing a valid new form is separate from admitting its
selected profile; balanced text is never a substitute for a family AST.

This requirement adds an explicit composed path. Existing finite-state syntax,
package formats and fixture bytes are not reinterpreted as composed artifacts.
The choice of Rust API names is an implementation detail; its result type must
make the selected edition and declaration kinds inspectable without downcasting
opaque strings or inferring kinds from diagnostic messages.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-035-AC-1 | A complete unit containing a reusable predicate, state clause, temporal clause and protocol declaration produces four inspectable declaration kinds, with their original profile selectors and source spans. | Test (TC-113) |
| FR-035-AC-2 | Precedence, nonchained comparisons and temporal relations match the shared grammar; temporal syntax in a value argument, malformed family bodies and trailing garbage refuse without a successful unit. | Test (TC-113) |
| FR-035-AC-3 | A model member spelled `result` or `retry` parses in its qualified position; the same reserved spelling as a native binder refuses in the composed edition. An identifier newly reserved there retains its historical `0-draft` treatment. | Test (TC-113) |
| FR-035-AC-4 | CRLF, comments and escaped multibyte text retain exact original bytes and correct byte spans; an escaped string's decoded offset cannot replace its source location. | Test (TC-113) |
| FR-035-AC-5 | Exact syntax limits admit the otherwise valid unit; the next required token/node/nesting step above a lowered limit returns `stage_limit_exceeded` (`node-count-exceeded` for a token or node, `nesting-depth-exceeded` for nesting), with no recovery path bypass. | Test (TC-113) |
| FR-035-AC-6 | Historical source, formatted output, refusal codes and package identities retain their frozen expectations; a composed selector is never silently replaced with the historical profile. | Test (TC-113) |

## Dependencies

[FR-002](FR-002-parse-native-units.md) owns the existing syntax path;
[FR-001](FR-001-read-exact-source.md) owns exact source intake.
[Compiler #35](https://github.com/agent-ix/quire-spec-language/issues/35) owns this
L2 implementation slice. Predicate evaluation and family execution remain their
existing roadmap tickets; this requirement does not claim them from parsing.

## Status

The syntax implementation uses `parse_native` and `parse_native_source`;
TC-113 exercises their typed output and historical compatibility. This completes
the syntax portion of compiler #35. FR-036's package linking remains open.
