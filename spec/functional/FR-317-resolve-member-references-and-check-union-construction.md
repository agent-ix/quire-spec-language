---
id: FR-317
title: "Resolve qualified member references and check union construction"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-032
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-065
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-313
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-316
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-143
    type: depends_on
---
# FR-317: Resolve qualified member references and check union construction

## Description

When S3 types an `Expression::Call` or an `Expression::Name` whose name is
qualified `q::m`, the checker SHALL resolve it through one `check`-core
call-target seam function to exactly one target (a union member, an enum
member, an import member, a declared function or a tuple constructor) or
refuse it; and when the target is a union member, the `SumCase` family SHALL
check the construction against QSpec FR-143's construction rule (ADR-012
§16.2).

## Inputs

- A typed `Call` or `Name` expression; the scope's declared unions, enums,
  import aliases, functions and tuples; the expected type, if any.

## Outputs

- A checked union-construction node: the union, the member, and the checked
  payload argument expressions in declared position order.
- Or the target `Value`'s existing resolution yields, unchanged.
- Or a refusal with a catalog code.

## Behavior

- The `Call` and `Name` arms of the S3 typer SHALL each call the call-target
  seam function exactly once. The seam function SHALL return a closed
  resolved-target enum with one union-member variant and one variant
  carrying `Value`'s existing resolution, and SHALL make one call into the
  owning family per variant.
- For `q::m`, the candidates SHALL be the declared unions, declared enums and
  import aliases named `q` that have a member `m`.
  - With exactly one candidate, `q::m` SHALL resolve against it: as a union
    member, as an enum member (as today) or as an import member (as today).
  - With more than one candidate, the checker SHALL refuse
    `ambiguous_declaration`/`ambiguous-name` at `q::m`, naming every
    candidate.
  - With no candidate, the checker SHALL refuse exactly as an unresolved
    name is refused.
- Every unqualified name SHALL resolve exactly as it does without unions.
- For a union-member target, the checker SHALL check each payload argument
  against its declared position type, in position order, under the
  enclosing clause kind (FR-065).
- If a construction names an undeclared member of `U`, supplies a payload for
  a nullary member, omits the call for a member that declares a payload, or
  supplies an argument count other than the member's declared arity, then
  the checker SHALL refuse `ill_typed`/`type-mismatch` at the construction
  expression, naming the expected and actual shape (QSpec FR-143).
- The checker SHALL check a construction the same way in a function body, a
  state clause body and a `decreases` measure, each under its clause kind.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-317-AC-1 | With `Shape` from FR-316-AC-1, `Shape::Empty` and `Shape::Rect(2, 3)` check as union constructions of type `Shape` naming members `Empty` and `Rect`; `Shape::Rect(2, true)` refuses `ill_typed`/`type-mismatch` at `true`; each of QSpec TC-262's construction cases gives TC-262's verdict. | Test (TC-819) |
| FR-317-AC-2 | `Shape::Square(1)`, `Shape::Empty(1)`, a bare `Shape::Circle` and `Shape::Rect(1)` each refuse `ill_typed`/`type-mismatch` at the construction expression, one refusal each. | Test (TC-819) |
| FR-317-AC-3 | When a union `q` and an enum `q` both declare `m`, when two unions named `q` both declare `m`, and when a union `q` declares `m` and an import alias `q` exports `m`, `q::m` refuses `ambiguous_declaration`/`ambiguous-name` naming every candidate. When two unions named `q` exist and only one declares `m`, `q::m` resolves to that union's member. An enum member `Kind::Empty` and a tuple constructor `Pair(1, 2)` resolve as they did before unions existed. | Test (TC-820) |
| FR-317-AC-4 | `Shape::Rect(2, 3)` checks as a union construction inside a state clause body and inside a `decreases` measure, each under its own clause kind, and `Shape::Rect(1)` refuses `ill_typed`/`type-mismatch` in both. | Test (TC-819) |

## Dependencies

- QSpec FR-143 ("Composite-value contract", "Declarations and identity"),
  FR-143-AC-12; QSpec shared grammar (the one qualified reference and call
  production).
- FR-065 (function application under the clause kind), FR-316.

## References

- Owning ticket: QSL-383. Design: ADR-012 §16.2, §16.4 S3 construction row,
  §16.5 construction row.
