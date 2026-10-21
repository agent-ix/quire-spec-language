---
id: SR-13697
title: 'Gap analysis of PR #1161: inline integer literal operands (QSL-940), gap-analysis'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@59112ebf2; git diff origin/main...59112ebf2 (PR #1161, branch qsl-940-inline-literal, merge base 0582efb14): qsl-semantics/src/check/lowering.rs and its tests, qsl-replay/src/spine/bind.rs and its tests, qsl-replay/src/execute/tests.rs, qsl-package/src/emit/tests.rs against FR-092-AC-11, FR-093 (Literal row), FR-357-AC-9, FR-357-AC-17, FR-367-AC-2; ticket QSL-940'
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

Ticket: QSL-940. PR: quire-spec-language#1161, head 59112ebf2. Method: quoin gap-analysis, planless. Plan completion: not assessed. `quire matrix --scope .` (quire 0.36.2, engine 0.50.2) reports FR-092-AC-11, FR-093-AC-1, FR-093-AC-7, FR-093-AC-12, FR-357-AC-9, FR-357-AC-17, FR-367-AC-2 and FR-367-AC-6 as `tagged`. The changed production code has an owning requirement: the FR-093 Literal row, amended in this PR. Its consumer side is FR-367 Behavior 4. No stub, no placeholder and no coverage inflation was found. One gap: the new lowering rule has no acceptance criterion of its own, and its tests are tagged to a replay criterion.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-093 Literal row (integer literal operand of add/sub/mul/negate is an inline term) | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:139 | examined |
| FR-092-AC-11 | spec/functional/FR-092-key-type-parameter-and-declared-nodes.md | examined |
| FR-357-AC-17 | spec/functional/FR-357-replay-scalar-parity-claims.md | examined |
| FR-357-AC-9 | spec/functional/FR-357-replay-scalar-parity-claims.md | examined |
| FR-367-AC-2 | spec/functional/FR-367-bind-a-call-s-named-values-to-a-scalar-application-occurrence.md | examined |
| inline-literal branch of `next_operand`, `is_scalar_operator` | qsl-semantics/src/check/lowering.rs:4035-4048,4826-4836 | examined |
| `Resolver::resolve` | qsl-replay/src/spine/bind.rs:525-545 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The producer rule (an integer literal operand of add, sub, mul or negate is inline, on either side and nested, and literal operands of other operators stay nodes) is stated only in the FR-093 Literal row, which no AC covers. Its four new tests are tagged `FR-357-AC-17`/`TC-904`, a criterion about the replay entry's refusal causes, not about lowering. | qsl-semantics/src/check/lowering/tests.rs:2127,2147,2173,2189; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:139 |

## Verdict

Not clean, with one medium gap. Each examined criterion has tests that assert it. FR-092-AC-11 is backed by `a_self_recursive_function_keys_to_g4_to_g6` and `a_standalone_integer_literal_is_a_node_l6`. FR-367-AC-2 is backed by `tc_1053_step_2_literals_repeats_and_unary_operands`. FR-357-AC-17 is backed by `tc_904_an_inline_literal_the_node_has_is_accepted_and_one_it_lacks_is_refused` and `tc_904_a_graph_child_that_is_not_the_nodes_child_is_refused`. Mutation results (SR-13696) show these tests detect a reverted scope, a dropped operator, a widened scope and a changed type node. The lowering rule itself needs an FR-093 acceptance criterion for its tests to trace to. The weakened FR-357-AC-9 coverage at operand index 1 is recorded as SR-13696 FND-002.
