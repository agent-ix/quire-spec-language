---
id: SR-1711002
title: "gap analysis of PR #1711 (QSL-1181 slice 1) against FR-375-AC-7/20/25 and FR-322-AC-134"
type: SpecReview
analysis: gap-analysis
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1711 slice (git diff 2c4aeda3c 9a6f966b1); QSpec FR-375-AC-7, AC-20, AC-25; FR-322-AC-134; TC-946 OM-07"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-375
    type: reviews
---
# SR-1711002: gap analysis of PR #1711 (QSL-1181 slice 1)

## Summary

Ticket: QSL-1181. Head reviewed: 9a6f966b1, slice only (see SR-1711001 FND-001). QSpec origin/main 3e4ccfd5. Plan completion: not assessed.

| Requirement | Measured |
| --- | --- |
| FR-375-AC-7 | Unchanged by the slice. The concrete `semantic_type` and the `dependencies` of `[abstract_root]` are asserted in `a_row_empty_refinement_identity_excludes_its_source_region`. |
| FR-375-AC-20 | **Partial** (FND-001). The emitted package reads back through IR's reader, which recomputes the key from the `quire.refinement-node/v1` preimage. Not tested: the refusal half (change `semantic_type` or a body row, keep the node id, refuse `stale-node-key`). |
| FR-375-AC-25 | Unchanged. The `abstract` key names the derived model root (asserted). |
| FR-322-AC-134 | Unchanged. The existing test at emit/tests.rs:2583 is untouched by the slice. |
| Nine-member closed body | Asserted: exactly the nine members, the six row arrays empty, and no optional member. That is FR-375's body statement ("all arrays are present, including empty arrays"), not AC-20. |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | **The new `#[trace("FR-375-AC-20")]` claims more than the test checks.** The test checks the closed body's members and the read-back. AC-20 also requires that a refinement node with a changed `semantic_type` or body row and its old node id is refused `invalid_package`/`stale-node-key` at that node, and no QSL test does that. Either add the refusal (the read-back is already in the test, so it is a few lines) or drop the tag. | qsl-package/src/emit/tests/admission_corpus.rs:1016, 1075-1107 |

## Verdict

**Not mergeable**, while SR-1711001 FND-001 is open. FND-001 here is medium; fix it in the same rebuild.

## Dispositions

Round 1 (head 3cdd54dcc).

Round 1. Head 3cdd54dcc.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `a_refinement_changed_under_its_node_id_is_a_stale_node_key` passes. The emitted package is admitted. Either change, under a kept node id with the package frame recomputed, is refused `InvalidPackage` at `/semantic_graph/nodes/{i}/node_id` (`StaleNodeKey`): `semantic_type` swapped to the abstract root (`package_id` asserted to change), or the body's `profile` changed. |

**Is `profile` a fair stand-in for a body row? Yes.**
- IR's reader at 796609b hashes `body` whole. `RefinementNodePreimage` writes `{version, semantic_type, body}` with the body as a single `Value` (encode.rs:173-186), so a changed row and a changed `profile` reach the key the same way.
- A row-empty model has no population or object over which a valid row could be written, so the coder's choice is the only admissible body change.
- The one gap: a reader that hashed every member except the row arrays would still pass this test. That needs a fixture with rows, which slices 2 and 3 add. Re-check this clause there with a real row. Not a finding.

**Verdict: mergeable once IR #627 merges** and the manifests are repinned. No finding remains open on SR-1711001 or SR-1711002.

Gates at 3cdd54dcc (locked-build, jobs 4, `--all-features`, debug info off, QSPEC_DIR=~/dev/quire-specification at 308596b0):
- fmt: exit 0.
- workspace clippy `-D warnings`: exit 0.
- qsl-semantics `refinement`: 88/88.
- qsl-package: 349 pass, 2 fail (c14, i2). Both fail identically on main 918e624f7: `positive-model-tuple-union.json` (QSpec #468, STD-519) is refused `MalformedWire` at `/semantic_graph/nodes/0/owner` by both IR pins. #470 fixed the earlier fixture; this is a new IR/QSpec disagreement, not caused by this PR.

