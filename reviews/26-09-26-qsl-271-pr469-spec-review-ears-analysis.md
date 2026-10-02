---
id: SR-677
title: "QSL-271 PR 469 EARS review of FR-100's refusal-record mapping"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@ceb5d905; spec/functional/FR-100-run-a-named-function-through-the-spine.md (changed statements at lines 102-106, 111-138, 168, 231-238, 256, 281-288); spec/test-cases/TC-452-spine-run-entry-lives-in-qsl-replay.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: reviews
---

## Summary

Ticket: QSL-271 (PR agent-ix/quire-spec-language#469). This review checks
the changed requirement statements against EARS. `quire validate` reports no
grammar warning for either file.

Clean: the new "If the call's outcome is the kernel `Refusal::CheckedInvariant`,
then `qsl_replay::spine::run` shall return it as an FR-096 `InternalFault`"
(lines 236-238) is well-formed unwanted-behaviour EARS with one actor. The
amended `spine::run` statement (lines 231-235) keeps its single actor.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Schedule and negative wording sits in normative text. The kernel mapping table says "until catalog revision `1-draft.8` (STD-110) gives them codes" and "until the `quire-exact` variant carries the universes". Line 137 says "A refusal a later catalog or kernel revision gives a record renders by the first mapping row with no change to FR-100". Lines 140 and 238 say "FR-096 does not spell..." and "not as a refusal". The normative content is right, and the STD-110 citation is wanted. Failure scenario: when STD-110 lands, the table's "until" cells are stale requirement text, not just a stale Status. Fix: state the rows as the rule at `1-draft.7` ("FR-096 builds no record: `{\"kind\": \"refused\"}`, exit 20"), and move the "until STD-110" and "until the kernel variant" notes to Status, where the same facts already appear (lines 281-288). | spec/functional/FR-100-run-a-named-function-through-the-spine.md:133-138, 140, 238 |

## Verdict

Approve with a nit: no statement changes meaning.

## Dispositions

Disposition pass at `agent-ix/quire-spec-language@bec5791c` (fix commits `e71e60cc` and `bec5791c`, on main 9425dd82). I checked each outcome against the spec at that head and the code on main 9425dd82, not against the commit message. `quire validate` over FR-100, FR-109, TC-452, TC-468 and these reviews exits 0.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed e71e60cc | The kernel table's "until" cells are gone. The normative text states the rule at `1-draft.7`, and the STD-110, QSL-281 and "later record" notes are in Status. |
