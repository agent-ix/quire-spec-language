---
id: SR-1933001
title: "Base review for QSL-498 retirement and QSL-1255 replacement tests"
type: SpecReview
analysis: base
scope: "FR-493, TC-1276, TC-137, TC-136, NFR-009-AC-3, FR-365 citation; changed files only"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-493
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1276
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-137
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-136
    type: reviews
  - target: ix://agent-ix/quire-spec-language/NFR-009
    type: reviews
---

## Summary

Base checklist run over the diff. Validation of the changed files is clean. Matrix status for the changed rows is untagged or partial, as expected for a spec-only change; the repository-wide `quire matrix` did not finish in 5 minutes (the source tag scan is the slow part; a spec-only scope returns at once).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-493 listed `formal_source` and "native linking or checking" among the retired modules. ADR-011 SEAM-2 and ADR-012 section 15.8 step 2 say `formal_source` belongs to the composed lane, which `checking::composed` imports, and is not deleted in step 2; the composed lane stays normative under FR-362 item 9. Fixed: scope now names `linking::native`, native `checking`, `model_source`, `located_json`, and states that `composed`, `formal_source` and `native_model` stay. | FR-493, ADR-012 15.8, FR-362 item 9 |
| FND-002 | high | Hash/digest/scan antipattern: FR-493-AC-1, AC-3, AC-4, AC-5 and TC-1276 steps 2 and 5 required scanning tracked files and spec text for strings and checking that documents are absent. Fixed: replaced with compile-fail, exhaustive-match and build-and-run controls; the matrix check stays. | FR-493-AC-1, AC-3, AC-4, AC-5, TC-1276 |
| FND-003 | high | OPEN: FR-100 depends on FR-028 and carries a `package` run-request member (lines 55, 238, 525, 634); FR-031 Behavior and AC-6 (`extraction-package-conflict`) and FR-026 cite it too. FR-493 retires FR-028 with replacement "FR-109, FR-100", but FR-100 itself is the consumer. FR-493 now records the amendment in its retained-requirements table; the owner must confirm the member is removed with no spine successor (ADR-012 15.8 step 2 retires NativePackage) before the edit is applied. | FR-100, FR-031, FR-026, FR-028 |
| FND-004 | medium | FR-493-AC-4 required the compiled-protocol reader to be absent while FR-042-AC-15 and FR-050-AC-8 handoff writers still call it. Fixed: deletion is ordered with the last caller (M-6d). | FR-493-AC-4, FR-042-AC-15, FR-050-AC-8 |
| FND-005 | medium | TC-137 control C2 expected the incomplete record to name the counter of the charge point. quire-exact records an injected denial against `work_units` with limit equal to consumed. Fixed. | TC-137 C2, quire-exact Incomplete |
| FND-006 | medium | The native front-end and protocol-artifact requirements had no per-criterion disposition. FR-493 now lists dispositions: FR-001 keep; FR-016, FR-017, FR-025 retire with SEAM-1; FR-035/036/040 keep (composed lane, FR-036-AC-5 stays live for FR-049-AC-10); FR-041 split; FR-042 retire with M-6d. OPEN: FR-041 split rows and the FR-042 dependents FR-043 to FR-050, FR-052, FR-053 need owner confirmation (QSL-837, QSL-838). | FR-493 |
| FND-007 | medium | OPEN: FR-016 is cited by FR-104 (the `CheckBindings` adapter that feeds execution points) and FR-098 (short-circuit order). Both must be re-pointed in the retiring slice; recorded in FR-493. | FR-104, FR-098, FR-016 |
| FND-008 | low | Verification cells name `TC-` ids, which the checklist discourages; the repository uses this form throughout, so the new rows follow it. | FR-493, NFR-009-AC-3 |
| FND-009 | low | Matrix status of changed rows: FR-493-AC-1 to AC-5 and NFR-009-AC-3 untagged; FR-049-AC-7 partly tagged (TC-1048 rows 11 to 14 only); TC-137 has no tagged test yet. Expected until the code tickets land. | FR-493, NFR-009, FR-049-AC-7 |
| FND-010 | low | TC-137 charge points were verified to exist in quire-exact at the locked revision, and the injected-denial and `test-support` admitted-charge log APIs exist. TC-136 H1 to H4 entry points (`call`, `evaluate_holds`, `evaluate_protocol_slot`, `ProtocolSlot`, `InputRefusal::UnknownFunction`) exist in the workspace; H2 (a non-callable declaration yields `UnknownFunction`) was not confirmed in source. | TC-136, TC-137 |
