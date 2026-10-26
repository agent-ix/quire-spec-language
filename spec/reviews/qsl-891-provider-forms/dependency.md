---
id: SR-17211
title: "Dependency review of QSL state-graph provider form negotiation"
type: SpecReview
analysis: dependency
scope: "QSL FR-075-AC-21/22, FR-167-AC-4, ADR-014 §6, ADR-022 QS-7, TC-193, TC-592"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-167
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-193
    type: references
---

# Dependency review of QSL state-graph provider form negotiation

## Summary

Checked ownership and prerequisite direction for the state-graph form
vocabulary, provider wire field, descriptor admission and candidate filter.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No issues found. QSpec FR-390 defines the claim forms and FR-331 owns the manifest wire member; QSL FR-075 consumes both and ADR-022 assigns the negotiation behavior. | FR-075, FR-167, ADR-022 QS-7, QSpec FR-331/390 |

## Verdict

PASS. The QSL changes consume the upstream form contract without changing its
wire ownership.
