---
id: SR-15482
title: "failure-domain review of QSL-974 quantity literal refusals (QSL PR #1214)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@b155d3ea321b8dbde055fe4a282ec7789ec3a3a7; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md, spec/test-cases/TC-483-assembler-admits-dimensions-and-units.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-483
    type: reviews
---
# SR-15482: failure-domain review of QSL-974 quantity literal refusals (QSL PR #1214)

## Summary

Ticket: QSL-974. Examined: the FR-093 Refusals list (lines 210-227), FR-093-AC-34 and TC-483
step 6, against `qsl-semantics/src/check/check/typing.rs:941-961` (quantity literal typing),
`check/probabilistic.rs:669-692` and `probabilistic/exact.rs` (measure and zero denominator),
`check/refusal.rs:2441-2458` (`from_ill_typed`), `check/facts.rs:938-942` (quantity divisor) and
`check/check.rs:2882-2931` (negation).

Each listed refusal matches the code path that raises it. The four failure modes listed (unknown
unit or dimension name, wrong unit, zero denominator, measure over the ceiling) and the inherited
divisor refusal are each covered by a concrete AC-34 vector. These have no finding: no
non-rational magnitude can reach S3 (the grammar's `exact-value` has no float), and lead default 6
is sound.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | minor: a negative quantity literal is unstated. `exact-value` has no sign outside `rational(n, d)`, so `-3 ms` is a negation over `3 ms`, and `check`'s `negate` refuses a `Quantity` operand `ill_typed`/`operator-ineligible` (check.rs:2919). Line 182 mentions a sign on the numerator, but the only way to write one is `rational(-3, 1) ms`, and the Refusals list says nothing about `-3 ms`. State that a negative magnitude is written `rational(-n, d) u` and that `-3 ms` refuses `ill_typed`/`operator-ineligible`. Alternatively admit it as a literal of value `"-3"`, which is an owner call. Add the matching vector to AC-34 (or AC-33). | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:182,210-227 |
| FND-002 | low | minor: the second bullet covers only a position that takes another unit. A quantity literal where no quantity is taken (`g(3 ms)` with `g(x: Integer)`, `x + 3 ms` with `x: Integer`) is not listed. QSpec FR-142 refuses it `ill_typed`/`type-mismatch` ("an expression of any other type"). Widen the bullet to "its operand position takes another unit or no quantity". | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:215-218 |
| FND-003 | low | minor: refusal precedence is unstated. `check` measures and reduces the magnitude before it resolves the unit name (typing.rs:944-953), so `rational(1, 0) ghost` reports `undefined_expression`, not `missing-name`, and an over-ceiling magnitude with an unknown unit reports the limit. State the order (measure, zero denominator, unit name, then the position's type) so one test can pin it and a reimplementation cannot silently flip it. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:210-224 |

## Verdict

Mergeable on failure-domain grounds. All three are minor, and FND-001 is the one most likely to
surprise an author.

## Dispositions

Round 1, reviewed QSL #1214 at 3fd9a7398 (fix commit over b155d3ea3); code facts re-verified in the QSL tree at that head.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | negative literal is written `rational(-3, 1) ms` (vector n, value "-3"); `-3 ms` refuses `operator-ineligible` at the negation (vector m); matches check.rs negate (Quantity arm -> ineligible) and contextual() not treating a quantity negation as contextual |
| FND-002 | fixed | bullet widened to "another unit or no quantity"; vector g2 (`x = 3 ms`, `x: Integer`) refuses `type-mismatch` at the comparison: a quantity literal is not contextual, so it is inferred, and check_equality_in (quire-semantic-value ac0e137) falls to its TypeMismatch arm, raised at the equality's location |
| FND-003 | fixed | order stated as ceiling, zero denominator, unit name, position type, with vector `f(rational(1, 0) ghost)`; matches exact::value (bits, then Rational::new) before scope.unit_named in typing.rs |
