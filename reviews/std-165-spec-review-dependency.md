---
id: SR-12887
title: "dependency review of STD-165 state clause names a population (PR #982)"
type: SpecReview
analysis: dependency
scope: "agent-ix/quire-spec-language; spec/functional/FR-391-name-a-population-in-a-state-clause.md, spec/functional/FR-155-derive-and-enforce-read-and-write-footprints.md, spec/functional/FR-104-check-state-clauses.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-391
    type: reviews
---
# SR-12887: dependency review of STD-165 (PR #982)

## Summary

Ticket: STD-165. Checked the new relationship edges: FR-391 depends_on
FR-084/091/093/094/104/380 and QSpec FR-153, FR-155 depends_on FR-391, and
FR-104 references FR-391. There is no cycle: FR-104 uses `references`, not
`depends_on`. FR-155 to FR-391 is correct enablement, since AC-2's
quantifier clause cannot be tested before FR-391 lands, which is the STD-165
premise. One edge is missing.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-391-AC-5 asserts S6a evaluation over a snapshot population (complete or incomplete membership, canonical order), but FR-391 has no `depends_on` to FR-106 (snapshot populations and their `complete` flag) or to FR-107 (state-clause evaluation). FR-107 is also the requirement that has to change: today it covers no `allInstances`/`lookup` (see SR-12885 FND-001). A plan built from these edges would schedule FR-391 with no evaluation task. | spec/functional/FR-391-name-a-population-in-a-state-clause.md:5-21,122 |

## Verdict

Mergeable on this lens after the edges are added along with the evaluation
rule. Examined and clean: the FR-155 `depends_on FR-391` edge and the FR-104
`references FR-391` edge.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 5604034c5: FR-391 now has `depends_on` FR-106 and FR-107 and `traces_to` FR-155; FR-107 `references` FR-391; FR-106's required-population sentence includes the populations a clause names by `M::P`. |
