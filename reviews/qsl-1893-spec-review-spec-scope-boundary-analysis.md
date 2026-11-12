---
id: SR-2689005
title: "scope-boundary review of protocol analysis requirements FR-570..FR-575 (quire-spec-language PR 2689)"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quire-spec-language; spec/functional/FR-570..FR-575; spec/test-cases/TC-1410..TC-1415; PR 2689 head 47ca5f0de4f20e2f6c299dad23e60da56554ddf4"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-570
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-571
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-572
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-573
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-574
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-575
    type: references
---

# scope-boundary review of protocol analysis requirements FR-570..FR-575 (quire-spec-language PR 2689)

## Summary

Ticket: QSL-1893. Method: spec-review/spec-scope-boundary-analysis. Diff: `git diff origin/main...HEAD` at 47ca5f0de4f20e2f6c299dad23e60da56554ddf4, 12 files, spec-only.

Boundary with QSpec: no QSpec file is changed. The two QSpec gaps (FR-360 Scope; FR-207 protocol-node step and subject) are raised on QSL-1898 and not assumed. Boundary with quire-protocol: FR-574 produces exactly the FR-351 record FR-302 imports byte-exactly and FR-152-AC-7 requires for decisive realizability. The FRs name quire-protocol behavior only as context. Placement PAs: PA-1 and PA-23 do not appear, and PA-4 appears only as the negotiation ACs. PA-2 leaks once (FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The replay-arm bullet and AC-2 pin a Rust enum variant and its crate (`ReplaySource::ProtocolAnalysis` in `qsl-replay`). The crate is ADR-036 PA-2 placement, which per the slice rule is not a requirement. Require a counterexample replay arm with the stated walk and disagreement rule, and leave the variant and crate to the ADR. | spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md:62-66,87 |

## Scope and evidence

- FR-570 (examined; spec/functional/FR-570-project-a-global-protocol-onto-one-role.md): QSL SHALL decide the `local-projection` capability kind for a checked global protocol and one role, in the `bounded-static` profile (FR-575), and return either the role's local graph with the evidence that it is sound or a minimal global-path counterexample. It implements ADR-036 PA-9 to PA-12 and PA-25 (projection artifacts) and QSpec FR-174-AC-1 to AC-5. The key of the result is FR-572's; its settlement is FR-573's.
- FR-571 (examined; spec/functional/FR-571-decide-whether-local-strategies-realize-a-global-protocol.md): QSL SHALL decide the `realizability` capability kind for a checked global protocol in the `bounded-static` profile (FR-575) as a game in which the environment chooses admitted external events and channel outcomes and each role chooses from its local history only, and return a replayable strategy witness, a counterexample, or a non-conclusive result. It implements ADR-036 PA-7, PA-8, PA-16 to PA-19 and PA-25 (realizability artifacts) and QSpec FR-175-AC-1 to AC-5. The key is FR-572's; settlement is FR-573's.
- FR-572 (examined; spec/functional/FR-572-key-protocol-analysis-results-structurally.md): Every projection and realizability result SHALL carry the structural key of QSpec FR-174-AC-6 or FR-175-AC-6 as typed members, SHALL never stand in a digest for a member, and SHALL be a function of its key. It implements ADR-036 PA-13 to PA-15 and QSpec FR-174-AC-6 and FR-175-AC-6.
- FR-573 (examined; spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md): QSL SHALL settle every projection (FR-570) and realizability (FR-571) result with one QSpec FR-360 label, through the FR-241, FR-242 and FR-243 combination FR-360 names, by the existing native-engine rule and with no rule of its own. It implements ADR-036 PA-12, PA-14 and PA-20 (ADR-011 S6c; ADR-018 SM-7, PC-1).
- FR-574 (examined; spec/functional/FR-574-carry-the-separating-witness-record-on-a-decisive-protocol-counterexample.md): Every `decisive-counterexample` result of FR-570 or FR-571 SHALL carry exactly one QSpec FR-351 separating witness record, with the trace position of the step at which the divergence or violation occurs, and its `value_path` derived from QSpec FR-207 as below. It implements ADR-036 PA-24 (ADR-031 SW-5, SW-7 as amended there). quire-protocol FR-302 imports the record byte-exactly and refuses a decisive realizability result without it.
- FR-575 (examined; spec/functional/FR-575-answer-unsupported-for-constructs-outside-the-bounded-static-profile.md): QSL SHALL decide projection and realizability only in the single profile `bounded-static`, and SHALL answer `unsupported`, naming a located construct cause under the existing `unsupported_projection` code, for every construct outside it, with no substitute graph and no guess. It implements ADR-036 PA-21 and PA-22 and QSpec FR-174-AC-3, and fixes the open item that ADR-036 left to the requirements: the profile word and the cause slugs.
- FR-573-AC-2 (examined; spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md): A projection counterexample and each of the two realizability refutation forms settle `refuted`, `decisive-counterexample`, execution `completed`, truth `violated`, only after the counterexample replays through `ReplaySource::ProtocolAnalysis`.
- FR-573-AC-6 (examined; spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md): A FR-575 construct settles `unsupported`, execution `unsupported`, truth `unavailable`, basis `unavailable`. No `proved` result of either kind settles `Certified` or `Trusted`, and the negotiation result for a kind with no candidate is still `unsupported_projection`/`unsupported-requested-capability` over exactly the three existing `NegotiationCause` variants (FR-057-AC-15).
- FR-575-AC-4 (examined; spec/functional/FR-575-answer-unsupported-for-constructs-outside-the-bounded-static-profile.md): A `NegotiationCause` has exactly its three existing variants and a kind with no candidate still negotiates `unsupported_projection`/`unsupported-requested-capability`; the slugs above arise only from an engine result and add no code family.
- FR-574-AC-2 (examined; spec/functional/FR-574-carry-the-separating-witness-record-on-a-decisive-protocol-counterexample.md): The record has an absent `deciding_quantifier` and an absent `index`, and the wire form omits both; a record the strict reader sees with either present for these families, or with a missing `deciding_element`, `value_path` or `trace_position`, is refused (QSpec FR-351-AC-5).

## Verdict

1 finding(s); FND-001 low.
