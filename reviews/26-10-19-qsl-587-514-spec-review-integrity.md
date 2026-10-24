---
id: SR-13063
title: "Integrity review of PR #1020: FR-307-AC-6 against AC-1, TC-1082 against TC-806, FR-251 coverage split"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@66ef5a3e3; FR-307-AC-1 to AC-6, TC-806, TC-1082; FR-251-AC-1 to AC-8, TC-706, TC-1081"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-307
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1082
    type: reviews
---

## Summary

Tickets: QSL-587, QSL-514. PR quire-spec-language#1020. Checks duplication and
consistency between the new AC/TCs and existing ones, given the QSL-382 review
FND-012 (SR in spec/reviews/qsl-382-spec-review.md), which deleted an earlier
FR-307-AC-6/TC-808 for restating AC-1 and asked for "a case AC-1 does not
cover". FR-251's split (TC-706 for AC-1 to AC-3, TC-1081 for AC-4 to AC-8) has
no overlap or gap.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-307-AC-6's first sentence (no element bound: one refusal per item, the export succeeds with no bound item) is a case AC-1 does not cover, as QSL-382 FND-012 asked. Its second sentence (ConfigVersion object type bound: clause item returns its binding, extent-domain item refused) restates FR-307-AC-1 on the same fixture, and TC-1082 step 2 repeats TC-806 step 1. Drop the second sentence and TC-1082 step 2, or keep only the all-refused case. | spec/functional/FR-307-export-the-bindings-each-item-references.md:88,93; spec/test-cases/TC-1082-export-partitions-requested-items-into-bound-and-refused.md:20,25 |

## Verdict

One low redundancy finding. IDs are unique and the FR-251 coverage split is
consistent.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 095366db6 and 6b1d00fb4: AC-6 drops the ConfigVersion-bound sentence that restated AC-1. TC-1082 (renamed to TC-1082-export-returns-bound-or-refused-per-item.md) replaces step 2 with an item that references no element, a case no other AC covers. |
