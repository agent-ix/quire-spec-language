---
id: SR-15455
title: "EARS review of STD-261 FR-122 statements (QSL PR #1211)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@f2331483313b4bc808636afafed19f23eb301b77; spec/functional/FR-122-replay-a-state-clause-counterexample.md, spec/test-cases/TC-517-replay-a-state-clause-counterexample.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-122
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-517
    type: reviews
---
# SR-15455: EARS review of STD-261 FR-122 statements (QSL PR #1211)

## Summary

Ticket: STD-261. These are the new or changed statements: the two "If the request's/envelope's replay
target ..., then the executor SHALL refuse ..." bullets (unwanted-behaviour pattern with a named subject),
"The executor SHALL recompile ... before it resolves the request's `state_clause`" (ubiquitous) and "The
executor SHALL resolve `state_clause` ... If it names no state clause ..., then the executor SHALL refuse"
(ubiquitous plus unwanted, in the file's existing style). `make check-spec-validation` reports no EARS
warning on FR-122.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean.
