---
id: SR-202611121441561
title: "integrity review of PR2688 ADR-036 protocol analysis engine"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; spec/decisions/ADR-036-protocol-local-projection-and-realizability-engine.md; PR2688 234fdec9bc4e24b543b6e63ec461c6b53a209f42"
review_set: subset
evaluated_revision: "234fdec9bc4e24b543b6e63ec461c6b53a209f42"
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-036
    type: reviews
---

## Summary

Ticket: QSL-1889. Integrity of ADR-036 against QSpec FR-174/FR-175 (as merged by QSpec #650), FR-054, FR-360, QSL ADR-011 S6c, ADR-018 V-7/PC-1/SM-7 and quire-protocol FR-152 (the consumer TL-387 implements). The key members, absent-member refusal, no-digest rule and projection-identity-excludes-analyzer rule (PA-14, PA-16) match FR-174-AC-6 and FR-175-AC-6 exactly. The witness shape does not give quire-protocol the trace-position witness its FR-152 requires, and the bound and settlement vocabulary diverges from ADR-018 and ADR-011.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | PA-25 gives no FR-302/FR-351 trace-position witness for a decisive realizability result, and claims CheckedNodeId references are quire-protocol's trace-position correspondence; quire-protocol FR-152 says a trace position is no graph identity and refuses a decisive realizability disposition without one (FR-152-AC-7), so TL-387 would not unblock | ADR-036:155 |
| FND-002 | medium | PA-15 makes `max_states` a key-member bound whose exhaustion is `inconclusive`; ADR-018 V-7 defines `max_states` as a run limit settling `failed`/`resource-incomplete` and only `max_depth` as the horizon. Same name, two meanings | ADR-036:130 |
| FND-003 | medium | PA-21 omits the ADR-018 PC-1 certification label every `proved` carries, and replaces ADR-011 S6c/ADR-018 SM-7 (refutation settles after E9 replay; a proof without a core checker settles `proved` `uncertified`) with an invented "settle only after the PA-2 checker" rule; no replay arm is defined for either counterexample | ADR-036:146 |
| FND-004 | low | Wrong citations: PA-7 cites FR-053-AC-1 (owner admission) for non-owner merge visibility; PA-24 calls the unserved-kind tests the "TC-1302 family" but TC-1302 is capability-to-family application and the FR-057-AC-15 test is TC-155; PA-22 attributes `join any`/`quorum` to G3 and `repeat` to G5, yet qsl-eval steps all three and G5 (invariant/variant discharge) is irrelevant to a bounded search; section numbering jumps from 8 to 12 | ADR-036:112 |
| FND-005 | low | PA-21 settles a projection `inconclusive` on an absent group (c)/(d) premise, but FR-174's inputs and key carry only channel, choice and visibility premises; (c)/(d) are realizability-only | ADR-036:146 |

## Verdict

Not mergeable as is: FND-001 must be fixed (the ADR exists to unblock TL-387). FND-002 and FND-003 are small text fixes. Clean units: PA-5, PA-14, PA-16 match FR-174/175 "Derived-artifact identity" and AC-6; PA-4 correctly adds no NegotiationCause variant; FR-360 Scope amendment correctly raised as an open item rather than assumed.

## Scope and evidence

Read QSpec origin/main at b690867b (includes #650 eb2932bd), quire-protocol origin/main FR-152 and ADR-010, QSL ADR-011, ADR-018, ADR-028. `quire validate` on the ADR is clean.

- FR-174-AC-6 (quire-specification spec/functional/protocol/FR-174-project-global-protocols.md): Changing the FR-288 definition key, the selected role, the projection profile, the channel, choice or visibility premises or the bounds yields a different derived-artifact identity, while changing only display name, path, span or producing run does not; the artifact carries its key members; an absent key member is refused invalid_runtime_input/missing-member naming the member; and no digest of the artifact stands in for a member.
- FR-175-AC-6 (quire-specification spec/functional/protocol/FR-175-analyze-protocol-realizability.md): Changing the FR-288 definition key, the strategy profile, the visibility, channel or environment premises, the bounds or the producing analyzer identity or version yields a different witness identity; two runs of one analyzer identity and version on one key return the same witness; an absent key member is refused invalid_runtime_input/missing-member naming the member.
- FR-152-AC-7 (quire-protocol spec/functional/FR-152-carry-projection-and-realizability-witness.md): Given a stand-in returning a decisive realizability result with no witness, and a second whose witness record has no trace position, each disposition is refused under FR-030, and a decisive local-projection result with no witness is carried unchanged.
- ADR-018 V-7 (spec/decisions/ADR-018): A run limit stopped the analysis before it completed its method: max_states, max_transitions, ... The max_depth horizon is not a limit.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | PA-24's record does not fit QSL's existing FR-351 type: `qsl-replay` `SeparatingWitnessRecord` (ADR-031 SW-5, SW-7) has a required `quantifier: OccurrenceKey` and a `deciding_element: Value` (kernel value), and its wire refuses a missing index, while PA-24 makes the quantifier and index absent and the deciding element a node-id string. Name the ADR-031 amendment (optional quantifier, protocol deciding element) and its slice, and the FR-352 `native-run-result/2` carriage quire-protocol FR-302 imports from | ADR-036:155 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | PA-24 adds one FR-351 record per decisive-counterexample (deciding_element, value_path, zero-based trace_position string), states node ids are not the trace position, and closed-scope carries none, matching quire-protocol FR-302-AC-2 and FR-152-AC-7; residual type fit tracked as FND-006 |
| FND-002 | fixed | PA-14: max_depth and max_history are key members (inconclusive); max_states, max_transitions and the meter are V-7 run limits (failed/resource-incomplete) outside the key |
| FND-003 | fixed | PA-12 and PA-20: proved is Uncertified per ADR-011 S6c and ADR-018 PC-1; refutation settles after the ReplaySource::ProtocolAnalysis replay arm (SM-7); no checker rule |
| FND-004 | fixed | TC-155 cited (PA-23); PA-21 gives the real reason for excluding join any, quorum and repeat; PA-7's FR-053-AC-1 citation now matches its earlier-message pattern; section numbering removed |
| FND-005 | fixed | PA-6: projection takes groups (a) and (b) only; the absent (c)/(d) inconclusive rule is realizability-only (PA-20) |
