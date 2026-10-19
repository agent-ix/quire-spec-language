---
id: SR-13267
title: "Spec review of PR #1038: dependency analysis"
type: SpecReview
analysis: dependency
scope: "agent-ix/quire-spec-language@651a86903; git diff origin/main...HEAD (PR #1038)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-140
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-141
    type: reviews
---

## Summary

Ticket: STD-206. Dependency analysis of the new normative references and the QSpec prerequisites at 651a86903. Clean: TC-1083 verifies FR-140; TC-540, TC-544 and TC-555 scopes match the new ACs; FR-139's new prose reference to FR-140 adds no cycle edge.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-140 Outputs EvalFailure and MappingRowRef prose | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md | examined |
| FR-135 complete-value typing bullet | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| FR-135 Int-Integer equality bullet | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md | examined |
| FR-139 sum bound bullet | spec/functional/FR-139-type-and-evaluate-population-valued-expressions.md | examined |
| FR-139 only-not-one bullet | spec/functional/FR-139-type-and-evaluate-population-valued-expressions.md | examined |
| FR-141-AC-7 | spec/functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md | examined |
| TC-1083 | spec/test-cases/TC-1083-the-refinement-mapping-lifts-references-and-reports-what-it-cannot-map.md | examined |
| PR #1038 body: QSpec asks | PR #1038 body | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Four new QSL rules depend on QSpec changes that have no ticket and no blocker edge (Int=Integer, sum bound, only-not-one reason, nested lifting). The PR lands QSL SHALLs ahead of the QSpec text they implement, and one of them is contrary to it (base SR FND-001). A Linear search for only-not-one or the new sum bound finds no ticket. File one QSpec STD ticket per ask and link it as blocking, or as related where QSL leads by design, so the code lanes (#1035 already emits only-not-one) do not ship ahead of an unfiled contract. | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:143-145 |
| FND-002 | low | FR-140 now cites ADR-013 O-16 and ADR-018 UE-2 normatively (the Undefined cause type), but its frontmatter has no depends_on edge to either. Add depends_on edges to ADR-013 and ADR-018 so a change to O-16 or UE-2 reaches FR-140 in the link graph. | spec/functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md:95-102 |
| FND-003 | low | FR-135 now rules on step-row arguments and history updates (FR-136, FR-138) and on the division law (FR-091), and FR-141-AC-7 relies on the law, with no edges. FR-136 and FR-138 already depend on FR-135, so an edge back would be a cycle. That is a sign the bullet belongs in FR-136/FR-138 (scope-boundary SR). FR-135 and FR-141 need an edge to FR-091 (or QSpec FR-147) for the division law. | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:134-142 |

## Verdict

Findings: 1 medium, 2 low. Fix the QSpec prerequisite tickets before the code lanes build on these rules.

## Dispositions

Round 1, head 72d04b1dc (fix commits d561b72e7, 72d04b1dc).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |

Disposition pass 1 (head 72d04b1dc): all three fixed. No new findings.
