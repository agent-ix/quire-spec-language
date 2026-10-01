---
id: FR-315
title: "Build union declaration and case expression forms at S1 and S2"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-032
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: traces_to
  - target: ix://agent-ix/quire-specification/AD-015
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-143
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-146
    type: depends_on
---
# FR-315: Build union declaration and case expression forms at S1 and S2

## Description

When a complete-V1 source unit contains a `union-decl` or an expression
`case`, S1 SHALL parse it by QSpec's shared-grammar productions
(`union-decl`, `union-member`, `case`, `case-arm`), and S2 SHALL build one
`UnionForm` declaration or one `CaseForm` expression for it, each carrying
its source span (ADR-012 §16.4 S1 and S2 rows).

## Inputs

- A complete-V1 source unit (FR-091's E1 and E2 input).

## Outputs

- `DeclarationForm::Union(UnionForm)`: the union name, and its members in
  source order, each with its name, its ordered payload type references
  (empty for a nullary member) and its span.
- `Expression::Case(CaseForm { scrutinee, arms })`, each `ArmForm` holding
  its member name as written (bare or qualified), its ordered binder
  identifiers, its body expression and its span.
- Or an S1 parse diagnostic or S2 refusal at the offending span.

## Behavior

- S1 SHALL treat `union` as a keyword and parse `union-decl` as a
  declaration (QSpec shared grammar).
- S1 SHALL parse an expression `case` by the expression production, and the
  protocol `choice` `case` by its own production; the two are told apart by
  syntactic context. An expression `case` SHALL produce a different CST
  production from the protocol one.
- S1 SHALL parse a `case` scrutinee as an expression that admits no
  top-level record value, so the `{` after the scrutinee opens the arm list.
- S1 SHALL admit a parenthesized record value as a scrutinee.
- S2 SHALL dispatch a unit's leading `union` token to the `SumCase` forms
  entry (one leading-token kind, one dispatch entry).
- S2 SHALL build `U::m(e, ...)` as an `Expression::Call` and a nullary
  `U::m` as an `Expression::Name`, with no separate construction form; S3
  decides what they denote (FR-317).
- S2 SHALL keep arms in source order, without reordering or deduplicating
  them; S3 checks them (FR-318).
- The FR-151 dispatch rewrite SHALL rename inside a `case`'s scrutinee and
  arm bodies and SHALL NOT rename an arm binder.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-315-AC-1 | Source `union Shape { Empty, Circle(Integer), Rect(Integer, Integer), }` yields one `UnionForm` named `Shape` with members `Empty` (no payload), `Circle` (`Integer`) and `Rect` (`Integer`, `Integer`) in that order, each with its own span; the trailing comma is admitted. | Test (TC-815) |
| FR-315-AC-2 | A function body `case s { Circle(r): r; Shape::Rect(w, h): w * h; Empty: 0; }` yields one `CaseForm` whose scrutinee is the name `s` and whose three arms keep source order, member spellings `Circle`, `Shape::Rect` and `Empty`, binders `[r]`, `[w, h]` and `[]`, and their bodies and spans. `Shape::Circle(3)` and `Shape::Empty` yield an `Expression::Call` and an `Expression::Name`, not a construction form. | Test (TC-815) |
| FR-315-AC-3 | A `case` whose scrutinee is an unparenthesized record value `R { f: 1 }` is not parsed as having that record as scrutinee; the same `case` with the scrutinee written `(R { f: 1 })` parses. A protocol `choice` with `case` arms in the same unit parses to the protocol production, unchanged. | Test (TC-816) |

## Dependencies

- QSpec shared grammar (`union-decl`, `union-member`, `case`, `case-arm`),
  QSpec AD-015.
- FR-091 (forms stage and assembler), FR-317 and FR-318 (S3).

## References

- Owning ticket: QSL-383. Design: ADR-012 §16.2 and §16.4.
