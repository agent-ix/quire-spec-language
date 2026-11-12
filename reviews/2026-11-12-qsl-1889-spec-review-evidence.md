---
id: SR-202611121441565
title: "evidence review of PR2688 ADR-036 witness and worked example"
type: SpecReview
analysis: evidence
scope: "agent-ix/quire-spec-language; spec/decisions/ADR-036-protocol-local-projection-and-realizability-engine.md PA-20, section 8; PR2688 234fdec9bc4e24b543b6e63ec461c6b53a209f42"
review_set: subset
evaluated_revision: "234fdec9bc4e24b543b6e63ec461c6b53a209f42"
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-036
    type: reviews
---

## Summary

Ticket: QSL-1889. Checks whether the evidence the ADR plans can show FR-174/175's ACs, and whether the worked example demonstrates what it claims.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | PA-20 claims FR-175-AC-4 holds "by construction" because the witness table is keyed by class; that is vacuous, since the property depends on the observation function that forms the classes. The evidence must recompute each history's observation sequence and compare actions, and FR-175-AC-1 "replayable" needs a defined replay of the strategy witness, which the ADR lacks | ADR-036:140 |
| FND-002 | medium | Section 8's fix is underspecified: a `decision` message sent with one message type in both branches makes the next visible actions equal, so PA-10 recurses and still diverges on `pick` against end (refuted). It distinguishes only if it precedes the choice and the choice's guards are a function of its payload (FR-053-AC-1 pattern), or if a divergent receive counts as evidence (failure-domain FND-003). State which | ADR-036:167 |

## Verdict

Fix both in the ADR text; neither blocks the placement decision.

## Scope and evidence

- FR-175-AC-1 (quire-specification FR-175): A realizable protocol returns replayable local strategy witnesses under exact premises.
- FR-175-AC-4 (quire-specification FR-175): A witness strategy selects the same action for every pair of locally indistinguishable histories.
- FR-174-AC-2 (quire-specification FR-174): A hidden choice or incompatible local merge yields a source-correspondent counterexample.
- FR-053-AC-1 (quire-specification FR-053): A fulfillment-owned ship/cancel choice is admitted when an earlier message supplies the distinguishing payment outcome.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | PA-19 drops the by-construction claim's weight: AC-4 test recomputes observation(role, history) for every explored history; FR-175-AC-1 replay walks every environment play against the table |
| FND-002 | fixed | Worked example now repairs with a distinct cancelled receive (PA-10) or an earlier guard-tested decision message, and states a same-type decision in both branches does not distinguish |
