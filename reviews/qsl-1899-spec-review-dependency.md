---
id: SR-2690004
title: "dependency review of FR-576 operation-body carrier (PR 2690)"
type: SpecReview
analysis: dependency
scope: "agent-ix/quire-spec-language; spec/functional/FR-576-emit-the-operation-body-into-checked-packages-for-backend-consumers.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-576
    type: reviews
---

## Summary

Ticket: QSL-1899. Checked the eight new `depends_on`/`implements` edges. All
of their targets exist: US-012 and FR-094/102/103/302/494 in this repo, and
FR-151 and FR-322 on quire-specification `main`. Upstream enablement is
untracked, and one sibling edge is missing (see integrity FND-001).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-576 states that QSpec FR-322 "must admit the body record before this requirement is implemented", but that enablement has no ticket. No STD ticket exists for the carrier, and in Linear QSL-1899 has no blocked-by edge (it only blocks IR-1509). The PR body says the carrier choice is an owner decision recorded in a "ticket report", but QSL-1899 has no comments, so that decision cannot be checked. | FR-576 Dependencies; QSpec FR-322 |
