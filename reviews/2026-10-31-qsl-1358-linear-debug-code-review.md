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


## Disposition pass 1 scope and evidence

Replacement independent reviewer at frozen `866f26cdb74c6ae1bdbd09cb882456ceb50fe771`; scoped to `87788cfc2b665ec115b34f9735b986261ee4ac46...HEAD`, excluding inherited parent implementation. All original findings and verdict text above are preserved as historical records. Methods: code-review with rust-review/rust-style baseline references and scoped gap-analysis. No applicable AssuranceProfile or spec changes; spec-review does not apply.

Examined all 34 changed source/test files: foundation source/node_table and provenance; package checked, checked_v2, emit, emit/tests and source_map_debug; semantics check abstraction, assemble/equivalence, check, claims, family, hyper_clause, identity, ir, lowering, mod, node_key, protocol_clause/types, refinement/catalog/rows, state_clause, task_set, temporal_clause/timed, time_units, timed_model, type_invariant, wire_id; library/mod; value member/nominal_payload. FR-264-AC-1 examined: 100,000-term emit/read/identity verification/clone/equality/Debug/drop on 512 KiB. NFR-015-AC-3 examined: foundation tests pass with and without default features. NFR-015-AC-4 examined: package host check succeeds without default features. NFR-015 feature contract is context. QSL-1358's complete uncapped Debug is examined.

In the std production path, one outer scoped table preserves every direct segment, package, ordinal, role and source region. Hashes only select buckets; full equality decides identity. Work lists avoid depth recursion. Provenance sorts cached hashes with full structural ties; other retained ordering stays deterministic. Expanded retained records use named destructuring. No caps, wire schema changes, duplicated upstream implementation, new framework or weakened CI gate occur in this diff.

`StructuralIds::key_reader` seeds the inverse memo from authoritative emitter Arc allocations, retaining Arcs against address reuse; it is not indexed by positions in a filtered emitted list. Omitted or foreign records cannot shift memo slots. A foreign allocation follows ordinary conversion. This memo is used for internal inspection only; untrusted I2 still admits the package and uses WireGraph's existing range/cycle validation. No hash-only identity is introduced.

Evidence read directly from `/home/peter/dev/.agents/tmp/pr1947-checks`: foundation 17/17, semantics library 20/20, inverse sharing 1/1, source-backed Debug 3/3, normal package 450 passed with 2 existing ignored and 10 held filters, scoped Clippy pass. The unchanged full FR-264-AC-1 test passed 1/1 in 142.96 s (elapsed 2:23.52, maximum RSS 4663316 KiB, exit 0), with its existing 512 KiB thread. Reviewer fmt and diff whitespace checks pass. No aggregate gate or duplicate heavy probe was launched.

At depth 100/200/400, I2 Debug writes 357465/702965/1395085 bytes and post-emission package Debug writes 1042011/2048011/4063737 bytes. Inspected locked IR 21fd5e9 Debug: flat wire/node_index/source_index and structural_key_count; expanded structural_keys are omitted. The old upstream blocker does not apply to this indexed path. No IR relock is needed.

The no_std formatter deliberately uses full NodeKey Debug without a shared table, as node_table module documentation states. It retains complete identities and regions but repeats nested keys; this pass establishes linear Debug for the std checked-package/I2 acceptance path only. It does not establish a no_std linear-growth guarantee. The supported no_std compilation and fallback tests must still pass, and the fixes should preserve complete fallback output rather than hide failures or cap depth.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | high | The new unconditional CheckedPackage Debug implementation uses std::fmt in a no_std crate, breaking the supported no-default-features host check (NFR-015-AC-4). Use core::fmt for the outer and inner implementations. Coder's focused check on frozen head exits 101 with six E0433 errors. | qsl-package/src/checked.rs:323, qsl-package/src/lib.rs:2 |
| FND-004 | medium | New foundation provenance tests require the std-only scoped table but run with no default features. That configuration deliberately formats full keys, so exact WithNodes, shared-leaf-count and linear-size assertions fail in the required NFR-015-AC-3 lane. Configure table-specific representation tests for std and retain meaningful no_std completeness checks; do not suppress the required no_std lane. | qsl-foundation/src/source/provenance.rs:607, qsl-foundation/src/source/node_table.rs:67 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | CheckedPackage, V2Read, CheckedGraph and retained key-bearing records join one scoped table. Locked IR Debug is flat for indexed packages; unchanged full FR-264-AC-1 passes. |
| FND-002 | fixed | I2 source-map keys share authoritative WireGraph operands; internal retained-emission inspection also seeds StructuralKeys from original emitter allocations. Full equality remains the collision fallback. Source-backed growth/sharing and full acceptance pass. Arbitrary independently constructed NodeKeys retain deep-equality cost, but production read no longer has the defect described. |
| FND-003 | still-open | Confirmed by package-no-default-features.log: focused cargo check --locked -j1 -p qsl-package --no-default-features exits 101 with six E0433 errors at the new std::fmt references. |
| FND-004 | still-open | The unconditional tests assume a representation the explicit no_std scoped branch does not produce; Coder probe foundation-source-no-default-features.log confirms 14 passed / 3 failed, exit 101: unique-leaf count 6 versus 1, PackageSourceMap versus WithNodes golden, and depth 500 to 1000 growth 2894057 to 11533308 bytes. |

## Current verdict after disposition pass 1

**FAIL / changes requested; not mergeable.** Both original findings are fixed and the unchanged 100k acceptance case passes. New no_std production/test regressions require correction and focused feature checks. A fmt qualifier/test-configuration-only fix does not justify repeating the 100k case.
