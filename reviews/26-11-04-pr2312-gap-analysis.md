---
id: SR-20261104231202
title: "PR 2312 (batch T-S1) gap analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language PR 2312 diff only; FR-091, FR-093, FR-397, FR-495, TC-483, TC-1091, TC-1284, TC-1334, TC-1335"
review_set: subset
---

## Summary
Spec-only diff, so no source or test files changed. Each new AC has a named TC that holds it in scope: FR-091-AC-71/72 in TC-483 (steps 16-17), FR-093-AC-58 in TC-1334, FR-093-AC-59 in TC-1335, FR-397-AC-27/28 in TC-1091 (steps 22-23) and FR-495-AC-7 in TC-1284 (step 6). The new TCs carry `verifies` edges to FR-091 and FR-093. Their trace-tag lines name the ACs and the QSpec IDs. Plan completion: not assessed.

## Verdict
No traceability gap attributable to this diff. Implementation tests follow in the coding tickets. QSL-1488's evaluation is blocked on kernel nodes (IR-1317), and QSL-1655's negation is blocked on a quire-exact IEEE negate.

## Scope
- AC-to-TC mapping and TC scope lines for all new ACs: consistent.
- New rows of the FR-091-AC-3 mapping table (`e ^ n`, `dropFirst`, `removeOne`): TC-1334 and TC-1335 tag FR-091-AC-3 for these rows.
- Ceremony check: no ledger, pin, digest or SHA records were added. QSpec is referenced by ID only.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new ACs (FR-091-AC-71/72, FR-093-AC-58/59, FR-397-AC-27/28, FR-495-AC-7) have no tagged tests yet. That is expected for a spec-first slice, and the implementation tickets must land the `#[trace(...)]` tests that the TCs name. | TC-483, TC-1091, TC-1284, TC-1334, TC-1335 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | accepted | Spec-first slice: the `#[trace(...)]` tests named by TC-483 steps 16-17, TC-1091 steps 22-23, TC-1284 step 6, TC-1334 and TC-1335 land with the coding tickets (QSL-1601/1602, QSL-1655/1654, QSL-1485, QSL-1656, QSL-1488). |
