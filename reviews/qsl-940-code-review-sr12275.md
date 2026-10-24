---
id: SR-12275
title: 'Code review of PR #1161: inline integer literal operands (QSL-940), code-review with rust-review lane'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@ca37b7138602ac416e40cbaa44cc6369a8f50bbd; git diff origin/main...ca37b7138 (PR #1161, branch qsl-940-inline-literal): qsl-semantics/src/check/lowering.rs, qsl-semantics/src/check/lowering/tests.rs, qsl-replay/src/spine/bind.rs, qsl-replay/src/spine/bind/tests.rs, qsl-replay/src/spine/wide_integer_tests.rs, qsl-replay/src/execute/tests.rs, qsl-package/src/emit/tests.rs, and the spec text the change amends; ticket QSL-940'
review_set: subset
date: '2026-10-21'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-093
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-358
  type: references
- target: ix://agent-ix/quire-spec-language/FR-367
  type: reviews
---

## Summary

Ticket: QSL-940. PR: quire-spec-language#1161, head ca37b7138. Method: code-review with the rust-review lane folded in. This is a fresh single-reviewer pass over the whole PR diff at the head. It replaces the earlier review round SR-13696 to SR-13698 (run B7F1A383), whose findings the head commit fixed. Two of that round's disposition findings (its FND-004 and FND-005) are still open at this head. They are restated here as FND-001 and FND-002.

The producer change is in `Lowering::next_operand`. When the application's identity is `quire.op.integer.add`, `sub`, `mul`, `negate`, `eq` or `ne` and the operand is a bare checked `Literal(Integer)`, the operand is pushed as an inline `LeafTerm::literal` typed at the builtin `Integer` node (T2). The lowering no longer descends into a `value`/`literal` node for it. `qsl-replay` drops `Resolver.checked`, `Resolver::literal_value` and the `node` field of `OperandSource::InlineLiteral`. No other code in the workspace uses them.

Checked by hand:

- The branch fires only on a bare literal. A literal that check narrows to a bounded peer (`x: Int[0, 9] == 1`) or coerces stays a node.
- The re-entry into `next_operand` is bounded by the operand count of a unary or binary operator.
- Every checked integer literal has type `Integer`, so typing the inline term at T2 matches the type of the standalone node.
- Contract IR at the locked revision 7c4367d admits an inline integer literal term in the arguments of both scalar applications (`scalar_operands.rs`) and comparison applications (`comparison_operands.rs`). The new eq/ne terms are therefore admissible.
- The composite arm (`composite_site::locate`) reads operand values from the checked AST, not from the graph, so an inline eq/ne literal still gets its package value.
- `stratum::is_stratified_body` accepts a literal leaf in an application's arguments.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| `Lowering::next_operand` inline-literal branch | qsl-semantics/src/check/lowering.rs:4032-4050 | examined |
| `takes_inline_integer_literals` | qsl-semantics/src/check/lowering.rs:4825-4838 | examined |
| `OperandSource::InlineLiteral`, `Resolver::resolve` | qsl-replay/src/spine/bind.rs:134-140,525-545 | examined |
| new lowering tests (FR-093-AC-30, FR-092-AC-11) | qsl-semantics/src/check/lowering/tests.rs:2099-2282 | examined |
| adapted TC-904 operator-arm tests, including the new index-1 test | qsl-replay/src/execute/tests.rs:2644-3660 | examined |
| adapted TC-1053 bind tests, wide-integer test, recursion-group label test | qsl-replay/src/spine/bind/tests.rs, qsl-replay/src/spine/wide_integer_tests.rs, qsl-package/src/emit/tests.rs | examined |
| composite arm operand identities and parity preimage | qsl-replay/src/execute/scalar_site.rs:183-230, qsl-replay/src/execute/composite_parity.rs:150-212, qsl-replay/src/execute/composite_site.rs:64-162 | examined |
| FR-358 step 6 identity tie | spec/functional/FR-358-settle-a-composite-bounded-shadow-item.md:256-273 | examined |
| FR-093 Literal row, FR-093-AC-30 | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:139,830 | examined |
| Contract IR inline-literal admission | quire-contract-model checked_package/v2/scalar_operands.rs, comparison_operands.rs (rev 7c4367d) | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-358 step 6 still says without qualification that "an integer literal operand is `inline_literal` with the singleton `range`". The FR-093 Literal row amended in this PR, the code and FR-093-AC-30 keep the literal of `x == 1` over `x: Int[0, 9]` as a node: the operand is a `numeric.narrow` application. The composite arm then keys that operand `graph_child` with the empty `bounds` domain. A consumer such as CG that follows FR-358 step 6 would compute `inline_literal` plus the singleton range and get a different obligation identity, which refuses `ScalarIdentity::Obligation`. Bounded integer peers are the common case. FR-358 step 6 needs to name the narrowed-literal case, or the producer needs to inline it. | spec/functional/FR-358-settle-a-composite-bounded-shadow-item.md:270-271; qsl-replay/src/execute/composite_parity.rs:201-205; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:139 |
| FND-002 | medium | The widening to integer `eq`/`ne` exists for FR-358 step 6. It makes the composite arm's `(Operand::Literal { Integer }, OperandIdentity::InlineLiteral)` branch reachable from compiled source for the first time (`x: Integer == 1`), but no replay test drives it. No test builds a composite claim over a compiled integer `==`/`!=` with a literal operand and checks the identity (`inline_literal`, singleton range), admission of the literal, or settlement. The only composite-parity test touching `InlineLiteral` panics on it ("a composite literal is a graph child"). | qsl-replay/src/execute/composite_parity.rs:192-199; qsl-replay/src/execute/tests/composite_parity.rs:1725 |
| FND-003 | low | `takes_inline_integer_literals` is declared after `#[cfg(test)] mod tests;` at the end of `lowering.rs`. It is a production item placed after the test module, and it sits far from its single caller in `next_operand`. | qsl-semantics/src/check/lowering.rs:4822-4838 |

## Verdict

Not clean. There is no major finding, and the producer change is correct for what it covers.

- The inline branch is narrow and bounded, and it is typed the same way as the node it replaces.
- Contract IR at the locked revision admits the new eq/ne inline terms.
- The deleted replay fields have no users.
- The round-1 findings (eq/ne scope, index-1 refusal coverage, the dead `(7, 7)` range) are fixed at this head. `tc_904_a_second_operand_outside_its_range_is_refused_at_index_1` restores a two-parameter, index-1 refusal over compiled source.

FND-001 is a spec inconsistency that a cross-repo consumer would trip on, and it needs a text change in this PR. FND-002 is missing consumer-side coverage for the stated purpose of the eq/ne widening. FND-003 is placement only.

## Dispositions

Round 1, at head b643c5b8d. While this review ran, the coder pushed dde724464, 63543b3f5, a merge of origin/main (dfd3aee39) and b643c5b8d. Those commits answer the parallel round-1 reviewer's FND-004 to FND-006 (SR-13696), which are the same defects as FND-001 and FND-002 here.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed dde724464 | FR-358 step 6 now separates the two cases. An integer literal the checker leaves unwrapped (an `Integer` peer) is `inline_literal` with the singleton `range`. One wrapped in a `numeric.narrow` application (a bounded peer) is `graph_child` of the narrow node with the empty `bounds`. 63543b3f5 amends ADR-013 O-09 to match, and the FR-093 row, FR-093-AC-30 and TC-415 step 12 agree. |
| FND-002 | fixed dde724464 | `tc_907_an_integer_literal_operand_is_inline_unless_the_checker_narrows_it` compiles `k(a: Integer) { a = 3 }` and `m(a: Int[0, 9]) { a = 3 }`. It asserts `InlineLiteral` with range (3, 3) for `k`, and a non-parameter `GraphChild` with a `bounds` domain for `m`, through `parity_preimage`. |
| FND-003 | still-open | `takes_inline_integer_literals` is still declared after `#[cfg(test)] mod tests;` at the end of lowering.rs. |

## New findings (disposition pass 1)

Re-run at head b643c5b8d. `cargo test -p qsl-semantics -p qsl-replay -p qsl-package` with `QSPEC_DIR` at QSpec origin/main fc98b5e6 gives three failures. Two of them, `conformance_c14_source_map_lookup_over_qspec_positive_fixtures` and `conformance_i2_read_over_qspec_checked_package_v2_fixtures`, already failed at ca37b7138 and in the earlier round: Contract IR refuses QSpec's own fixture node 46 with `OperatorIneligible`. This PR does not touch that path. The third failure is new. Clippy with `--all-targets --all-features -D warnings` on the three crates exits 0.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | high | b643c5b8d rewrites the golden `add` and `mul` cases to literal operands (`7 op -2`), which lower inline. At ca37b7138 `conformance_emitted_application_nodes_match_qspec_positive_fixtures` passed; at this head it fails against QSpec origin/main fc98b5e6, whose `positive-operation-identities.json` and `positive-union-nodes.json` still give `quire.op.integer.mul` and `add` two `reference` arguments (`left: [["literal"], ["literal"]]`, `right: [["reference"], ["reference"]]`). The test passes against quire-specification#323 head 3b17ea8a, which rewrites those fixtures. Merging #1161 before #323 leaves QSL's `make ci` red on QSpec main. Merge QSpec #323 first, or in the same batch. | qsl-package/src/emit/tests/golden.rs:109-119,226-241; qsl-package/src/emit/tests/golden.rs:414 |
