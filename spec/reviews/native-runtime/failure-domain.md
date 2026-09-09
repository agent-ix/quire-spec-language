---
id: SR-089
title: "Native runtime failure-domain review"
type: SpecReview
analysis: failure-domain
scope: "FR-007/008/018, NFR-006, native-runtime input/evaluation contracts, IT-006, TC-055–077 and TM-004"
review_set: all
evaluated_revision: "045025f843346001a91919b8d0816a519e2df337"
review_date: "2026-09-09"
---

## Summary

Identity, caller-extension failure, immutable execution and cyclic structures have explicit contracts and adverse cases. The initial context and diagnostic ambiguities were corrected before the reviewed revision.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Resolved: requiring post self for every operation would refuse a permitted self-deleting precondition. The contract now requires post self only for a postcondition, while both snapshots remain mandatory for recorded-frame validation. | FR-007-AC-9; TC-061 |
| FND-002 | medium | Resolved: cancellation poll panic behavior was unstated. It now propagates by ordinary Rust unwinding without a report or partial context; returning true is the distinct cancelled outcome. | NFR-006; TC-065; TC-075 |
| FND-003 | medium | Resolved: zero diagnostic capacity now has a separate terminal reason and retains known-invalid classification, so an unrecordable invalid defect cannot become a success or merely unavailable input. | FR-007-AC-5; FR-007-AC-12; TC-064 |

## Trust, identity and topology

The cancellation poll is the only caller executable hook. It cannot mutate
borrowed checked/input data through the API; prompt return is a caller premise.
No callback deadline or allocator-failure recovery is promised. Production
introduces no shared mutable state, worker, plugin or user helper evaluator.

Object identity is the exact model requirement/type/universe/text-key tuple.
Artifact identity is exact labels plus full emitted-byte digest, with distinct
Rust roles and no same-label conflict fallback. Draft indices are local offsets,
not portable or generative identities. Distinct equal-valued objects stay
distinct; identical identities can be compared across observations while reads
retain their capture.

Flat topological value storage bounds structural traversal and destruction.
Native object cycles use identity lookups, never recursive owned objects.
Closure validation indexes before resolving references. Reaches tests each
reached target before visited suppression, handling self-loops and cycles.
TC-068 independently enumerates all functional graphs with one to three nodes.
Deep shared arenas, disconnected nodes, missing targets and already deleted
pre-captured references have separate controls.

Known malformed input survives alongside unavailable data; short circuiting
cannot hide invalid supplied fields. Work limits and events are charged before
operation/storage. Stopped diagnostic traversal reports only observed facts.

## Verdict and provenance

PASS for implementation of this specified LC03 API scope. Agent A applied the
actual installed QUOIN base and all seven analysis skills serially, under the
owner's existing all-review selection. No additional agents or Cargo builds
ran. No applicable required AssuranceProfile was found. The declined optional
semantic gap comparison remains excluded; this specification review still
checks the actual adopted meaning and existing interface boundaries.

All runtime test rows remain planned. Review approval does not qualify native
execution, finish LC02/FS03 acceptance or complete the original backend/Quire
workflow. Implementation changes to this contract reopen specify/review.
