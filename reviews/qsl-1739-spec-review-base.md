---
id: SR-20261105101015
title: "Base specification review — QSL-1739 FR-090 observation contract"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md; FR-255, FR-397, FR-520 context"
review_set: subset
---

## Summary

Reviewed the edited FR-090 description and AC-23 against FR-255, FR-397, and FR-520. The acceptance criterion leaves the operation-ledger record granularity unclear because the cited requirements describe different accounting views.

## Verdict

**CONDITIONAL** — AC-23 needs an unambiguous source for its ordered operation-record comparison before implementation can be tested consistently.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC-23's cited granularity does not define one ordered operation-record contract | spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md:642 |

FR-397 specifies charge points and their order for IEEE evaluator nodes, while FR-520 specifies a per-binder charge log and how its totals combine. Neither defines the ordered record shape/granularity for every checked function-body operation. Consequently, two implementations could expose different ledger entries yet both claim to follow the cited requirements, leaving the operation-by-operation oracle underspecified.
