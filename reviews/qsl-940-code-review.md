---
id: SR-13696
title: 'Code review of PR #1161: inline integer literal operands (QSL-940), code-review with rust-review lane'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@59112ebf2; git diff origin/main...59112ebf2 (PR #1161, branch qsl-940-inline-literal, merge base 0582efb14): qsl-semantics/src/check/lowering.rs, qsl-semantics/src/check/lowering/tests.rs, qsl-replay/src/spine/bind.rs, qsl-replay/src/spine/bind/tests.rs, qsl-replay/src/spine/wide_integer_tests.rs, qsl-replay/src/execute/tests.rs, qsl-package/src/emit/tests.rs; ticket QSL-940'
review_set: subset
date: '2026-10-21'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-093
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-357
  type: references
- target: ix://agent-ix/quire-spec-language/FR-358
  type: references
- target: ix://agent-ix/quire-spec-language/FR-367
  type: reviews
---

## Summary

Ticket: QSL-940. PR: quire-spec-language#1161, head 59112ebf2. Method: code-review with the rust-review lane folded in. The producer change in `Lowering::next_operand` lowers an integer `Literal` operand of an application whose identity is `quire.op.integer.add`, `sub`, `mul` or `negate` to `LeafTerm::literal` typed at the builtin `Integer` node (T2) instead of descending to a `value`/`literal` node. `qsl-replay` drops `Resolver.checked`, `Resolver::literal_value` and `OperandSource::InlineLiteral.node`; none of them has a remaining user in the workspace.

Evidence run at the head:

- `cargo test -p qsl-semantics -p qsl-replay -p qsl-package -p qsl-analyze --no-fail-fast`: 3,629 passed, 0 failed.
- The same crates with `QSPEC_DIR` set to quire-specification origin/main (fc98b5e65), `conformance` filter: only the two known failures, `conformance_c14_source_map_lookup_over_qspec_positive_fixtures` and `conformance_i2_read_over_qspec_checked_package_v2_fixtures`. `conformance_emitted_application_nodes_match_qspec_positive_fixtures` and the two-owner fixture test pass.
- Mutations in a scratch copy, `qsl-semantics` lowering tests and `qsl-replay` spine tests. M1, making the scope never match, fails 6 tests. M2, typing the inline term at the Boolean node, fails 3 golden-vector tests and 11 `tc_1053` tests. M3, dropping `negate`, fails `integer_literals_are_inline_on_either_side_of_each_scalar_operator` and `a_directly_negated_two_pow_127_is_one_literal`. M4, widening to `integer.gt`, fails `literal_operands_of_other_operators_stay_nodes` and 3 vector tests. M5, dropping `sub`, fails 4 tests. All five mutants are killed.
- Mutation M6, limiting operand admission in `qsl-replay/src/scalar.rs` `compare` to the first operand with `.enumerate().take(1)`, passes all 746 `qsl-replay` lib tests. It survives.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| `Lowering::next_operand` inline-literal branch | qsl-semantics/src/check/lowering.rs:4035-4048 | examined |
| `is_scalar_operator` | qsl-semantics/src/check/lowering.rs:4826-4836 | examined |
| `Lowering::integer_literal`, `Lowering::literal` (type of the inline term against the standalone node) | qsl-semantics/src/check/lowering.rs:1844-1851,4439-4482 | examined |
| `OperandSource::InlineLiteral`, `Resolver::resolve` | qsl-replay/src/spine/bind.rs:134-140,525-545 | examined |
| new producer tests | qsl-semantics/src/check/lowering/tests.rs:2099-2216 | examined |
| adapted TC-904 operator-arm tests | qsl-replay/src/execute/tests.rs:2644-3610 | examined |
| adapted TC-1053 bind tests, wide-integer test, recursion-group label test | qsl-replay/src/spine/bind/tests.rs, qsl-replay/src/spine/wide_integer_tests.rs, qsl-package/src/emit/tests.rs | examined |
| composite arm operand identities | qsl-replay/src/execute/scalar_site.rs:183-230, qsl-replay/src/execute/composite_parity.rs:160-205 | context_only |
| checked integer literal type | qsl-semantics/src/check/check/typing.rs:902-913 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The producer scope stops at the four FR-357 operators, but FR-358 step 6 also requires an integer literal operand of an `Equal`/`NotEqual` application to be `inline_literal` with the singleton range, "as FR-357". For `x == 3` the literal stays a node, so the composite arm's `operand_identities` reads `GraphChild` and `parity_preimage` gives it the empty `bounds` domain, not the `range` FR-358 names. Either the scope or FR-358 step 6 must change. | qsl-semantics/src/check/lowering.rs:4826-4836; qsl-replay/src/execute/scalar_site.rs:207-226; qsl-replay/src/execute/composite_parity.rs:200-205; spec/functional/FR-358-settle-a-composite-bounded-shadow-item.md:270-273 |
| FND-002 | medium | The adapted TC-904 tests no longer cover a refused operand at index 1. `add(1, 101)` became `add(101, 1)` (index 0) and the `with_literal(8)` case was deleted. A mutant that admits only the first operand survives every qsl-replay lib test, and the parity unit has no two-parameter application through which index 1 could still be reached. | qsl-replay/src/execute/tests.rs:2936-2968; qsl-replay/src/scalar.rs:654-658 |
| FND-003 | low | In `tc_904_an_operand_outside_its_range_is_refused`, `with_literal` still builds the right operand with `literal.range` `(7, 7)` and value 1, but `claim()` replaces it with `ScalarOperand::literal(1)`. The `(7, 7)` range is dead and misleading, and the test's FR-357-AC-9 singleton clause now rests only on the `ScalarOperand::literal(7)` constructor assertion. | qsl-replay/src/execute/tests.rs:2951-2967 |

## Verdict

Not clean, but the producer change is correct for what it covers. The branch fires only for a bare checked `Literal(Integer)` operand of the four FR-357 identities. A `Coerce` or `narrow` wrapper is still a node, as it should be. The `Keyed::Application` arms are the only producers of those identities. The re-entry into `next_operand` is bounded by the operand count. Typing at T2 through `integer_literal` gives the same type the standalone path gives, because every checked integer literal has `ValueType::Integer` (typing.rs:910). L5 and L6 still carry T2. The deleted consumer fields have no users, and `InlineLiteral` maps one-to-one onto FR-357's `OperandIdentity::InlineLiteral`. The replay and package tests were adapted with care. The `[-150, 150]` range and `mul(100, 2)` keep the out-of-range branch (200 > 150). Re-pinned literal values (`add(41, 1)`, `sub(39, 1)`, `mul(21, 2)`) follow the compiled literals. The new AC-17 test exercises acceptance, `NotInlineLiteral` and `LiteralValue`. FND-001 is a scope question for the lead. FND-002 is a coverage regression the PR introduced, and a test over a two-parameter application would restore it.

## New findings (disposition pass 1)

Re-reviewed at head ca37b7138 (fix commit ca37b7138 over 59112ebf2). The fix widens the inline rule to `quire.op.integer.eq` and `ne`. Under the lead's ruling, a literal behind a `numeric.narrow` conversion stays a graph child.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | Over a bounded peer (`x: Int[0, 9]`, `x == 1`) the literal is a `Coerce` whose application is `numeric.narrow`. The composite arm classifies that operand as `Operand::Literal` with an integer value (composite_site.rs evaluates it closed), but the package argument is a reference, so `parity_preimage` keys it `graph_child` over the narrow node with the empty `bounds` domain. That is FR-358 step 6's composite-literal rule applied to an integer literal operand, which step 6 says is `inline_literal` with the singleton `range`. The add/sub/mul/negate precedent does not cover this: arithmetic operands are never coerced, so no FR-357 operand is ever a narrowed literal. Bounded parameters are the usual case, so integer `eq`/`ne` claims mostly land on this path. Either FR-358 step 6 must state the narrowed case, or the producer must inline it. | qsl-semantics/src/check/lowering.rs:4036-4048; qsl-replay/src/execute/composite_site.rs:140-162; qsl-replay/src/execute/composite_parity.rs:199-205; spec/functional/FR-358-settle-a-composite-bounded-shadow-item.md:266-273 |
| FND-005 | low | No replay-side test drives FR-358 step 6's integer-literal branch, (`Operand::Literal` integer, `InlineLiteral`) giving the singleton `range` domain, from a source-compiled package. Producer emission is tested only in qsl-semantics. The composite-parity tests cover only the composite-literal `graph_child` arm (tests/composite_parity.rs:1725). | qsl-replay/src/execute/composite_parity.rs:192-198; qsl-replay/src/execute/tests/composite_parity.rs:1700-1730 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `takes_inline_integer_literals` adds `quire.op.integer.eq` and `ne`; `integer_literals_are_inline_operands_of_integer_equality` covers both operators on both sides over `Integer`; dropping eq/ne, or ne alone, fails it (M7, M8). The narrowed-literal case is FND-004. |
| FND-002 | fixed | `tc_904_a_second_operand_outside_its_range_is_refused_at_index_1` over a two-parameter `x + y` asserts `RefusedInput(OperandRefusal { index: 1 })`; the `.enumerate().take(1)` mutant of scalar.rs `compare` now fails it. |
| FND-003 | fixed | `with_literal` and its dead `(7, 7)` range are removed; the test asserts `ScalarOperand::literal(7).range == (7, 7)` and an agreeing `add(2, 1)`. |
