---
id: SR-20261104025135
title: "Spec review — QSL-1580 FR-091 predicate measure, FR-513-AC-4/AC-7, TC-1308 (PR #2243)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@c11158986; PR #2243 diff; FR-091 Predicate form, FR-513-AC-4, FR-513-AC-7, TC-1308 rows 8-10 and 16"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-513
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1308
    type: reviews
---

## Summary

Ticket: QSL-1580. Spec review of the amended statements, with integrity and criterion-strength sub-checks applied inline. QSpec was consulted by ID only, read-only from origin/main.

The FR-091 "Predicate form" amendment matches QSpec FR-146's function declaration shape `[decreases(m)]`. It leaves FR-091-AC-26 true, because that predicate has no measure. FR-513-AC-7 now names FR-267's `native-run-result/2` `disposition.stage` carrier (`compile` / `evaluate`) and the strict reader's refusals. That is testable, and the new test exercises it.

On FR-513-AC-4, the PR body says the old `10 / x` over `Int` "had no QSpec meaning" because, under QSpec FR-147/FR-149, `/` is never integer division. That rationale is inaccurate:

- QSpec FR-147-AC-11 states that under complete value a rational `/` with an expected rational result type promotes integer operands exactly under FR-149.
- QSpec FR-146 gives an `Int[..]` operand of a rational `/` its integer interval over `[1,1]`, and records `x != 0` as a nonzero fact for an integer stable path.

A probe confirms QSL implements this. `function F using v(x: Int[-10, 10]): Rational[-10, 10; 1, 10] pure { if x != 0 then 10 / x else rational(0, 1) }` admits, and the unguarded `10 / x` refuses `unproved-nonzero`. The legacy literal `10 / x > 1` has no expected rational type, so it is ill-typed (`ambiguous-literal`) and needed restating. Restating it only over `Rational[..]` loses the integer-operand divisor and its integer nonzero fact on this successor criterion. That is a narrowing of the retained case, not a necessity.

## Verdict

**CONDITIONAL.** Correct FR-513-AC-4 (and TC-1308 rows 9/10) to keep an integer-divisor case alongside the rational one, or drop the "no QSpec meaning" justification and record why the integer case is out of scope. The other statements are sound.

## Scope

- FR-091 "Predicate form" bullet: examined.
- FR-091-AC-26, FR-091-AC-30: context only.
- FR-513-AC-4: examined.
- FR-513-AC-7: examined.
- FR-046-AC-2: context only.
- TC-1308 rows 8, 9, 10, 16: examined.
- QSpec FR-146, FR-147 (by ID): context only.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-513-AC-4 and TC-1308 rows 9/10 replace the integer `10 / x` case with `rational(10, 1) / x` over `Rational[..]`, on the stated premise that `10 / x` over `Int` has no QSpec meaning. QSpec FR-147-AC-11 and FR-146 give an `Int[..]` operand of a rational `/` meaning (exact promotion under an expected rational type, plus the `x != 0` integer nonzero fact), and QSL admits it. Fix: add the integer case in a rational-typed context, for example `function F using v(x: Int[-10, 10]): Rational[..] pure { if x != 0 then 10 / x else rational(0, 1) }` admits and the unguarded form refuses `unproved-nonzero`, with a test. Otherwise state explicitly why it is excluded. | spec/functional/FR-513-name-the-successor-or-keep-of-each-retained-criterion-that-rides-a-retired-path.md:68 |
| FND-002 | low | The FR-091 "Predicate form" now says a predicate carries its `decreases` measure, but no FR-091 AC (TC-480/TC-481) asserts that S2 builds the measure form for a predicate. FR-091-AC-26 covers only the no-measure case, so the behaviour is covered only transitively by the TC-1308 spine admit. Fix: extend FR-091-AC-26 or AC-30 with a measured predicate. | spec/functional/FR-091-produce-value-forms-and-assemble-package-declarations.md:244 |
