---
id: SR-15511
title: "EARS review of STD-264 requirement statements (QSL PR #1221)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@4b52347fe6a7d07471189e0d4745c08ec078148b; spec/functional/FR-098-execute-a-replay-request.md, spec/functional/FR-071-implement-typed-replay-request.md, spec/functional/FR-290-settle-plugin-results-as-typed-terminal-records.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: reviews
---
# SR-15511: EARS review of STD-264 requirement statements (QSL PR #1221)

## Summary

Ticket: STD-264. This review checks the new and edited SHALL statements. FR-071 adds two: the
ubiquitous "The request type SHALL carry ... `occurrence_key`" and the unwanted-behaviour "If a
`function` request carries no `occurrence_key`, then the reader SHALL refuse it". FR-098 adds the
selection statement, steps 1 and 3, and the application-claim call statement. FR-290 edits the
event-driven statement "When a process provider reports a counterexample for a function-contract item
or an application item". The grammar gate reports no warning on these lines. FR-071 line 97 has a
warning, but it is outside the diff.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | minor: the application-claim statement has two obligations in one sentence: "The executor SHALL call the resolved function through S6a as for a predicate claim ... and SHALL observe each evaluation of the record's application at the record's occurrence in the call's own body." Split it into one call requirement and one observation requirement. The observation rule is the one FR-098-AC-22 tests, and SR-15512 FND-001 asks for it to be made more precise. | spec/functional/FR-098-execute-a-replay-request.md:470-473 |

## Verdict

No major findings. The new statements follow EARS forms, and each one names its subject (the request
type, the reader or the executor).

## Dispositions

Round 1, reviewed QSL #1221 at 92094c48b.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
