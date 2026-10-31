---
id: SR-1947001
title: "code review of PR #1947 (QSL-1358): linear Debug for the checked package source map"
type: SpecReview
analysis: code-review
date: 2026-10-31
scope: "agent-ix/quire-spec-language; PR #1947 git diff origin/main...516b5f9b9 (qsl-foundation/src/source.rs, qsl-foundation/src/source/node_table.rs, qsl-foundation/src/source/provenance.rs, qsl-semantics/src/library/mod.rs)"
review_set: subset
relationships:
- target: ix://agent-ix/quire-spec-language/FR-264
  type: references
---
# SR-1947001: code review of PR #1947 (QSL-1358)

## Summary

Ticket: QSL-1358. This is a code review with the rust-review lane folded in. It is scoped to the PR diff at head 516b5f9b9 and covers major bugs only.

Focused runs:

- `cargo test -p qsl-foundation --lib source::`: 14/14.
- `cargo test -p qsl-semantics --lib library`: 19/19.
- `cargo clippy -p qsl-foundation -p qsl-semantics --all-targets --all-features -- -D warnings`: clean.
- `cargo fmt --all -- --check`: clean.

Local reviewer probes, not pushed:

- One 100,000-deep key was interned, formatted and dropped on a 512 KiB stack in 168 ms (3.47 MB of output).
- Interning cost was compared for Arc-shared and unshared keys.
- `from_entries` was timed and profiled with perf.
- The FR-264-AC-1 deep_bodies test was run at this head.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| NodeTable, intern, position | qsl-foundation/src/source/node_table.rs:36-110 | examined |
| Nodes / Row Debug | qsl-foundation/src/source/node_table.rs:112-136 | examined |
| PackageSourceMap Debug | qsl-foundation/src/source/provenance.rs:270-300 | examined |
| ImportView Debug | qsl-semantics/src/library/mod.rs:294-315 | examined |
| NodeKey Eq/Ord/Hash/Drop (quire-exact 6ae189e) | quire-exact src/node.rs:205-300,575-680 | context_only |
| V2Read | qsl-package/src/checked_v2.rs:546-559 | examined |
| CheckedPackage / CheckedGraph | qsl-package/src/checked.rs:132; qsl-semantics/src/check/mod.rs:582-640 | examined |
| emit::admitted_key, package_source_map | qsl-package/src/emit.rs:406-440; qsl-package/src/checked_v2.rs:352-375 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The PR leaves most of the ticket's Debug paths quadratic. V2Read and CheckedPackage Debug still write a full nested NodeKey or CheckedNodeId for every key. In this repo: CheckedGraph's derived Debug over parameter_spans, type_nodes, model_roots, occurrences (OccurrenceMap), requirements (RequirementMap), model_correspondence and semantic_graph. In quire-contract-ir: V2Read.admitted's CheckedPackageV2 Debug (node_index and source_index are keyed by CheckedNodeId, whose derived Debug is quadratic and recursive), and timed_models. FR-264-AC-1 is therefore not shown. At this head its test was still in emit (recorded_occurrences, BTreeMap<NodeKey> insertion) after 27 minutes and never reached the read or Debug. | qsl-package/src/checked_v2.rs:546-559, qsl-package/src/checked.rs:132, qsl-semantics/src/check/mod.rs:582-640, qsl-semantics/src/check/family.rs:1421, qsl-semantics/src/check/claims/map.rs:161, qsl-semantics/src/check/lowering.rs:308, qsl-semantics/src/check/identity.rs:269 |
| FND-002 | medium | The NodeTable lookup is correct but takes quadratic time on wire-read keys. The bucket fallback is a full `NodeKey ==`, which is O(1) only for the same Arc. Keys that are structurally equal but not shared are compared level by level, at O(depth) per lookup. `admitted_key` rebuilds every source-map entry's tree separately, so the V2Read case is exactly the unshared case. Measured at n=2000/4000/8000: shared 2.9/10.8/15.7 ms, unshared 0.57/3.0/10.7 s. Output stays linear, but the "linear" docs are true only for size. | qsl-foundation/src/source/node_table.rs:89-101, qsl-foundation/src/source/node_table.rs:10-11, qsl-foundation/src/source/provenance.rs:270-272, qsl-package/src/emit.rs:406 |

## Verdict

**Changes requested. Not mergeable as the QSL-1358 fix.** The code that is here is correct and stack-safe.

What holds:

1. Identity. `position` buckets on the cached structural hash. It then picks a candidate only by full `NodeKey ==` (`structural_hash` equal and `Ord::cmp` equal), so two distinct nodes with a colliding hash get distinct positions. A node shared at two depths is found by the same lookup and gets one position. Positions are assigned in BTreeMap entry order and by a LIFO work list. Buckets are a `BTreeMap<u64, _>`. No HashMap order enters the output, so it is deterministic. `PassThrough` rebuilds the u64 deterministically, and only bucketing uses it.
2. Stack. `intern` walks with an explicit work list. `Row` writes only direct segments. `NodeKey` drop drains through quire-exact's DropQueue, and the table's own fields are flat Vecs. A 100,000-deep chain interned, formatted and dropped on a 512 KiB thread.
3. Per-node cost. Each node costs one Arc clone, one `path()` Vec, and a String clone per direct text segment. That is linear in direct segments. Hashing reads the cached u64.
4. Completeness. `Origin` is exactly (role, ordinal), so `(#n, role, ordinal)` loses nothing. There is no cap or truncation (QSL-381). Every node's package, segments and ordinal are written.

Side finding: `from_entries` is superlinear. It is not this PR's defect; it belongs to quire-exact. Measured at n=1000/2000/4000 on the test's shared chain: 0.47/2.2/9.1 s. perf puts nearly all of the time in `quire_exact::node::NodeKey::cmp`, under BTreeMap insertion, in `largest_matching_child`, `path_segments_order` and the zip iterators. Two chain keys of different depths are never `Arc::ptr_eq` at aligned positions, so each comparison descends the shallower depth. That gives O(n^2 log n) overall. The same cost is what holds FR-264-AC-1 in `emit::recorded_occurrences`. Raise it with the quire-exact owner. Hash-consing keys at decode in this repo would also resolve FND-002.
