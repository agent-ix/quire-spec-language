---
id: SR-9022
title: "Integrity review of QSL-902 observed temporal replay"
type: SpecReview
analysis: integrity
scope: "FR-071, FR-072, FR-098, FR-331, TC-847; QSpec FR-323 context"
review_set: all
---

## Summary

FR-098 now dispatches the observed arm before the function-only path. FR-331 and TC-847 cover identity mismatch and preserve the distinct packet and evaluator positions. One contract-boundary ambiguity remains: the new arm must not silently change QSpec FR-323's immutable serialized request.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | QSpec FR-323 says the native runtime producer SHALL use its immutable envelope, whose `replay` property carries exactly one of `function`, `frame_operation` or `state_clause`; it has no observed `TemporalCounterexample` member. QSL FR-071 adds the payload to `ReplayRequestWire` and AC-8 asks to round-trip it without defining whether that round trip is an in-process typed conversion or external JSON serialization. QSL's current `ReplayRequestWire` is explicitly an in-process struct with no serde serialization, so this does not inherently block a QSL-local field. Clarify that the observed arm is local call context populated from the typed QSpec FR-331 counterexample and is excluded from FR-323 serialization; otherwise the FR-323 schema and vectors must be amended by QSpec's owner before conformance can be claimed. | FR-071; FR-098; FR-331; QSpec FR-323 | missing-requirement |

## Round 2 Disposition

| Prior Finding | Disposition | Evidence |
| --- | --- | --- |
| FND-001 | Resolved | QSpec FR-323 now specifies the external observed request and closed result encodings, including `ReplaySource: Witness`, the envelope/payload position equality and exact refusal, and the closed typed temporal value union. QSL FR-071 and FR-072 explicitly target those external forms, preserve the typed observed values and category mappings, and reject `ReplaySource::Input` and position mismatch as specified. TC-847 covers those request refusals and typed value outcomes. The earlier external-boundary ambiguity no longer blocks conformance. |

No remaining integrity gap from the prior review was found in this round.

## Final Fix-Round Disposition

The latest AC and test updates close the remaining observable-case coverage:
TC-847 now checks the distinct request refusal codes and locations before
document admission, preserves `InsufficientData` and `Incomplete` typed
outcomes as `ReplayParity`, and checks same-position undefined results with a
changed locus or cause. The external request and result forms remain aligned
with QSpec FR-323 and TC-278. The prior external-boundary finding remains
resolved; no remaining integrity blocker was found.
