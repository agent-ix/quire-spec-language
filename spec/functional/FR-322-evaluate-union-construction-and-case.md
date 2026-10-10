---
id: FR-322
title: "Evaluate union construction and case at S6a and charge their accounting points"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-032
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-318
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-321
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-143
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-146
    type: depends_on
---
# FR-322: Evaluate union construction and case at S6a and charge their accounting points

## Description

When S6a evaluates a checked union construction or `case` node inside the
node machine of the enclosing `Value` function or clause, the evaluator SHALL
construct the union value, or select the arm whose member identity equals
the scrutinee value's active member, bind its payload positionally and
evaluate its body (QSpec FR-146-AC-11); and SHALL charge union construction
at the charge point `quire.value.accounting/v1` defines for it (QSpec
FR-143), while a `case` charges exactly its scrutinee's charges and then its
selected arm body's charges (QSpec FR-146-AC-13; ADR-012 §16.4 S6a row).

## Inputs

- A checked construction or `case` node; an admitted environment; the run's
  meter.

## Outputs

- `Outcome::Completed` with the union value or the selected arm's value; or
  the first stopped operand's outcome; or `Err(InternalFault)` for an S6a
  invariant break.

## Behavior

- The evaluator SHALL reach construction and `case` through one
  `Machine::apply` arm per node kind, each making one call into the
  `SumCase` evaluation module. `SumCase` SHALL have no S6a declaration kind
  of its own.
- Construction SHALL evaluate payload arguments in position order. The first
  argument whose outcome is not `Completed` SHALL become the construction's
  outcome, with no later argument evaluated or charged and no union value
  built (QSpec FR-143 "Construction evaluation order").
- The evaluator SHALL evaluate a `case`'s scrutinee first.
- If the scrutinee's outcome is not `Completed`, then the evaluator SHALL
  return that outcome unchanged as the `case`'s outcome, selecting no arm.
- `case` SHALL select the one arm whose member identity equals the
  scrutinee's active member, bind the payload values to the arm's binders in
  declared position order, and evaluate only that arm's body.
- If a value reaches S6a and matches no arm, then the evaluator SHALL return
  `Err(InternalFault)` naming stage S6a and the invariant (FR-096), never a
  refusal or an undefined outcome: admission (FR-321) and exhaustiveness
  (FR-318) make that a QSL defect.
- The evaluator SHALL charge union construction at exactly the charge
  point, size and order QSpec FR-143 and `quire.value.accounting/v1` define
  for it (one `composite.result-retain` after the payload arguments), before
  the corresponding work.
- Selecting a `case` arm and binding its payload SHALL charge nothing: a
  `case` SHALL charge exactly its scrutinee's charges and then its selected
  arm body's charges, and an unselected arm body is neither evaluated nor
  charged (QSpec FR-146-AC-13).
- Evaluation SHALL handle `case` nesting and union values of any depth with no
  depth limit, bounded only by the run's caller-configured `work_units` and
  value limits. Exhausting one SHALL return
  `incomplete { limit_kind, ... }` naming the limit and its configured value
  at the denied point, and SHALL NOT turn a host stack limit into an
  outcome.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-322-AC-1 | `area` returns 0, 27 and 6 for `Shape::Empty`, `Shape::Circle(3)` and `Shape::Rect(2, 3)`, taking the `Empty`, `Circle` and `Rect` arms with `r = 3`, and `w = 2`, `h = 3`. A nested `case` in an arm body evaluates its inner arm. QSpec TC-265's cases each give TC-265's outcome. | Test (TC-831) |
| FR-322-AC-2 | Under `CheckMode::Kernel`, with `type Small = Int[0, 3]` and `q` holding `2, 2`: a `case` whose scrutinee is `Shape::Circle(sum<Small>(x in q: x))` returns `Undefined::SumOutOfDomain` and evaluates no arm body, and `Shape::Rect(sum<Small>(x in q: x), f())` returns the same outcome and never calls `f`. An injected scrutinee value whose `VariantId` matches no arm returns `Err(InternalFault)` naming S6a, not a refusal. | Test (TC-831) |
| FR-322-AC-3 | Evaluating `Shape::Rect(2, 3)` and `area(Shape::Rect(2, 3))` records one `composite.result-retain` for the construction with `value_occurrences = 3`, and for the `case` exactly the scrutinee's charges followed by the `Rect` body's charges, with no charge for arm selection or payload binding and none for the unselected arms; a work budget one unit below the total stops at the last listed point with `incomplete { limit_kind: work_units }`. | Test (TC-832) |
| FR-322-AC-4 | The unchanged TC-831 recursive sum over a Tree 10,000 levels deep completes under default run limits without host stack overflow. With all pre-call bounds independently permitting evaluator entry, evaluation `work_units` 39,996 denies the final actual arithmetic charge unspent and 39,997 completes. The trace verifies the semantic total and actual denied point rather than substituting a source prediction. Direct and replay have the same semantic charge sequence, including successful spend and the denied amount at the one-less boundary; no membership/conversion helper event or fabricated FunctionCall appears in that sequence. | Test |

## Dependencies

- QSpec FR-143 ("Construction evaluation order"), FR-146 ("Case
  exhaustiveness", FR-146-AC-11, FR-146-AC-13), `value-accounting.md`.
- FR-090 (family evaluation outcome), FR-096 (`InternalFault`), FR-318,
  FR-321.

## References

- Owning ticket: QSL-383. Design: ADR-012 §16.4 S6a row, §16.6 (S6a).
- Accounting charge points for union construction and `case` selection
  (SC-G4): QSpec FR-143, FR-146 and `value-accounting.md` (specification
  ticket STD-115).
