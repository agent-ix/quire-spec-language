---
id: SR-522
title: "Dependency review of the M-6a owner-question rulings"
type: SpecReview
analysis: dependency
scope: "PR #353 diff against main: ADR-011 §2.4, §7.3, the 2026-09-22 rulings, the QSpec question list and the tickets table; ADR-013 O-04, QC-18, TK-08"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
  - target: ix://agent-ix/quire-specification/AD-016
    type: references
  - target: ix://agent-ix/quire-specification/FR-322
    type: references
---

## Summary

The rulings add three cross-repository edges. The last M-6a change now waits
on the skeleton spine, which needs QSL #243 and
agent-ix/quire-contract-codegen#87. v2 emission waits on a QSpec lock
accessor. QSL's type-node keys wait on a QSpec preimage arm. The first two
edges put #216 behind a QSpec deliverable, because E9 recompiles through S4
and has to reproduce `package_id`, and no `package_id` exists without an
edition selection.

The diff names each QSpec ask but cites no ticket for any of them. It also
does not record that SG-2 departs from AD-016's `capability_report` rows. OQ-5
is correctly taken off the critical path.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | #216 now depends on a QSpec deliverable. The chain is: the QSpec lock accessor → an edition selection, without which the emitter refuses (§2.4) → S4 `package_id` → the E9 recompile in the skeleton spine (§1.1, ADR-013 O-26) → a green skeleton → the last M-6a change (IT-010 and `lowering` deletion) → #216. §2.4 records the accessor only as "Remaining work: the QSpec lock accessor, owned and raised by QSL". It cites no ticket, and the accessor is missing from "Tickets to open at #212". The skeleton edge (QSL-5 and agent-ix/quire-contract-codegen#87 block the last M-6a change) is also stated only in prose. Fix: file the accessor ticket in Linear, cite its ID in §2.4 and in the tickets table, and add Linear `blocks` edges from the accessor to QSL-5 and QSL-8, and from QSL-5 and agent-ix/quire-contract-codegen#87 to QSL-8. | ADR-011 §2.4 (:502-505), §1.1, §7.3 M-6a row, OQ-3 and OQ-6 rulings; ADR-013 O-26 |
| FND-002 | medium | SG-2 departs from QSpec AD-016 at 2449ceb. AD-016 says the QSL capability point "Records `capability_report`" (AD-016:120), that "Unsupported families are carried in `capability_report`" (:133), and that IR reads "`capability_report` is data for arrow 4" (:136). Under SG-2, `capability_report` is FR-322's feature-level report, and per-item `Requirements` travel outside it. ADR-011's E3 row flags AD-016 arrow 1 as stale for identity only, and the "To QSpec" list has no AD-016 amendment. Fix: add an AD-016 amendment ask to "To QSpec" (arrow 1 and the IR row's `capability_report` wording, and the per-item carrier of SR-521 FND-001). Add "AD-016 arrow 1's `capability_report` wording is stale here" to the E3 Proof-metadata cell. | ADR-011 §2.2 E3, "To QSpec"; QSpec spec/assurance/AD-016-semantic-family-extension-path.md:120, :133, :136 |
| FND-003 | medium | No spec defines the proposed type-node preimage. OQ-7, O-04 and ADR-011 E3 say QSL implements a proposed preimage until QSpec publishes one, but no FR or ADR in this repository gives its members. QC-18 only asks QSpec for the arm. Application-node keys hash type-node keys, so every emitted `package_id` depends on this undefined preimage. Fix: name the owning requirement (the A4 or FR-091-AC-18 work) and state the members there: the JCS SHA-256 of the node without `node_id`, occurrences and its self-referential `semantic_type`, per the research option A. Cite it from O-04. | ADR-013 O-04 (:178), QC-18 (:992); ADR-011 OQ-7 ruling (:1278-1286), E3 (:356) |
| FND-004 | low | The QSpec asks in "To QSpec" (the accessor, the type-node arm, the `dependency_selections` schema defect and the fixture placeholders) and TK-08's QC-18 carry no ticket reference. Fix: cite the Linear IDs once they are filed. The QSL-side accessor blocker is QSL's to raise, as §2.4 says. | ADR-011 :1182-1200; ADR-013 TK-08 (:1060) |
| FND-005 | low | QSL's `DefinitionLock` catalog (src/value/definition.rs:313-480, `revision()` :537) restates the identities and revisions in QSpec's `complete-value-lock.json`, and it has drifted. §2.4 bans digest constants but leaves this restatement standing. Fix: in §2.4, state that the accessor replaces the catalog, and put that removal in the accessor ticket (FND-001). | ADR-011 §2.4 (:502-508) |
| FND-006 | low | The M-6c lane gains native `run`, `NativePackage`, `runtime`, `mapped` and `model_source`, but T-3 (:1300) still lists #217 among the tickets that take M-6b to M-6e deletions, although M-6b now deletes nothing. T-3 also does not add native `run` to the #120, #121 and #164 exit criteria. Fix: drop #217 from T-3, and name native `run` and its SEAM-1 modules against #120, #121 and #164. | ADR-011 T-3 (:1300), §7.3 M-6b and M-6c rows |
| FND-007 | low | Not edited here, because another in-flight PR owns them: FR-091-OQ-2 and FR-091-OQ-3 are now answered or stale. §2.4 gives the source of lock evidence, and QC-18 no longer carries `name@version`. FR-091-AC-18's dependency on OQ-3 moves to OQ-7 and FND-003. | FR-091:401-414 |

## Resolution

Fixed in the PR: FND-002 (AD-016 amendment added to "To QSpec"; E3 marks arrow 1 stale), FND-003 (O-04 names QSL-156 as the owner of the proposed preimage), FND-005 (the accessor replaces the catalog's restatement), FND-006 (T-3 adds native `run` to the M-6c exit criteria).
Left for the lead: FND-001 and FND-004. The Linear tickets and `blocks` edges for the QSpec asks are ticket work outside this spec PR. FND-007 belongs to the in-flight FR-091 rulings PR.
