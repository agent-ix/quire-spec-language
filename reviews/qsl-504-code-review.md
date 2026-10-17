---
id: SR-12069
title: "QSL-504 PR #933 code review: union and case node keys and checked-package v2 emission"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@24bdbe692ff82d7508cffa89f527bc367a682e65; PR #933: qsl-semantics/src/check/lowering.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/src/check/check.rs, qsl-semantics/src/check/node_key/mod.rs, qsl-semantics/src/check/sum_case.rs, qsl-package/src/emit.rs, and the tests in qsl-semantics/src/check/{node_key,sum_case,lowering}/tests, qsl-package/src/emit/tests, qsl-package/src/checked_v2/tests.rs, qsl-replay/src/execute/tests/union.rs, tests/it/union_nodes.rs"
review_set: subset
---

## Summary

Ticket: QSL-504. PR quire-spec-language#933, diff `origin/main...24bdbe69`
(17 files, +1638/-112). Code review with the rust-review lane folded in.
Read-only: no build, test or cargo run. The author's run4 gates (EXIT=0) are
taken as given, per the brief. Gates are not reviewed. QSpec is private and
is cited by id only.

Checked hard, per the brief:

1. Node-key determinism. The case arms come from `CaseBuilder::finish`, which
   sorts them by member position, so `CaseFrame` lowers them in declared
   member order. An arm binder is a `value`/`parameter` node keyed by name,
   level and type. Slot numbers and locations never enter a key, so trivia
   and arm reorder change no id. The level is `binders.scope.len()`, and the
   arm's binders are popped before the next arm, so sibling arms start at
   the same level, as QSpec FR-440 "Binder levels" requires (s=0, r=1, w=1,
   h=2 matches the published fixture's parameter nodes). The union type node
   is a binding per member over an aggregate of payload references. The
   arm is a binding over a two-member aggregate (the binder aggregate, then
   the body). `union_value` is one binding over the payload aggregate. All
   three match the published union fixture's bodies. `Operator::Case`
   serializes as `case`, and `LeafSegment::Member` as `member:<name>`.
   `typed_preimage` forces the application preimage only for
   `NodeTag::Value` plus `union_value`. `record_value` and `tuple_value`
   stay structural. The hand-written RFC 8785 bytes in
   `a_union_value_is_keyed_by_the_application_preimage_and_a_tuple_value_is_not`
   use the same member order (body, declaration, node_tag, recursion,
   semantic_form, semantic_type, version) as the FR-322 application
   preimage. Rebuilding that preimage from the fixture's two `union_value`
   nodes (Python, read-only) reproduces both published node ids. Rename,
   add and retype propagation is asserted transitively over every node that
   names the union.
2. Sealing. `seal_unions` runs once inside `check`, before any body is
   typed. It runs over the declarations, from a throwaway `Lowering` with
   the same scope, owner, models, lock evidence and identity limits as the
   real one. Before sealing, the code between `Scope::new` and the seal only
   resolves signatures and validates dispatch. Both compare types through
   `same_type`, which joins on handles whether or not a union is sealed.
   Construction typing uses `union_member_named`, which works before
   sealing. Body typing sees sealed unions, as the old pre-sealed fixture
   environment did, so behaviour is unchanged. `union_fixture` remains only
   for the replay and evaluator adversarial tests.
3. Recursive unions. In a union frame, `CompositeFrame::accept` collects
   payload keys. `advance_union` closes each member as a binding over its
   payload references, nullary members included. Self-reference uses the
   existing composite placeholder and `settle` SCC path. The recursive
   `List` test checks that `union_key` equals the emitted group member's id.
4. Mint boundaries. No `NodeKey::from_digest` or `EffectiveId` constructor
   is added in shipped code. The FR-441 keys use the existing
   `check::sum_case::mint_union_member_key`, called from
   `check::Scope::seal_unions`. The new `from_digest` calls are all in
   tests.
5. Removed refusals. `sum_case::unlowered`, `union_frame` and the three
   `UnsupportedFeature` union refusals are all deleted, along with the test
   that asserted them. Every union path now has a real arm, and the `S3`
   internal faults left behind are invariant guards only. Nothing is dead.
6. The emit seam. `omissions`, `emit_package_inner` and `ir_decodes` are
   private. Every production caller passes `&ir_decodes`, which is exactly
   the old `CheckedNodeKind::decode(..).is_some()`. `emit_package_decoding`
   is `#[cfg(test)]`. There is no public API change and no test code in
   shipped builds.
7. Replay. `qsl-replay/src/spine/refinement.rs` is untouched, and its
   `CheckCause` match is still exhaustive (`UnsupportedFeature` remains a
   variant). The new public `Operator::Case` and `LeafSegment::Member`
   variants are matched exhaustively in `node_key` (`semantic_form`,
   `Serialize`) and in `lowering`'s `WalkSegment::segment`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

PASS. No major bug found in the changed code. Union, construction and `case`
lowering match QSpec FR-440's spelling and FR-322's preimage selection.
Sealing happens once, before body typing, over the same content the real
lowering keys.

Not raised (below the brief's major-only bar, recorded for the planner):

- Nothing in production checks that the sealed union key equals the union
  node the real lowering emits. Equality holds by construction, because
  both lowerings key the same declarations under the same inputs. The tests
  check it end to end, for `Shape` and for recursive `List`.
- The sealing lowering runs on its own `Meter`, with no `with_node_limit`.
  Its type-closure work is not charged to the package `contract_meter`.
- Two comments are stale. The `union_fixture` comment in `check/mod.rs`
  says "no checked package can hold a union". The FR-110 comment near the
  staged type admission says that admission completes "before ... sealing",
  but union sealing now runs earlier.
