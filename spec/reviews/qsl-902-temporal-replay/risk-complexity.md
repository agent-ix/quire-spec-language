---
id: SR-17048
title: "Risk and complexity review of QSL-902 temporal replay"
type: SpecReview
analysis: risk-complexity
scope: "FR-071, FR-072, FR-098, FR-331, TC-847; QSpec FR-323/FR-364"
review_set: all
---

## Summary

The shared model/observed counterexample sum and observed-temporal result arm are high technical risk because they cross request decoding, document admission, temporal evaluation, parity settlement and result encoding. The QSpec FR-364 payload is defined, but its relationship to FR-323's separate `counterexamples` and `replay` members needs an explicit adapter contract before external compatibility is claimed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-071, FR-072, FR-098 and FR-331 are high technical risk. The typed observed arm must coexist with FR-128's model payload; replay must preserve request identity, admit and evaluate documents, then settle a distinct result. The external-boundary volatility is high until QSL specifies how this typed request/result maps to QSpec FR-323's separate `replay`, `results` and `counterexamples` members. Mitigate by clarifying that mapping before claiming wire compatibility, then implement one vertical slice with both-variant and TC-847 tests. | FR-071, FR-072, FR-098, FR-331, TC-847, QSpec FR-323/FR-364 |

## Risk Register

| Requirement | Technical Risk | Volatility | Drivers | Mitigation |
| --- | --- | --- | --- | --- |
| FR-071 | High | High | FR-364 counterexamples are a separate QSpec envelope member from FR-323's `replay`; QSL's observed arm must say how those members become one typed request | Define the adapter mapping and identity join; request a QSpec amendment only if the mapping requires a new wire member |
| FR-072 | High | High | The observed temporal result must map to FR-323 `results` and/or `counterexamples`; that mapping is not stated | Define the result-envelope mapping before claiming serializer compatibility; test outcome/category and exact position/locus/cause |
| FR-098 | High | High | `qsl_replay::replay` dispatches a typed observed temporal arm built from separate FR-323 envelope members | Specify the input adapter mapping before dispatch; retain function/frame/state-clause refusal precedence |
| FR-331 | High | Medium | Recompile, identity resolution, document admission, temporal evaluation, profile/fairness/interval checks and parity ordering | Implement in FR-331 order; verify refusal before admission/evaluation and parity paths in scoped tests |

## Top Hazards

1. QSpec FR-323's `replay`, `results` and `counterexamples` members are distinct; without an explicit QSL mapping, the new typed request/result may be mistaken for a change to FR-323's wire contract.
2. The shared `TemporalCounterexample` representation could break FR-128 model replay; retain and test both typed variants.
3. FR-331 requires fairness refusal before formula evaluation; test the absence of evaluation as well as the refusal code and constraint.

## Failure-domain Gaps

See `failure-domain.md` for the companion analysis. The most relevant cross-check is whether the adapter rejects mixed model/document payloads and binds request/envelope identities before reading documents.

## Round 2 Disposition

| FND | Outcome | Evidence / remaining issue |
| --- | --- | --- |
| FND-001 | fixed | The QSL request now maps to QSpec FR-323's `state_clause`, `ReplaySource: Witness`, `observed_counterexample` and common `WitnessEnvelope` identity; FR-072 maps its result to FR-323's closed `observed_temporal` object and value union. FR-071 defines the envelope/payload position equality refusal before document admission. TC-847 now exercises false, true, undefined, pending, insufficient-data, incomplete and the fairness refusal. |
| FND-002 | medium | FR-331 explicitly gives reproduced behavior for false and matching undefined, and its general fallback names true, pending and wrong-position false. It does not enumerate `InsufficientData`, `Incomplete`, or same-position undefined with a mismatching locus/cause in the settlement rule; TC-847 checks the first two as `ReplayParity` but has only the wrong-position undefined control. Make the fallback explicit and add same-position locus/cause mismatch vectors so these parity edges are unambiguous. | FR-331, FR-072, TC-847 |
| FND-003 | medium | FR-071 requires refusing `ReplaySource::Input` but does not define its refusal code or location. TC-847 currently expects that refusal at `/replay/observed_counterexample/trace_position`, the location specified for a packet/envelope position mismatch. Define the Input-arm refusal separately and assert it at the source-arm location; keep the trace-position vector specific to position disagreement. | FR-071, TC-847 |

## Final Disposition — Second Fix Round

| FND | Outcome | Evidence / remaining issue |
| --- | --- | --- |
| FND-001 | fixed | Exact QSpec FR-323/FR-364 request and result mappings are now stated in FR-071/072, including `ReplaySource: Witness`, clause/witness identity, the closed observed result union and position equality. |
| FND-002 | fixed | FR-331 now explicitly settles `InsufficientData`, `Incomplete`, and undefined results with same-position locus or cause mismatch as `ReplayParity`; TC-847 covers these outcomes and both mismatch controls. |
| FND-003 | fixed | FR-071 defines distinct refusal codes and locations for Input source, model payload, missing document, non-state target and packet/envelope position mismatch. TC-847 asserts the Input error at `/replay/source` and the position error at `/replay/observed_counterexample/trace_position`. No remaining risk-review blocker found. |
