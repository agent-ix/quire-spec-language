---
id: SR-2164004
title: "spec-dependency-analysis of FR-522 to FR-527 (quire-spec-language PR 2164)"
type: SpecReview
analysis: "dependency"
scope: "agent-ix/quire-spec-language; spec/functional/FR-522..FR-527, FR-255, FR-327, FR-330, FR-499, FR-516, spec/test-cases/TC-1317..TC-1322, TC-837, spec/spec.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-516
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-522
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-523
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-524
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-526
    type: references
---

# spec-dependency-analysis of FR-522 to FR-527 (quire-spec-language PR 2164)

## Summary

Tickets: QSL-1502 (FR-522, FR-523, FR-516), QSL-1503. Dependency pass over the relationships of the six new FRs and the edited FR-516 at head 6e7d7d34fd6a030a7dcfa8eae0f4cd3013a5585f. Edge order FR-522 -> FR-523 -> FR-524 -> FR-526 holds in the machine edges; FR-527 depends on FR-045, FR-050, FR-337, FR-515 only and not on FR-522 to FR-526, as it states. FR-525 depends on all of them and on FR-501, FR-502, FR-504, FR-516. The cycles are below.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | [QSL-1502] FR-516 now depends_on FR-522, FR-523, FR-524 and FR-526, and each of those four depends_on FR-516, so there are four two-edge depends_on cycles (FR-522 -> FR-516 -> FR-522). The PR's own text calls FR-516 the carrier the ports feed ('Feeds QSL-1545') and the ports the 'upstream'. Make the direction one-way: either FR-516 references the ports and they depend on it, or it depends on them and they carry references/extends toward FR-516. | spec/functional/FR-516-carry-the-temporal-evidence-request-and-result-on-the-checked-package-route.md:28 | wrong-requirement |
| FND-002 | low | [QSL-1502] The requested order is FR-522/523 -> 524 -> 526, and the machine edges follow it (523 -> 522, 524 -> 523, 526 -> 522/523/524) but the prose does not: FR-522's identity includes FR-523's ceilings and FR-526's relation, FR-523's lineage counter is defined by FR-526, and FR-522's Dependencies cite both, with no edge from FR-522 to FR-523 or FR-526 or from FR-523 to FR-526. Add the edges or reduce the prose to a reference; the identity rule needs FR-523 and FR-526 members, so state which owns the identity function. | spec/functional/FR-522-bind-a-temporal-trace-to-the-checked-clause-s-profile-and-clock-and-report-the-authenticated-clock.md:26 | missing-requirement |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-516 relationships to FR-522, FR-523, FR-524 and FR-526 are references and each of them states it references FR-516 rather than depends on it; a depends_on cycle search over the spec finds none. |
| FND-002 | still-open | FR-522 now states that FR-523 and FR-526 depend on it, but FR-523's Dependencies still list FR-524 (activation charges) and FR-526 (lineage) as dependencies while FR-524 and FR-526 depend on FR-523, and no edge from FR-523 states them: a prose cycle among FR-523, FR-524 and FR-526 remains. Reduce the FR-523 prose to references. |

## Verdict

2 findings: 0 high, 1 medium, 1 low. Tickets: QSL-1502 2.

Disposition pass 1 at head 87bdf5f2828737f46113bab3ee801a1e369fd27b: 1 fixed, 1 still-open, 0 new findings.
