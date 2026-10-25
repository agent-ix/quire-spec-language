---
id: SR-17193
title: "QSL-926 PR #1524 dependency review of FR-337 and FR-125"
type: SpecReview
analysis: dependency
scope: "agent-ix/quire-spec-language; spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md, spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md, spec/test-cases/TC-1265-an-on-each-clause-emits-its-trigger-and-guard.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: reviews
---

## Summary

Ticket: QSL-926. PR: quire-spec-language#1524 (head a8b073ce6). The new FR-337 edges are `depends_on` FR-104 and `references` FR-381 and FR-383. Each target exists and no cycle is added: FR-104 does not reach FR-337. TC-1265 `verifies` FR-337.

On enablement, the spec cleanly separates S3/S4 (FR-337) from the letter seam (FR-125). Both are needed before FR-126-AC-16's fixtures can be produced from source. The test-built fixtures keep FR-126, FR-338 and FR-339 independent of that ordering.

Examined units: FR-337 frontmatter and Dependencies, FR-125 frontmatter and its new section's cross-references (FR-107, FR-120, FR-126, FR-156, FR-337), TC-1265 frontmatter.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Two edges are missing. FR-337's S4 rules follow FR-447's level-1 trigger, and its Dependencies prose names FR-447, but its frontmatter has no FR-447 edge. FR-125's new section rests on FR-337's checked guard ("built over a checked clause") but has no FR-337 edge. Add `references` FR-447 to FR-337, and `references` FR-337 to FR-125. Use `references`, not `depends_on`, because FR-337 already `depends_on` FR-125. | FR-337 frontmatter, FR-125 frontmatter |

## Verdict

Sound, with one low finding.
