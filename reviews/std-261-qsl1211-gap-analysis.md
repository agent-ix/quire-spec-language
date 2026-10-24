---
id: SR-15457
title: "gap analysis of STD-261 FR-122 ACs and TC-517 (QSL PR #1211)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@f2331483313b4bc808636afafed19f23eb301b77; spec/functional/FR-122-replay-a-state-clause-counterexample.md, spec/test-cases/TC-517-replay-a-state-clause-counterexample.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-122
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-517
    type: reviews
---
# SR-15457: gap analysis of STD-261 FR-122 ACs and TC-517 (QSL PR #1211)

## Summary

Ticket: STD-261. This is a spec-only PR. Plan completion: not assessed. AC-to-TC trace: FR-122-AC-8
is traced by TC-517 step 8, and TC-517's scope is updated to AC-8. AC-1, AC-3 and AC-6 have matching TC-517
step 1, 3 and 6 edits. The code does not yet implement this (`replay_state_clause` still reads the payload
`clause`), which the PR body names as the code follow-up. `make check-spec-validation` with QSPEC_DIR at
#364's head: exit 0. The missing FR-070/FR-071 `state_clause` coverage is recorded in SR-15454 FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean apart from SR-15454 FND-001.
