---
id: SR-17183
title: "QSL-1072 PR #1523 dependency review of FR-487 and the FR-315 edge"
type: SpecReview
analysis: dependency
scope: "agent-ix/quire-spec-language; spec/functional/FR-487-lower-hyper-atoms-and-match-conjuncts-to-checked-nodes.md (relationships, Dependencies), spec/functional/FR-315-encode-the-smt-lib-transition-relation.md (relationships, Dependencies)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-487
    type: reviews
---

## Summary

Ticket: QSL-1072. PR: quire-spec-language#1523 (head <sha removed>). This lens checks the new edges: FR-315 depends_on FR-487, and FR-487 implements US-021, depends_on ADR-023/FR-104/FR-107/FR-172, references FR-337, and traces_to FR-175/FR-176/FR-315. No depends_on cycle forms. FR-487 traces_to FR-315 and does not depend on it, and none of FR-104, FR-107 or FR-172 depends on FR-315. Enablement is separated from feature work: QSL-1072 (lowering and seams) comes before QSL-1010 (`encode_hyper_query`, which consumes it) and QSL-1071 (emission into the `hyperproperty` node). The merged #1521 (QSL-867) is a prerequisite and is already on main.

Examined units: FR-487 relationships and Dependencies, and FR-315 relationships and Dependencies (examined). US-021 was read for context only.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-487's Dependencies section lists FR-120 (transition identities and argument encoding), FR-128 (argument decoding), FR-177, FR-178 and FR-183 (seam readers), and the Out of scope section names FR-093. None has a `relationships:` entry, so `quire` link analysis and blast-radius queries miss them. FR-120 and FR-128 are real inputs to `evaluate_step_predicate`. Add `depends_on` for FR-120 and FR-128, and `traces_to` for FR-177, FR-178 and FR-183. | FR-487:5-23, FR-487:340-355 |

## Verdict

Sound ordering and no cycles. One low finding on missing edges.

## Dispositions

Round 1, head <sha removed> (fix commit <sha removed>).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-487 relationships now include depends_on FR-120 and FR-128, references FR-093, and traces_to FR-177, FR-178 and FR-183. FR-172 cites FR-487 in prose only, with no relationships edge, so no depends_on cycle forms. |
