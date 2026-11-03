---
id: SR-20261103155256
title: "QSL-1556 r1 integrity analysis of PR #2205 (FR-517 recipe table and paragraph)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; PR #2205 head vs origin/main; FR-517 recipe table rows 3 and 6, new paragraph, FR-517-AC-14"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-517
    type: references
---

# QSL-1556 r1 integrity analysis of PR #2205

## Summary

Ticket: QSL-1556. Checked the new FR-517 text against the rest of FR-517
(the table, AC-10, AC-14, and the dependencies list) for consistency. QSL-1556
and QSL-1555 are named consistently. `quire validate` on FR-517 reports no
errors.

## Verdict

One low inconsistency: the unchanged `workflow` row and the new paragraph
describe R's `recover` conditions differently.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The `workflow` row says the `recover` conditions "test `size` of the receipts (FR-517-AC-14)". FR-517-AC-14 and the new paragraph ("their `recover` conditions are those of FR-517-AC-14") say the conditions compare the receipt sum with the captured target. `size` is R's interim form while QSL-1555 is open. The row is unchanged by this PR, but the PR places a contradicting statement two lines below it. Make the row match AC-14 (sum against the captured target), as the paragraph does. | spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md:139, :183 |
