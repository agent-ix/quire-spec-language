---
id: SR-2026111116452400000002
title: "integrity review of PR 2679 negotiation cause, read-surface views and unordered loss"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; spec/functional/FR-057-admit-shared-capability-kinds.md, spec/functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md, spec/functional/FR-519-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md, spec/test-cases/TC-155-keep-admission-backend-independent.md, spec/test-cases/TC-651-protocol-step-kinds.md, spec/test-cases/TC-1314-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md, spec/decisions/ADR-027-protocol-transition-system-for-parallel.md"
review_set: subset
---

## Summary

Ticket: QSL-1877. Structural and cross-document consistency of the 7 changed files at 8eaaab74e: id freeness, scope lines, trace tags, table shape, citations, and paragraph placement. FR-057-AC-15, FR-519-AC-18/19 and FR-206-AC-12 are each defined once, and no other artifact in spec/ or reviews/ uses those ids. The TC-155, TC-651 and TC-1314 scope lines and the TC-1314 trace tag were updated to match. The FR-057 Dependencies bullets for FR-271/FR-272 and ADR-011 were checked and are consistent apart from base FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new block is inserted in the middle of the `attempts_maximum` paragraph. The sentence "It refuses with `ViewRefusal`, and returns no value, when `compensation` is absent ... (`NotACompensation`) ... `attempts.maximum`" now follows the `channel_endpoints` paragraph. As written, the spec says `channel_endpoints` refuses `NotACompensation` and `attempts.maximum`, and `attempts_maximum` loses its refusal text. Fix: move "It refuses with `ViewRefusal` ... `MalformedMember` naming `attempts.maximum`" back after "the integer encoding of QSpec FR-322." and before "Two further view functions". | spec/functional/FR-519-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md:131-137 |
| FND-002 | medium | TC-1314 rows 18 and 19 have 3 cells, but the table header has four columns (Row, AC, Input, Expected). The AC column is missing, so the input sits under AC and the expected result under Input. check-spec-validation does not catch this. Fix: give each row its AC number in the second column (18 and 19). | spec/test-cases/TC-1314-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md:37; spec/test-cases/TC-1314-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md:38 |
| FND-003 | low | `await_anchor` cites "QSpec FR-464" for the `after` Group layout. FR-464 has only the `await_anchor` causal-relation row. The Group layout ("`after`: a Group, `kind` a text `event` or `compensation`, `target` a reference") is in QSpec FR-466 (checked-package protocol control bodies), at line 62. Fix: cite FR-466 for the layout, and FR-464 for the causal edge if that citation is wanted. | spec/functional/FR-519-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md:115 |

## Verdict

FAIL. FND-001 (the text spliced into attempts_maximum) and FND-002 (the TC-1314 row shape) are mechanical fixes.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Verified in 626c658b9. |
| FND-002 | fixed | Verified in 626c658b9. |
| FND-003 | fixed | Verified in 626c658b9. |
