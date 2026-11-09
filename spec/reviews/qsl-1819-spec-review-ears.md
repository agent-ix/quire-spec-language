---
id: SR-20261109172912000002
title: "EARS review of the L5 engine-gap spec batch"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language; QSL-1819 (with QSL-1820); branch spec/l5-engine-gaps @ 9514a0e41 vs origin/main db995d817; ADR-029; FR-195; FR-201; FR-280; FR-281; FR-282; FR-286; FR-564; FR-565; FR-567; TC-761; TC-762; TC-763; TC-770; TC-1391; TC-1393; TC-1394"
review_set: subset
---

## Summary

Ticket: QSL-1819 (with QSL-1820). EARS conformance of the requirement statements this batch adds or edits in FR-195, FR-201, FR-280, FR-281 and FR-286 (Behavior bullets and new normative prose).

## Verdict

Mergeable on EARS grounds once FND-001 is fixed. Clean: FR-280's new EN-5 manifest and BackendId bullets, FR-281's provision-refusal bullet (an unwanted-behaviour If/then form with its precondition), its build-the-request bullet, its counterexample When/shall bullet, FR-286's from_analyze shall bullet, and FR-195's Capability SHALL.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The provider entry's missing-provision behaviour (every item `declined`, incomplete-input, missing_import/missing-selection) appears only in Description prose. FR-280 Behavior has no If/then shall bullet for it, and no FR-280 AC or TC-761 step exercises it. analyze refuses the whole request for the same input (AnalyzeRefusal::Provision), so the two paths differ, and the difference is unverified. Add the EARS bullet and an FR-280 AC (TC-761 step) for the case. | FR-280:104-107, 179-200 |
| FND-002 | low | 'shall run no item adapter that takes a caller-supplied closure' is a negative design constraint with no observable response. Its only check is AC-2's 'No closure, letter table or verdict is supplied by the test', which describes how the test is written, not something it can observe. Restate it as a signature property (analyze and provider::run take no fn or closure parameter) checked by a compile-time or API-surface test, or drop the bullet, since the build-the-request bullet already implies it. | FR-281:469-471, 517 |

## Coverage

- FR-195 (examined)
- FR-280 (examined)
- FR-281 (examined)
- FR-286 (examined)
- FR-201 (examined)
