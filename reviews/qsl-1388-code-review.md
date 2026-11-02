---
id: SR-17040
title: "Code and Rust review: ConnectiveOrigin on checked connective nodes (QSL-1388, PR #18)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-eval PR #18 @730deae543360514f239da9bdda60a38694af214; crates/quire-checked/src/node.rs, crates/quire-checked/tests/node_projection.rs; FR-002-AC-1 through FR-002-AC-3"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-eval/FR-002", type: references }
---
# Code and Rust review: ConnectiveOrigin on checked connective nodes

## Summary

Ticket: QSL-1388 (related IR-1276). The PR adds `ConnectiveOrigin { Authored, Synthesized }` and the variant `NodeKind::Connective(Connective, ConnectiveOrigin, Child, Child)`. `map_ref` forwards the origin unchanged. Rust-review lane folded in.

## Verdict

PASS, mergeable. The carrier is the smallest typed one: a two-variant `Copy` enum, positional on the existing variant, no default and no shim. `NodeKind` derives only `Clone, Debug, PartialEq` (no Hash, Ord or serde), so origin enters equality only. A repo-wide grep finds the variant only in node.rs and the two test files; the repo has no graph construction, no interning and no keyed use of `NodeKind`. Mutation checks (force `Authored` in map_ref, swap operands, flip operator) each fail `connective_read_retains_origin_and_operand_order`. fmt, clippy (default, all-features, both no-std lib targets), doc -D warnings, test (default and all-features) and cargo deny pass.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Evidence

- Sharing risk: this repo never compares or dedups nodes. In QSL origin/main d1487ae the body builder (qsl-semantics/src/check/ir.rs BodyBuilder::push) appends without dedup, and the only interning found is the temporal formula arena keyed on its own Node type. So equal authored and synthesized applications stay two NodeIds as before; the change cannot alter sharing there. The risk exists only if a future consumer hash-conses on NodeKind, which would need a Hash derive the type lacks.
- Downstream QSL consumers pattern-match the old three-field variant (about 15 sites in qsl-eval, qsl-replay, qsl-semantics); they fail to compile until QSL adopts the new arity, which is intended (no compat layer).
- The AC-3 test exercises derived PartialEq; it fails if origin were removed from the variant, which is the regression it guards.

## New findings (disposition pass 3)

Head 8f7989902 on origin/main 880e19dac.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | high | A state clause whose decision path reaches a float ordering settles an InternalFault instead of a derivation. The ordering lowers to a Synthesized connective, which now records no decision, but derive_separating_witness calls decision(node)? on every connective it walks, with the stated invariant that every node reached was evaluated and recorded. A probe (temporary test in qsl-replay spine/clause/tests/witness.rs, reverted) with the invariant clause fl(float64(bits: 0x3ff0000000000000)) < fl(float64(bits: 0x4000000000000000)) over the witness fixture returned Err(InternalFault { stage: witness, invariant: decision-path-node-recorded }). The same occurs for a float ordering under not, let, the walked operand of and/or, or an implies operand. Before the origin change the decision-suppressing guard had the same effect, so this was introduced with FND-004's fix and no test exercises a float ordering in a state clause. | qsl-replay/src/witness/derivation.rs:60-95 (decision(node)? at the Connective arm) |

## Dispositions (round 3)

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-007 | fixed | The location heuristic is gone: evaluate.rs Task::Short records a decision only when matches!(origin, ConnectiveOrigin::Authored), read from the node (quire-eval 4e59f729 ConnectiveOrigin as second field of NodeKind::Connective). equality_matrix.rs connective_sites reads origin; no location comparison remains on the decision path (the only location comparison left in expression/mod.rs is an unrelated Query lookup). Every producer passes an origin: typing.rs accept_connective (the only producer of an authored connective, and/or/implies) uses Authored; the four float_ordering connectives use Synthesized; invariants.rs test builder uses Authored; evaluate.rs tests pass both. No other synthesized connective exists in lowering, expand or derivation (those only read connectives). Mutation: with the origin test replaced by an always-true condition, a_connective_records_a_decision_by_its_origin_not_its_location and a_float_ordering_records_no_decision_but_an_authored_connective_does both fail; with an always-false condition both fail. |
| FND-003 | fixed | Rerun on this head: with operand hoisting disabled, each_ordering_operand_is_evaluated_exactly_once and an_ordering_spends_each_operand_once_and_stops_at_its_limit fail. |
| FND-001, FND-002, FND-004, FND-005, FND-006 | fixed | Unchanged; the full qsl-eval, qsl-replay and qsl-semantics suites pass on this head. |

## Gaps for other lanes (round 3)

Spec lane: FR-265 and ADR-031 SW-2 do not state that a decision is recorded only for an authored connective, nor how the witness walk treats a Synthesized one; the rule exists only in code. FND-008 needs the same ruling (path ends at a synthesized connective with closed-scope, per SW-2's catch-all, is the natural reading).

## Verdict (round 3)

Not mergeable until FND-008 is fixed (derive_separating_witness must end the walk at a Synthesized connective, or otherwise not demand its decision, with a test deriving a clause whose body is a float ordering). Gate on this head, all green: cargo fmt --check, cargo check --workspace --all-targets, clippy -D warnings default and --no-default-features, arch-lint qualified-core, qsl-eval/qsl-replay/qsl-semantics tests (4414 passed, 0 failed), qsl-semantics --all-features tests. Lock moves legitimate: quire-eval and quire-checked to 4e59f729 (the merged ConnectiveOrigin commit), quire-semantic-value 3129a4cc (identical to that repo's main), quire-exact unchanged from main; no rev pins, Cargo.toml unchanged.
