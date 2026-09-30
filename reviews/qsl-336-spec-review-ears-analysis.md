---
id: SR-825
title: "QSL-336 EARS review of PR 539 (FR-122)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@15d200d648a0fc06ec3d5a6c4270debfb40ee913; spec/functional/FR-122-replay-a-state-clause-counterexample.md; spec/test-cases/TC-517-replay-a-state-clause-counterexample.md; spec/spec.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-122
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-517
    type: reviews
---
## Summary

Ticket: QSL-336. PR: quire-spec-language#539 at 15d200d6.

Every FR-122 Behavior bullet is a SHALL statement, unconditional
(ubiquitous) or `If ... then` (unwanted behaviour), with one actor (the
executor). The nested settlement bullets are each one trigger and one
response. The descriptive sentences at lines 104-106 and 111-112 restate
FR-106 and FR-072 by reference and add no second obligation.

## Verdict

Conforms, with one compound condition whose response is under-specified.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Compound trigger: "If the node identity differs ..., or its `claim` occurrence key differs ..., then ... refuse ... naming the envelope's and the recompiled identity". When both differ (the usual case, since an `OccurrenceKey` carries its node id) it does not say which is checked first or which pair is named. PR #538's FR-116 check orders the node first. AC-3 changes one at a time, so either order passes. State the order: node, then occurrence. | spec/functional/FR-122-replay-a-state-clause-counterexample.md:96-101 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | cf00f646 |
