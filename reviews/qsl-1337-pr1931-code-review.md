---
id: SR-1931001
title: "code review (with rust-review) of PR #1931 (QSL-1337): rank node keys by structure instead of comparing deep keys in check"
type: SpecReview
analysis: code-review
date: 2026-10-31
scope: "agent-ix/quire-spec-language; PR #1931 git diff origin/main...a24b23b59 (qsl-semantics/src/check/{key_order,claims,family,lowering,mod}.rs, lowering/tests.rs, lowering/tests/depth.rs)"
review_set: subset
---
# SR-1931001: code review of PR #1931 (QSL-1337)

## Summary

Ticket: QSL-1337. Head a24b23b59, one commit over origin/main. Rust-review lane folded in. Major bugs only, per the brief. Not built: the verdict rests on reading the diff and quire-exact's `NodeKey` `Ord`/`Eq`/`Hash` at the locked rev 6ae189e.

Examined, clean:

- key_order.rs gives exactly `NodeKey::cmp`'s order. quire-exact's cmp (node.rs:575-623) is the lexicographic order of a self-delimiting token stream (`Package < PathEnd < Text < NodeStart < Ordinal`), which equals the recursive order: package, then path segments pairwise (text by str, text before node, node by recursive order, shorter path first), then ordinal. The "largest aligned child" descent with `deferred_order` is the same order: a deeper non-equal suffix/ordinal overrides a shallower one only when the deeper children compare equal, which is the recursive rule. `Ranking::compare` implements that recursive rule with child ranks in place of child keys. Ranks are valid because classes are placed children-first and a treap insert never changes the relative in-order of existing classes. Equal keys built separately share one id via the `HashMap` (`Eq` = structural hash and cmp equal), so ties and stability hold (`keyed.sort_by_key` is stable). The treap `insert`/`rotate_up` keep in-order and subtree sizes; `rank` and `positions` agree.
- No HashMap iteration leaks into observable order. Every hash-ordered walk was checked: `finish`'s generated-occurrence loop (ordinals are per node, so cross-node order is irrelevant); `parameter_spans` via `iter_any_order` (the removed site key includes the node, so only that node's own (role, ordinal) order matters, which is kept); `roots` (keeps the least `Location`, a derived total order); `relax` (ties are equal anchors, so either tie order yields the same anchors); `enum_members` member order (only feeds `relax`'s worklist, whose result does not depend on visit order). All other converted maps (`ordinals`, `composites`, `composites_open`, `composite_placeholders`, `text_reach`, `entered_at`, LeafWalk `reach`) are point lookups only. Outputs that leave the stage (`type_nodes`, `Lowered.composites`, `key_claims` records) are re-collected into `BTreeMap`s after ranking; `key_claims` sorts by origin then stably by node, which is `OccurrenceKey`'s derived (node, origin) order. `type_nodes` keeps last-wins on a duplicate key, as `BTreeMap::insert` did (`from_iter` dedups keeping the last).
- `OccurrenceMap`: `iter` / `iter_role` remain ascending by (node, role, ordinal). `has` is equivalent (no empty role maps are ever created). `first_origin` returns the same origin as the replaced `iter_role(Claim).find(node == key && at == location)`: the first matching ordinal of that node's Claim spans.
- SemanticGraph ranked order: `rank()` runs first in `finish`, and no node is added between it and the hand-off. After that, `insert_finalized` only inserts a key absent from `nodes`, so the `binary_search` (`NodeKey::cmp`, the same order the ranking produced) lands on `Err` and inserts at its place; `remove` and `with_test_narrow_over` keep `order` in step. No graph is read through `nodes()` before `finish` (lowering submodules use `nodes.get` only).
- Test rewrite (`a_100000_deep_option_type_keys_on_a_small_stack`): `key_shapes` is injective (shape i fixes key i's package, texts, ordinal and child indices, and children are numbered deterministically), so equal shape lists mean equal ordered key lists. That is at least as strong as the old `BTreeSet` equality.
- No once-cells, `Lazy`, `RefCell` or global state added (FR-062-AC-3).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**Clean for major bugs.** Not a defect, recorded for the lead: collecting a ranked Vec into a `BTreeMap` still makes n-1 adjacent `NodeKey::cmp` calls, and `insert_finalized`'s `binary_search` compares keys directly. Both cost depth on a deep shared chain. Today they only touch declaration-level keys (type nodes, composites, claim records, finalized nodes), not the option/body chains the timed tests exercise.
