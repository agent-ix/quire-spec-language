---
id: SR-102
title: "Native package failure-domain review"
type: SpecReview
analysis: failure-domain
scope: "US-002, FR-019/020/021, NFR-007, IT-007, TC-078–091, TM-005 and native package wire/API/schema"
review_set: all
evaluated_revision: "41da6e5eb86bb727fbad5370fd23b38d330bcfea"
supplement_evaluated_revision: "2c6b9b83dc5c87c68666ebcd24c41b198f4c339b"
review_date: "2026-09-09"
---

## Summary

Checked trust boundaries, identity keys, purity and graph/container termination.
The package has explicit authority inputs and atomic success. Format selection,
cycle termination and all bounded passes are defined before implementation.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | Resolved source-boundary mismatch: serialized empty inventories cannot authorize an empty checked unit that the actual native parser rejects. | TC-078; TC-087; TC-090 |
| FND-001 | low | Draft ordering defect repaired before the evaluated revision: generic bounded recognition and format selection precede version-specific decoding; multi-defect tests establish precedence. | FR-020-AC-2; FR-020-AC-3; TC-084 |
| FND-002 | low | Draft cycle omission repaired: feature discovery visits exact model/declaration identities and scans all syntax without recursively expanding object-reference cycles. | FR-019-AC-6; TC-080 |
| FND-003 | low | Projection-only forgery is excluded from static identity but still rejected by full reconstructed manifest comparison. | FR-021-AC-4; TC-091 |

## Boundary analysis

There is no callback, hook, plugin, mutable cache, asynchronous worker or
network lookup in this API. Failures abort package construction/readback;
nothing publishes partial success. The caller supplies immutable original
FormalSource, exact authored ClauseBindings and admitted NativeModels.
Decoded artifact strings cannot manufacture an admitted model, and a byte
digest is a selector rather than an authority claim.

Keys are explicit: native identity/revision/digest, formal document/revision,
RequirementRef, owner-qualified DeclarationKey, owner/clause pair and local
ExprId/span under exact source. Imports retain aliases/order; valid external
binding order may vary. Conflicting unselected model inventory entries still
reach link_native admission. Display paths cannot replace source identity.

Malformed duplicate keys are rejected before map insertion can erase them.
Numeric recognition is bounded by Serde's documented generic domain; typed
integers are decoded afresh from original bytes. A rounded recognition value
never enters identity. Container limits cover the recognition pass as well
as typed decoding. Token scratch is bounded by input length; decoded string
retention has its own limit. TC-088 explicitly tests the selected recursion
ceiling, delimiters inside strings and coupled earlier stops.

Parser/link/check failure keeps its real Diagnostic, phase and source;
unexposed upstream counters remain absent. Fresh retries cannot inherit a
feature set, budget or successful check from a prior request. Existing runtime
closure, frame and graph evaluation contracts remain qualified separately.

## Admitted source setup correction — 2026-09-09

Re-reviewed specification 2c6b9b83dc5c87c68666ebcd24c41b198f4c339b using this
installed QUOIN lens, superseding the initial review's header-only positive
fixture assumption at 41da6e5. The original PASS and its missed precondition
remain visible at 69588ad. Escape cause: wrong-requirement.

The real parser still runs before successful rebind. Header-only and import-only sources are adverse inputs; no unchecked constructor or alternate grammar bypass is introduced.

Task-016 began under the original completed review gate. Its initial API-red
run is followed by a first implementation run with one passing nonempty case
and three parser setup failures. Further implementation paused for this
specification correction and all-eight re-review; no parser behavior was
relaxed. The initial stdout/stderr is retained in reviews/data/native-packages/.
No claim of executed full package, schema or canonical-vector qualification is
made. PASS to continue against the corrected admitted-source fixture contract.

## Verdict and provenance

PASS for planning and implementing this producer/reader slice. Agent A applied
the installed QUOIN 0.22.5 skills serially under the owner's existing all-review
selection; no required AssuranceProfile applies. This is the author's recorded
review, not independent B/C acceptance. All package cases remain planned.
Changes to the reviewed requirements or interface reopen specify/spec-review.
Interchange, backend qualification and integration remain required. No Cargo build, additional agent or hosted CI dispatch ran.
