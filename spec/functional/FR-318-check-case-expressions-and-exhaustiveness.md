---
id: FR-318
title: "Check case expressions, their result type and the exhaustiveness obligation"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-032
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-315
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-316
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-146
    type: depends_on
---
# FR-318: Check case expressions, their result type and the exhaustiveness obligation

## Description

When S3 types an `Expression::Case` (FR-315), the checker SHALL return one
checked `Case` node or exactly one refusal, found by the `SumCase` family's
staged builder (scrutinee, then arms, then `finish`): the first, in the
order below, of a scrutinee refusal, an arm body refusal, or the first
failed QSpec FR-146 exhaustiveness obligation, QSpec FR-146's order
(ADR-012 §16.5).

## Inputs

- A `CaseForm`; the scope; the enclosing expected type, if any;
  `CheckContext`.

## Outputs

- A checked `Case` node: the scrutinee, the union `U`, and one arm per member
  of `U`, each with its member, its typed binders and its checked body; and
  the `case`'s result type.
- Or one refusal with a catalog code and a locus, or `StageFailure::Limit`.

## Behavior

### Order

- The checker SHALL first type the scrutinee and resolve its static type to
  a declared union `U`. If the scrutinee is refused, or its type is not a
  declared union (an enum, `Integer` or any other type), then the checker
  SHALL return that refusal, using `ill_typed`/`type-mismatch` with expected
  "a declared union" and the actual type for a non-union, at the scrutinee,
  and SHALL resolve no arm.
- The checker SHALL then visit arms in source order. For each arm it SHALL
  resolve the arm's head against `U` (a bare `m` or a qualified `U::m` names
  `U`'s member `m`) and record one of: a resolved member, unknown, or an
  arity mismatch. Recording a head SHALL refuse nothing.
- For each arm whose head resolved with the right arity, the checker SHALL
  type its binders positionally from the member's payload types, scoped to
  that arm's body only, and check the body. The first body refusal in source
  order SHALL be the `case`'s refusal. The checker SHALL NOT check the body
  of an arm whose head is unknown or whose arity mismatches.
- When no arm body is refused, `finish` SHALL read only the recorded heads
  and check QSpec FR-146's obligations in FR-146's order, `duplicate-arm`,
  `unknown-member`, `arm-arity`, `missing-arm`, and SHALL return the first
  failure as `undefined_expression`/`unproved-exhaustiveness`, naming `U`'s
  declaration identity, the arm set and the failed obligation.
- An arm naming a member of another union SHALL be `unknown-member`. An arm
  that repeats an earlier arm's member SHALL be `duplicate-arm`; there is no
  other unreachable-arm refusal.
- A refused `case` SHALL emit no checked node.

### Result type

- The checker SHALL decide a `case`'s result type by QSpec FR-146 "Case
  result type": the expected type its position supplies when that is
  unique, and otherwise the type inferred from the arm bodies. It SHALL
  check the arm bodies against it in source arm order, and SHALL refuse the
  first body that does not have it `ill_typed`/`type-mismatch` at that
  body.

### Loci and causes

- Each refusal's locus SHALL resolve through the S2 spans to a
  `Locus::Region` (FR-096): a scrutinee refusal at the scrutinee; a body
  refusal at the body; `duplicate-arm` at the later arm of the first
  duplicate pair in source order; `unknown-member` and `arm-arity` at the
  first such arm; `missing-arm` at the `case` expression.
- The exhaustiveness refusal SHALL be carried by one `SumCaseCause` whose
  obligation is one of `DuplicateArm`, `UnknownMember`, `ArmArity` and
  `MissingArm`, each mapping to `undefined_expression`/
  `unproved-exhaustiveness`. No new catalog code is added.

### Requirements and clause kinds

- A `case` SHALL check the same way in a function body, a state clause body
  and a `decreases` measure, each under its clause kind. A clause that
  contains a `case` SHALL record the clause's own `value-validity` claim
  through its own family; the `case` and its exhaustiveness obligation SHALL
  add no claim and request no capability kind (FR-057).

### Resources

- The checker SHALL admit `case` nesting of any depth, `case`s with any
  number of arms and unions with any number of members, bounded only by the
  checking ceilings a caller configures (NFR-011's node and work ceilings).
  Reaching one SHALL return `StageFailure::Limit` naming the ceiling's kind,
  its configured bound, the `CheckingLimits` field that raises it and the
  locus of the node whose charge was denied (FR-096). The checker SHALL NOT
  turn a host stack limit into an outcome.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-318-AC-1 | Over `Shape`, `function area using v (s: Shape): Integer pure { case s { Circle(r): r * r * 3; Shape::Rect(w, h): w * h; Empty: 0; } }` checks: one `Case` node over `Shape` with arms for `Empty`, `Circle` and `Rect`, binders `r: Integer`, `w: Integer`, `h: Integer`, each binder visible only in its own arm body, and result type `Integer`. A `case` nested in an arm body checks. | Test (TC-821) |
| FR-318-AC-2 | With no enclosing expectation, arms `Circle(r): r; Rect(w, h): true; Empty: 0;` refuse `ill_typed`/`type-mismatch` at the body `true`, by QSpec FR-146 "Case result type" (the first body whose type is neither the first body's nor, with an `Integer` first body, an integer type). | Test (TC-821) |
| FR-318-AC-3 | One `case` per obligation refuses `undefined_expression`/`unproved-exhaustiveness` with that obligation, `Shape`'s identity and the arm set: two `Empty` arms (`duplicate-arm`, at the second); an arm `Other::X` naming a member of a second union (`unknown-member`); `Circle(a, b)` (`arm-arity`); arms for `Empty` and `Circle` only (`missing-arm`, at the `case`). QSpec TC-264's cases each give TC-264's verdict. | Test (TC-822) |
| FR-318-AC-4 | A `case` with two `Empty` arms and no `Rect` arm reports only `duplicate-arm`. An `unknown-member` arm whose body is ill-typed reports only `unknown-member`, and an `arm-arity` arm `Circle(a, b)` whose body uses an unbound identifier `z` reports only `arm-arity`. A `case` with an ill-typed body in a resolved arm and a missing arm reports only `ill_typed` at that body. A scrutinee of type `Integer` followed by arms naming unknown members reports only the scrutinee's `ill_typed`/`type-mismatch`; a scrutinee of an enum type reports the same. | Test (TC-823) |
| FR-318-AC-5 | The `case` of AC-1 checks inside a state clause body and inside a `decreases` measure, each under its clause kind. A state clause containing it yields exactly one `value-validity` requirement record, the clause's own, and a function containing it yields no record for the `case` or its exhaustiveness obligation. | Test (TC-824) |
| FR-318-AC-6 | A function whose body nests `case` 1,000 levels deep (each arm body a further `case`) checks under the default checking ceilings. Checking it with the node ceiling set one below the units it needs returns `StageFailure::Limit` with kind node count, the configured bound, the `CheckingLimits` field to raise and the region of the node whose charge was denied; raising the ceiling by one admits it. No outcome names a nesting-depth limit, and none is a host stack overflow. | Test (TC-825) |

## Dependencies

- QSpec FR-146 ("Case exhaustiveness", "Case result type"), FR-146-AC-10,
  FR-146-AC-11; the
  catalog cause `undefined_expression`/`unproved-exhaustiveness`.
- FR-062 (family contract and staged builder), FR-057 (claim forms),
  FR-096 and NFR-011 (limits and loci), FR-315, FR-316.

## References

- Owning ticket: QSL-383. Design: ADR-012 §16.4 S3 `case` row, §16.5.
- `case` result-type rule (SC-G6): QSpec FR-146 "Case result type"
  (specification ticket STD-115).
