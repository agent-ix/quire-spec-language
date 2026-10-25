---
id: SR-9021
title: "Failure-domain review of QSL-902 observed temporal replay"
type: SpecReview
analysis: failure-domain
scope: "FR-071, FR-072, FR-098, FR-331, TC-847; QSpec FR-323 context"
review_set: all
---

## Summary

The revised requirements now route the observed arm through the facade, bind request and envelope clause identities, and distinguish packet position from the evaluator's actual undefined position. The remaining boundary risk is whether the new payload is an in-process QSL call context or an addition to the immutable QSpec request wire.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-071 adds a `WitnessEnvelope<TemporalCounterexample>` arm to `ReplayRequestWire` and requires a round trip, but does not say whether that is QSL's in-process request context or a new member of QSpec FR-323's serialized `quire.native-runtime/v1` request. FR-323 defines an immutable `replay` property with exactly three target members and no counterexample payload. State that the observed arm is QSL-internal and how it is populated from the typed FR-331 provider counterexample, without extending FR-323's JSON; otherwise a QSpec-owned contract update is required. | FR-071; FR-098; QSpec FR-323; QSpec FR-331 | missing-requirement |

## Round 2 Disposition

| Prior Finding | Disposition | Evidence |
| --- | --- | --- |
| FND-001 | Resolved | QSpec FR-323 now defines the external observed request member `replay.observed_counterexample`, its closed FR-364 payload, required `ReplaySource: Witness`, common-envelope identity, and byte-provision mapping. QSL FR-071 models the corresponding typed envelope/payload arm, requires `Witness`, and states the exact envelope/payload position mismatch refusal before document admission. TC-847 exercises the `ReplaySource::Input` and position-mismatch refusals. The previous uncertainty over whether the change had an owning external wire contract is resolved by the amended QSpec contract. |

No remaining failure-domain gap from the prior review was found in this round.

## Final Fix-Round Disposition

The latest FR-071 revision distinguishes the observed-arm refusal cases by
code and location: `ReplaySource::Input` at `/replay/source`, model payload at
`/replay/observed_counterexample/trace`, missing document digest by its digest
record, non-state target at `/replay/observed_counterexample`, and envelope
position mismatch at `/replay/observed_counterexample/trace_position`. These
all occur before document admission. FR-331 retains typed `InsufficientData`
and `Incomplete` results and treats same-position undefined locus or cause
mismatch as `ReplayParity`; TC-847 asserts each case. The previously resolved
external wire/source finding remains resolved, and no additional
failure-domain gap was found.
