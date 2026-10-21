---
id: SR-13703
title: 'Gap analysis of PR #1166: converted equality operands lower as numeric.convert (QSL-944), gap-analysis'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@fe77ab328; git diff origin/main...fe77ab328 (PR #1166, branch qsl-944-converted-equality, merge base 4cd371d40): qsl-semantics/src/check/check/typing.rs, qsl-semantics/src/check/check.rs, qsl-package/src/emit/tests/converted_equality.rs against FR-093 (application table, Equality and ConvertScalar rows), FR-093-AC-3 and the QSL-944 acceptance list; ticket QSL-944'
review_set: subset
date: '2026-10-21'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-093
  type: reviews
---

## Summary

Ticket: QSL-944. PR: quire-spec-language#1166, head fe77ab328. Method: quoin gap-analysis, planless (plan completion not assessed). Tooling: quoin 0.28.3, quire 0.36.2 (engine 0.50.2), spec-artifacts-process@b82e42e.

`quoin matrix --json` at head binds all six `converted_equality` tests to FR-093-AC-3 through `#[trace("FR-093-AC-3", "TC-415")]`. No production change lacks an owning requirement in the sense that FR-093's `ConvertScalar` and `Equality` rows cover the emitted nodes. But no requirement statement or AC says that a converted equality operand is a `ConvertScalar` node, or which operand's type picks the equality family (FND-001).

QSL-944 acceptance, judged against the head:

- AC 1, the ticket source emits `numeric.convert` on the Int operand plus `decimal.eq`, and strict IR admits it: met. `a_converted_decimal_equality_converts_the_int_operand` reads it back Verified and asserts the convert node's digest is argument 0.
- AC 2, Ne and reversed order: met, by the inequality and right-operand tests.
- AC 3, other families wherever the checker records a conversion: the code meets it, the tests do not (FND-002). `equality_typed` is the only recording site. Probes at head show Decimal to Int (`numeric.convert` + `integer.eq`, Verified), both operands converted (two converts + `decimal.eq`, Verified) and Quantity to Quantity (`quantity.convert` + `quantity.eq`/`.ne` over the convert node, either order) all lowered. On the merge base, Decimal to Int emitted `decimal.eq` and the strict read refused it; Quantity emitted `quantity.eq` over the raw operands.
- AC 4, titan:kani:plan confirmation: external, not assessed.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-093-AC-3 | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:803 | examined |
| FR-093 `Equality(operator, schedule)` row | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:175 | examined |
| FR-093 `ConvertScalar` rows | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:177-178 | examined |
| FR-093 identity-conversion sentence ("a `ConvertScalar` whose operand type equals its target builds no node") | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:226-228 | examined |
| TC-415 | spec/test-cases/TC-415-checked-expressions-lower-to-catalogued-fr-322-nodes.md | context_only |
| `converted_equality` tests (6) | qsl-package/src/emit/tests/converted_equality.rs | examined |
| QSL-944 acceptance list and the CG-needs sweep comment | Linear QSL-944 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No requirement owns converted-equality lowering. FR-093's `Equality` row picks `quire.op.<f>.eq` "for an operand family `<f>`" without saying which operand, or that a `convert<T>(e)` operand compares at `T`. No AC names a converted equality, so the six tests trace to the generic FR-093-AC-3 table AC, and the defect this PR fixes was consistent with the spec as written. Proposed FR-093 prose, after the `Coerce`/`ConvertScalar` no-node sentence: "**Converted equality operand.** An `Equality` operand written `convert<T>(e)`, whose `e` has a type other than `T`, is checked as a `ConvertScalar(T, e)` node of type `T`. It lowers by the `ConvertScalar` rows: `quire.op.numeric.convert`, or `quire.op.quantity.convert` for a quantity. The equality's family `<f>` is that of the compared type `T`, and its argument at that operand's position references the convert node, never `e`'s node. A `convert<T>(e)` whose `e` already has type `T` builds no node." Proposed AC (next free number): "`function f using v(a: Int[-100,100], b: Decimal[-100,100;0,0;nearest-even]): Boolean pure { convert<Decimal[-100,100;0,0;nearest-even]>(a) = b }` lowers to one `quire.op.numeric.convert` node over `a`'s parameter node, whose `result_type` is `b`'s type node, and one `quire.op.decimal.eq` with arguments `[ref(convert), ref(b)]`. `!=` gives `quire.op.decimal.ne`. `b = convert<..>(a)` puts the convert node second. `convert<Decimal[..]>(a) = convert<Decimal[..]>(c)` builds one convert per operand. A Quantity operand `convert<Km>(q) = r` lowers to `quire.op.quantity.convert` with mode `rounding` = `exact` and `quire.op.quantity.eq`. IR's v2 reader admits each package, and evaluating each gives the same result as comparing the converted values." | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:175,177-178,226-228,803 |
| FND-002 | medium | QSL-944 AC 3 ("other converted-equality families get the same treatment") has no test beyond Int to Decimal, Int to Rational, Int to wider Int and Decimal to wider Decimal. The CG-needs sweep's other cross-family rows (Decimal to Int, Rational to Int, Rational to Decimal, Decimal to Rational), a pair with both operands converted, and Quantity to Quantity (reachable through `PackageDeclarations`, and fixed at head as `quantity.convert` + `quantity.eq`) are unpinned. A regression in any of them would pass the suite. | qsl-package/src/emit/tests/converted_equality.rs:12-16,123-242 |

## Verdict

Not clean. The code satisfies QSL-944 AC 1 to 3, and AC 4 is external. The tests evidence AC 1 and 2 in full and AC 3 only in part (FND-002). The spec has no sentence that owns the behaviour (FND-001). It needs the proposed FR-093 paragraph and an AC naming converted equality, with the six tests retagged to that AC. The facts regression and the untested runtime half are recorded in SR-13702.
