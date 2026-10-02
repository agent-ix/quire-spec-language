---
id: SR-964
title: "QSL-366 EARS conformance review of FR-123 to FR-128"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@9b546eb069048b46d0c365f85c4481c0a3012115; spec/functional/FR-123..FR-128"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-124
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: reviews
---

## Summary

Ticket: QSL-366. `quire validate` reports no `[ears:*]` or `[quality:*]`
warning in FR-123 to FR-128. Every Description and Behavior SHALL was read.
Most have a named subject and a concrete response, and FR-128's refusals use
the `If ... then the executor SHALL` unwanted-behaviour form. The two defects
are wording only.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | "The map SHALL be exhaustive, with no `_` arm" states a Rust coding style, not observable behaviour. "give each input exactly one row" already says what can be tested, and FR-127-AC-1 tests it. Drop "with no `_` arm". | spec/functional/FR-127-settle-a-model-check-verdict-as-a-terminal-record.md:74-75 |
| FND-002 | low | "`P` SHALL be read only at terminal states" is agentless passive. Name the subject: "The deadlock classifier SHALL evaluate `P` only at terminal states." | spec/functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md:78 |

## Verdict

EARS-conformant apart from two wording nits.

## Dispositions

Round 1, reviewed at ed7bcc8b (re-checked at the #562 head; first drafted at f7180085). This file's findings were posted on QSL-366 under the id SR-952.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 906c3b0b |
| FND-002 | fixed | 906c3b0b |
