---
id: SR-001
title: "Integrity and criterion-strength review of the FR-052 owner-report retirement"
type: SpecReview
analysis: integrity
scope: "FR-052, FR-526, FR-516, TC-1321, TC-144 (diff origin/main...spec/qsl-fr052-owner-report)"
review_set: subset
---

## Summary

Reviewed the spec-only diff that retires FR-052-AC-17/18/21 with the native
owner-report byte carrier and ports FR-052-AC-15/16/19/20 to FR-526-AC-20..28
(TC-1321 rows 20..28). The lenses were base, integrity/consistency and criterion
strength, each checked against FR-501, FR-516, FR-522, FR-523 and FR-524 and
against the current qsl-eval code. No port narrows its criterion without a
justification. The multiplicity narrowing holds: FR-501 defines decision support
as a set of `(position, atom)` pairs, and `EvidencePosition.valuations` is a
`BTreeMap<u64, Valuation>`. The FR-052-AC-19 equal-identity arm becomes an argument
instead of a test, and that is acceptable given FR-526-AC-16/AC-17. There are two
major defects: a fixture that does not exist, and a type-name collision. Several
requirements need code changes that the spec does not mark.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-526-AC-22/AC-23 and TC-1321 rows 22/23 name "R's event-position clause `on origin eventually[0,1] (holds(a) and holds(b))`", but R's event-position package (examples/checked-handoff/unit-event-position.native) holds only `always[0,30]` and `always[0,60]` clauses. Fix: say it is a constructed event-position package that holds this clause, or add the clause to R through FR-517. | FR-526-AC-22, FR-526-AC-23, TC-1321 |
| FND-002 | high | The result member `subject: EvidenceSubject` (FR-526 line 83, FR-516 line 114) reuses the name of FR-515's existing input type `EvidenceSubject<'p>` (request.rs:430), which has different members. Fix: rename the result type, for example `ResultSubject`, in both files. | FR-526, FR-516, FR-515 |
| FND-003 | medium | FR-526 now keys result entries by occurrence (lines 66, 107), but FR-524 line 109 still says the ledger is keyed by "the package identity and the clause" and the trigger identity. The code `ObligationKey` already carries `occurrence`. Fix: add the clause occurrence to the FR-524 key text. | FR-524, FR-526 |
| FND-004 | medium | The spec requires code changes it does not mark as new. `EvidenceResult` has no `subject` member. Support construction (call.rs ~1821) drops silent samples only: it does not filter observed positions whose atom valuation is absent (FR-526 line 158) and does not deduplicate pairs (line 152). Fix: state these as implementation changes in the ticket and on FR-526-AC-20/AC-23. | FR-526-AC-20, FR-526-AC-23 |
| FND-005 | medium | The multiplicity claim (line 152) leaves out `EvidenceRequest.native_valuations: Vec<u8>`. That field is the residue of the duplicate-preserving native relation, and it is still encoded into the input identity (call.rs:611). Fix: state that the field is removed with the native route. | FR-526, FR-516 |
| FND-006 | low | Tests in tests/it/native_owner_report.rs and tests/it/native_full_predecessor.rs still carry trace tags for the removed FR-052-AC-15..21. Fix: re-tag or delete them in the implementing PR. | FR-052, TC-144 |
| FND-007 | low | FR-526-AC-21 asks for "equal evidence" over the FR-516-AC-32 twins, but those twins have different clock rows, so the binding must differ. FR-526-AC-27 asks for a ledger equal "byte for byte", but the ledger has no byte form. Fix: write "equal evidence apart from the occurrence's binding" and "a `ledger` equal (`==`) to the request's". | FR-526-AC-21, FR-526-AC-27 |
| FND-008 | low | The "attempted `lineage`" (line 182) is undefined when `checked_add` overflows; the code then reports 0. Fix: name the value for the overflow case. | FR-526 |
| FND-009 | low | The FR-052 retirement text says AC-18 exists only for the carrier, but its subject/occurrence `BindingMismatch` arm has a spine successor in FR-516-AC-32 and FR-522. Fix: cite them. | FR-052 |
