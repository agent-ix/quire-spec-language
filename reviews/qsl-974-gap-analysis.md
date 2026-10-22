---
id: SR-15483
title: "gap analysis of QSL-974 FR-093-AC-33, AC-34 and TC-483 (QSL PR #1214)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@b155d3ea321b8dbde055fe4a282ec7789ec3a3a7; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md, spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md, spec/test-cases/TC-483-assembler-admits-dimensions-and-units.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-483
    type: reviews
---
# SR-15483: gap analysis of QSL-974 FR-093-AC-33, AC-34 and TC-483 (QSL PR #1214)

## Summary

Ticket: QSL-974. Plan completion: not assessed (planless). This PR is spec only. The PR body
assigns the code half (lowering `Value::Quantity`, dropping `ProbabilisticCause::QuantityLiteral`)
to another lane. Checked: the tagged tests for the changed ACs, the code paths the spec states, and
other spec positions that read quantity literals.

Code facts the spec states, all confirmed at b155d3ea3:

- `check::typing` already types `ExprNode::Quantity` as `ValueType::Quantity(unit)` after measuring
  the magnitude (typing.rs:941-961).
- Only `lowering.rs:4453-4457` refuses, with `ProbabilisticCause::QuantityLiteral`.
- `DistinctUnits` maps to `type-mismatch` (refusal.rs:2451).
- The quantity divide arm refuses `Nonzero` unconditionally (facts.rs:940).
- `ExactFault::ZeroDenominator` maps to `undefined_expression`/`unproved-nonzero`
  (probabilistic.rs:568,588).
- `AboveCeiling` maps to `stage_limit_exceeded`/`rational-bits-exceeded` (probabilistic.rs:639-650).

No other spec still asserts the value-expression refusal: NFR-011, FR-185, FR-195, FR-392, FR-393,
TC-620 and TC-1078 were checked.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | minor: the only test tagged FR-093-AC-33, `qsl-semantics/src/check/assemble/tests.rs:2294` (`#[trace("FR-091-AC-44", "FR-093-AC-33", "TC-483")]`), asserts the superseded `ProbabilisticCause::QuantityLiteral` refusal (tests.rs:2321-2336). FR-093-AC-34 has no tagged test. Once this PR merges, the matrix shows AC-33 covered by a test that asserts the opposite of the AC. The code half must rewrite that test and add AC-34's vectors, and the ticket should record that the spec and code halves land together or back to back. The lead's code-lane ticket should name this; this spec PR cannot fix it. | qsl-semantics/src/check/assemble/tests.rs:2294-2336; qsl-semantics/src/check/lowering.rs:4451-4457 |
| FND-002 | low | minor: the "Probabilistic positions" paragraph lists the non-value-expression quantity positions as supports, thresholds and horizons only. Quantity literals also appear in time declarations and timed-interval bounds (FR-393, `qsl-forms/src/time_model.rs:53,126`, `temporal_clause.rs:516`, TC-1078). Those are not value-expression literals either, and they lower by their own FRs. Add them to the paragraph, or say generally that a quantity literal outside a value expression is read by the requirement owning that position, so no one reads FR-093 as lowering them. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:202-208 |

## Verdict

No gap blocks the spec PR. FND-001 is a hand-off condition for the code half, and FND-002 is a
scope clarification.

## Dispositions

Round 1, reviewed QSL #1214 at 3fd9a7398 (fix commit over b155d3ea3); code facts re-verified in the QSL tree at that head.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | deferred | code half of QSL-974 (terra) rewrites assemble/tests.rs:2294-2336 and adds the AC-33/AC-34 tests; the PR body's "Code half must land back to back" section records the landing order |
| FND-002 | fixed | the paragraph is now "Positions outside a value expression" and lists time declarations and timed intervals (FR-393, TC-1078) |
