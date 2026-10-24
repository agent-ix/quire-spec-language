---
id: SR-12276
title: 'Gap analysis of PR #1161: inline integer literal operands (QSL-940), gap-analysis'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@ca37b7138602ac416e40cbaa44cc6369a8f50bbd; git diff origin/main...ca37b7138 (PR #1161, branch qsl-940-inline-literal): qsl-semantics/src/check/lowering.rs and its tests, qsl-replay/src/spine/bind.rs and its tests, qsl-replay/src/execute/tests.rs, qsl-package/src/emit/tests.rs, checked against FR-092-AC-11, FR-093-AC-7, FR-093-AC-12, FR-093-AC-30, FR-357-AC-9, FR-357-AC-17, FR-367-AC-2, FR-367-AC-6; ticket QSL-940'
review_set: subset
date: '2026-10-21'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-093
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-092
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-367
  type: reviews
---

## Summary

Ticket: QSL-940. PR: quire-spec-language#1161, head ca37b7138. Method: quoin gap-analysis, planless. Plan completion: not assessed. This is a fresh pass at the head. It replaces the earlier round SR-13697, whose single finding (the lowering rule had no acceptance criterion) is fixed by the new FR-093-AC-30.

`quire matrix --scope .` (quire 0.36.2, engine 0.50.2) reports every examined criterion as `tagged`: FR-092-AC-11, FR-093-AC-7, FR-093-AC-12, FR-093-AC-30, FR-357-AC-9, FR-357-AC-17, FR-367-AC-2 and FR-367-AC-6.

The changed production code has an owning requirement, the FR-093 Literal row, and FR-093-AC-30 now covers it. No stub, placeholder or coverage inflation was found. The gaps are two FR-093-AC-30 clauses that its tests do not fully assert.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-093-AC-30 | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:830 | examined |
| FR-093 Literal row | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:139 | examined |
| FR-093-AC-7, FR-093-AC-12 (re-keyed group label, order and dependency lists) | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:807,812 | examined |
| FR-092-AC-11 | spec/functional/FR-092-key-type-parameter-and-declared-nodes.md | examined |
| FR-357-AC-9 | spec/functional/FR-357-replay-scalar-parity-claims.md | examined |
| FR-357-AC-17 | spec/functional/FR-357-replay-scalar-parity-claims.md | examined |
| FR-367-AC-2, FR-367-AC-6 | spec/functional/FR-367-bind-a-call-s-named-values-to-a-scalar-application-occurrence.md | examined |
| TC-415 step 12, TC-413 step 9, TC-416 steps 5-6, TC-1053 step 2 | spec/test-cases/ | examined |
| inline-literal branch of `next_operand`, `takes_inline_integer_literals` | qsl-semantics/src/check/lowering.rs:4032-4050,4825-4838 | examined |
| FR-358 step 6 | spec/functional/FR-358-settle-a-composite-bounded-shadow-item.md:256-273 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-093-AC-30 says that, for each listed body, "the checked graph holds no node for that literal". Only `an_integer_literal_right_operand_of_add_is_inline` (`x + 1`) asserts that the node is absent. For `1 + x`, `x - 1`, `x * 2`, `-5`, `x == 1` and `1 != x`, the tests check only the argument term. A producer that wrote the inline term and also kept a stray literal node would pass for those bodies. | qsl-semantics/src/check/lowering/tests.rs:2149-2172,2225-2282 |
| FND-002 | low | FR-093-AC-30 says that over `x: Int[0, 9]`, `x == 1` "compares `x` with a `numeric.narrow` application of the literal's node". The test asserts three things separately: the eq argument 1 is some reference, the narrow argument 0 is some reference, and L6 is held. It never checks that the eq argument targets the narrow node, or that the narrow argument targets L6. | qsl-semantics/src/check/lowering/tests.rs:2258-2268 |

## Verdict

Not clean. There are two low gaps in FR-093-AC-30's test assertions and no medium or high gap.

Each examined criterion has tests that assert it:

- FR-093-AC-30: the five new lowering tests, which cover every listed operator on both sides, nesting, the ordering and Boolean controls, and L6.
- FR-092-AC-11: `a_self_recursive_function_keys_to_g4_to_g6` and `a_standalone_integer_literal_is_a_node_l6`.
- FR-093-AC-7 and FR-093-AC-12: the qsl-package emit tests over the re-keyed label `d5012d73...`.
- FR-357-AC-9: index 0, plus the new two-parameter index-1 test.
- FR-357-AC-17: accepted inline literal, `NotInlineLiteral`, `LiteralValue` and `OperandChild` with `found: None`.
- FR-367-AC-2: `tc_1053_step_2_literals_repeats_and_unary_operands`.

The replay consumer of the eq/ne widening, FR-358 step 6's integer-literal branch, has no acceptance criterion or test that reaches it from compiled source. It is recorded once, as SR-12275 FND-002.

## Dispositions

Round 1, at head b643c5b8d. No lowering test changed between ca37b7138 and b643c5b8d.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | still-open | The absence of a literal node is still asserted only for `x + 1`. |
| FND-002 | still-open | The narrowed `x == 1` test still does not check that the eq argument targets the narrow node, or that the narrow argument targets L6. |
