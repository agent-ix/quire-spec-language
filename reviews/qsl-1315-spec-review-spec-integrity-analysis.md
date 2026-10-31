---
id: SR-552001
title: "Spec review of quire-specification PR #552 (QSL-1315): FR-322-AC-287 equivalence anchor ordinal"
type: SpecReview
analysis: integrity
date: 2026-10-31
scope: "agent-ix/quire-specification@afbe938b4fea53a7cfa9f15476f04ca98784c470; PR #552 git diff c5deb110...afbe938b (spec/objects/interfaces/FR-322-checked-package-artifact.md AC table, spec/test-cases/TC-233-published-checked-package-and-scalar-contract-vectors.md frontmatter and EA-01); ticket QSL-1315"
review_set: subset
relationships:
- target: ix://agent-ix/quire-specification/FR-322
  type: reviews
- target: ix://agent-ix/quire-specification/TC-233
  type: reviews
---
# SR-552001: spec review of quire-specification PR #552 (QSL-1315)

## Summary

Ticket: QSL-1315. The PR adds FR-322-AC-287, which states the dimension-equivalence anchor ordinal FR-322's equivalence paragraph already requires. It also adds the TC-233 vector EA-01 and a `verifies` edge from TC-233 to the new AC.

- `quire validate` on FR-322 fails identically before (c5deb110) and after (afbe938b), with exit 3 both times. The errors are the same pre-existing two: a `value_object` frontmatter `id` is missing, and `semantic.record-invalid` reports that `fields` is required. The warnings are the same too.
- TC-233 validates with exit 0 both before and after.
- AC-287 is unique, and TC-233 is its only citer.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-322-AC-287 | spec/objects/interfaces/FR-322-checked-package-artifact.md:3472 | examined |
| FR-322 dimension equivalence paragraph (anchor ordinal, reader checks) | spec/objects/interfaces/FR-322-checked-package-artifact.md:1574-1623 | context_only |
| TC-233 EA-01 and frontmatter edge | spec/test-cases/TC-233-published-checked-package-and-scalar-contract-vectors.md:58-59, 624-630 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The last clause of AC-287 can never fail, and neither can EA-01's second half. Both say "reordering the nodes in the graph moves none". Reordering `semantic_graph.nodes` in an emitted package leaves `source_map` byte-identical, so "the reordered package gives the same ordinals" holds for every producer, including one that assigns ordinals by node-id order. The real property is that the ordinal follows source order and not node-id order. That needs a case whose node-id order is the reverse of its source order. For example, `E0` relates `B ~ C` and `E1` relates `A ~ B`, so `E1`'s node id sorts first, yet `E0` takes ordinal 0. | spec/objects/interfaces/FR-322-checked-package-artifact.md:3472; spec/test-cases/TC-233-published-checked-package-and-scalar-contract-vectors.md:630 |

## Verdict

**CHANGES REQUESTED** (FND-001, a one-sentence wording fix in AC-287 and EA-01). The rest of AC-287 is clear and testable:
- the two-node case;
- exactly one `anchor` occurrence per node;
- the declaration's own region;
- the inserted third declaration that moves the later ordinal to 2.

It agrees with the FR-322 body at lines 1598-1601. EA-01 is honestly described as test-built rather than a published fixture.
