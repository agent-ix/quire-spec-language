---
id: SR-1932001
title: "code review of PR #1932 (QSL-1337): rank requirement records once instead of building a BTreeMap over deep keys"
type: SpecReview
analysis: code-review
date: 2026-10-31
scope: "agent-ix/quire-spec-language; PR #1932 git diff origin/main...95843faf8 (qsl-semantics check/claims.rs, check/claims/map.rs, check/claims/tests.rs, check/mod.rs; qsl-route request.rs; qsl-replay call_requirements.rs; qsl-analyze tests schedulability_routing, probabilistic_request_items)"
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-062
  type: reviews
---
# SR-1932001: code review of PR #1932 (QSL-1337)

## Summary

Ticket: QSL-1337. Code review with the rust-review lane folded in, scoped to the PR diff at head 95843faf8a6c0af1c4e2a4d37d85f209c2189a1c. Major bugs only. `cargo test -p qsl-semantics --all-features --lib -- claims` (skipping deep_bodies and 100_000) passed 21 of 21, and `cargo check --workspace --all-targets --all-features` passed, so every caller of `CheckedGraph::requirements()` compiles against `RequirementMap`.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| RequirementMap | qsl-semantics/src/check/claims/map.rs | examined |
| key_claims return | qsl-semantics/src/check/claims.rs:974-1101 | examined |
| assembly entry matches | qsl-semantics/src/check/mod.rs:5066-5772 | examined |
| finish_step_relations | qsl-semantics/src/check/mod.rs:5861-6004 | examined |
| CheckedGraph::requirements | qsl-semantics/src/check/mod.rs:6997-7004 | examined |
| OccurrenceKey Ord | qsl-foundation/src/source/provenance.rs:188-192 | context_only |
| key_order::sorted_by_key | qsl-semantics/src/check/key_order.rs:24-40 | context_only |
| callers | qsl-route/src/request.rs, qsl-replay/src/call_requirements.rs, qsl-analyze/tests/it | examined |
| FR-062-AC-13 | spec/functional/FR-062-implement-checked-family-contract.md:379 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**Approve: no major bugs.**

1. Iteration order. `OccurrenceKey` derives `Ord` over `(node, origin)` in that field order. `RequirementMap::ranked` sorts by origin, then applies `key_order::sorted_by_key` by node, which is a stable sort documented to give exactly `NodeKey::cmp` order. A stable sort on the major field after a sort on the minor field gives lexicographic `(node, origin)` order, which is the derived order. Keys come out of a `HashMap`, so they are distinct and there are no ties that could depend on hash order. This is the same ranking main already did in `key_claims` before collecting into a `BTreeMap`. The `agrees_with_a_btree_map` test checks it against a `BTreeMap` built from the same records, using a 200-deep chain and two origins.
2. Duplicates and faults. The 21 Entry matches keep the same arms: Vacant inserts, an equal record is accepted where it was before, and anything else returns `UnkeyableRequirements`. The first error still comes from the order the program inserts records, never from map iteration. In `finish_step_relations`, a duplicate among the relation records still faults at once. A collision with a key the package already holds now faults after the loop, through `extended`, and gives the same `CheckRefusal` (`root(Origin::Expression)`, `InternalFault(UnkeyableRequirements)`). The only difference is which of two internal faults is reported when both happen, for example a later `StageFault`. That is below the major-only bar. The loop never reads `graph.requirements`, so delaying the merge changes no lookup.
3. HashMap leaks. Between `key_claims` and `requirements.into()` the assembly only calls `entry`. `key_claims` already used a `HashMap` internally on main. `extended` and `FromIterator` both rank before anything can read them. No hash order reaches output, diagnostics or the choice of first error.
4. Debug and Eq. `Debug` is `debug_map` over sorted entries, the same format a `BTreeMap` uses. The test asserts the two strings are equal. `Eq` compares the sorted `entries` Vecs, which is the same as `BTreeMap` equality. The `index` field is derived from `entries`, so leaving it out of `Eq` is correct. TC-160 "checking twice gives equal records" and request_builder's `assert_eq!(records, ...)` keep their meaning.
5. Public API. No FR, AC, ADR or TC names `BTreeMap<OccurrenceKey, RequirementRecord>`. FR-062-AC-13 and TC-160 speak of "the map" and its contents and equality, and the new type keeps both. Workspace callers compile.

Rust lane: no `unsafe`. The one new panic is `Index`, which mirrors `BTreeMap`'s documented panic. `Hash` and `Eq` on `OccurrenceKey` are both derived, so they agree. `extended` rebuilds all records once per package, which is linear in size.
