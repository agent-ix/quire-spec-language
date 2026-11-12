---
id: SR-2690003
title: "integrity review of FR-576 operation-body carrier (PR 2690)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; spec/functional/FR-576-emit-the-operation-body-into-checked-packages-for-backend-consumers.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-576
    type: reviews
---

## Summary

Ticket: QSL-1899. Checked FR-576 for duplication and consistency with
FR-102, FR-103, FR-302, FR-094, FR-494 and the sibling FR-496. FR-576 is not a
duplicate. FR-102 retains the form, FR-103 and FR-302 join it and set
`has_body`, FR-094 builds the node, and FR-494 carries body identity only per
dispatched call candidate. None of them emits a per-operation body record or
its typed absence. There are three consistency defects.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The key "operation occurrence" is never defined: an operation is a declaration, not an occurrence. The alias rule, under which one operation reached through two model aliases gets two records, diverges from FR-496, the existing per-operation carrier, which keys `operation_effects` rows by `(document, declaring, operation)` and gives one row per document. FR-576 does not reference FR-496, so the two per-operation records would key the same operation differently. | FR-576 Behavior bullet 3; FR-576 Outputs; FR-496 Behavior |
| FND-002 | medium | For a dispatched operation, its body identity is emitted twice, once in the FR-494 dispatch candidate and once in the FR-576 record, and nothing requires the two to be equal. A consumer can read two disagreeing body handles for one operation. | FR-576 Behavior bullet 1; FR-494 Behavior; QSpec FR-322-AC-290 |
| FND-003 | low | Restated obligations. The last Behavior bullet and AC-7 restate FR-302's body type check and FR-302-AC-8, which already asserts no checked package. The second half of AC-6, where renaming the lookup label keeps identities, restates FR-094-AC-5 ("Rebuilding with a different synthesized label gives the same ids"), and that label is synthesized by `check`, not authored. | FR-576-AC-7; FR-576-AC-6; FR-302-AC-8; FR-094-AC-5 |
