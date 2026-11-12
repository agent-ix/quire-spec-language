---
id: SR-2690005
title: "scope-boundary review of FR-576 operation-body carrier (PR 2690)"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quire-spec-language; spec/functional/FR-576-emit-the-operation-body-into-checked-packages-for-backend-consumers.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-576
    type: reviews
---

## Summary

Ticket: QSL-1899. Checked FR-576's stated boundary: QSL owns emission, QSpec
FR-322 owns the wire, and quire-driver and CG own the read. The text crosses
the boundary on both sides and conflicts with QSpec FR-322 as it stands on
`main`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-576 says it "owns no wire shape", yet its Outputs and ACs fix the wire: the record goes in `checked_term_data`, and an absence is a "body member" `no_body`. QSpec FR-322-AC-288 on `main` defines `checked_term_data` as a closed carrier with exactly the arrays `equality_decisions`, `protocol_locations`, `dispatch_tables` and `probabilistic_claims`, so a package emitted to FR-576 is refused by the strict reader. The precedent FR-496 uses a separate top-level `operation_effects` member. FR-576 should name only the content QSL must emit and defer the carrier, the member name and the absence encoding to QSpec FR-322. | FR-576 Outputs; FR-576-AC-2; QSpec FR-322-AC-288; FR-496 |
| FND-002 | medium | AC-5 verifies the quire-driver typed read, which FR-576's own Description says it does not own. "No step that parses source or walks the graph" is also not observable from a QSL test. Move it to the quire-driver consumer requirement. On the QSL side the testable form is that the record contains every handle a reader needs (operation identity plus body NodeRef or `no_body`). | FR-576-AC-5; FR-576 Description |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The record is the top-level `operation_bodies` member following FR-496, not a `checked_term_data` array. FR-576 says FR-322-AC-288 closes that carrier and that QSpec FR-322 owns the wire shape and reader joins, through STD-660. |
| FND-002 | fixed | The quire-driver read AC is gone. AC-9 tests, inside QSL, that a row plus its named graph node yields the operation identity and the body by lookup alone. |
