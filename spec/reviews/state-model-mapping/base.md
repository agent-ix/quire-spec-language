---
id: SR-786
title: "Base review of ADR-016 state, model and finite execution mapping"
type: SpecReview
analysis: base
scope: "spec/decisions/ADR-016-state-model-finite-execution-mapping.md, spec/decisions/ADR-012-semantic-family-extension-contracts.md (Status), spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md (§6 Outcomes row), spec/functional/FR-089-carry-population-identity-across-the-kernel-boundary.md (Status), spec/spec.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: reviews
---
# SR-786: Base review of ADR-016 state, model and finite execution mapping

## Summary

Reviewed ADR-016 on `spec/19-arch40-mapping`, and the
amendments made in the same commit: the ADR-012 Status paragraph, the ADR-013
§6 Outcomes row, the FR-089 Status paragraph and the `spec/spec.md` index row
and `contains` edge. The checklist was applied as it fits an ADR: ID formats
and uniqueness, link integrity, and the coverage rules for every acceptance
criterion and test the record hands to an implementation ticket.

What holds:

- The local item ids are well formed and sequential with no gaps or
  duplicates: SC-1 to SC-8, ID-1 to ID-11, EX-1 to EX-9, FE-1 to FE-5, ND-1
  to ND-4, FP-1 to FP-4, G-1 to G-7, OR-1 to OR-8 and PI-1 to PI-5.
- Every relationship target resolves. ADR-011 to ADR-015, FR-089, FR-101 and
  FR-120 exist in this repo. QSpec FR-013, FR-150, FR-151, FR-153 and FR-181
  exist in `quire-specification`.
- Every test id the record cites exists as a TC file: TC-198, TC-222 to
  TC-225, TC-230 to TC-232 (FR-085), TC-291, TC-439, TC-453 to TC-455,
  TC-467, TC-469, TC-471 to TC-474, TC-514 and TC-515.
- The acceptance criteria that G-1 and G-4 hand over exist: FR-101-AC-12 to
  AC-14 and FR-120-AC-1 to AC-12.
- The relative links resolve: ADR-012 and FR-089 to ADR-016, and ADR-016's
  Status link to `spec/reviews/state-model-mapping/integrity.md` (SR-787).
- The `spec/spec.md` row and `contains` edge are in place, and SR-786 and
  SR-787 are unused elsewhere in the repo.

What does not hold: two of the seven handed-over gaps have no acceptance
criterion id or test id. The record cites QSpec ids bare, and its front
matter omits artifacts that the G and OR rows depend on. The Status line
names review documents that do not exist yet.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | G-2 has no acceptance criterion or test id to trace to. Its "bounded acceptance criterion" is prose that sits in no FR, and its test column says "FR-063 seam probe; the existing FR-082 to FR-084 TCs unchanged". OR-2 adds "a new agreement test in G-2 over every pair of an FR-082 fixture's types", but G-2's criterion does not mention the agreement rule, and no TC id is given. Under the coverage rule (every AC has at least one TC), the SC-2/SC-6 agreement rule, which the ADR treats as normative, has no traceable check. Fix: give G-2 an AC home, either a new FR-082 criterion for the agreement rule plus an FR-063 criterion for the `StateModel` seam arms, or a named QSL-68 child. Allocate its TC id and put the agreement test in G-2's criterion and test columns. | ADR-016 §9 G-2, §10 OR-2, §1 Agreement rule |
| FND-002 | low | G-3's acceptance criterion says "FR-089 amended to match", but FR-089-AC-1 still defines the three-part preimage (package, key, role). The amendment adds only a Status paragraph. The two new behaviors, "differs only in members gives a different id" and "the unequal re-record is an `InternalFault`", have no AC id. Fix: write the new criterion into FR-089 now (for example FR-089-AC-7, marked pending under G-3) and reword FR-089-AC-1 to match. Name the TC-291 step that covers it. See SR-787 FND-001 before fixing the preimage content. | ADR-016 §2 ID-5, §9 G-3; FR-089-AC-1, FR-089 Status |
| FND-003 | low | QSpec ids appear bare. FR-181 appears six times (for example "FR-181 triple", "FR-181 state key"), and FR-331 and FR-351 appear in EX-5 and §2. QSL has no FR-181, FR-331 or FR-351, so a reader or link resolver looks in the wrong repo. SR-625 FND-009 set the convention of prefixing them. Fix: write "QSpec FR-181", "QSpec FR-331" and "QSpec FR-351" everywhere, as the record already does for "QSpec FR-013-AC-3". | ADR-016 §2 ID-4, ID-8, ID-9, §3 EX-5, Alternatives |
| FND-004 | low | The front matter omits artifacts that the decisions and hand-over depend on: QSpec FR-331, FR-351 and STD-111 (PI-3), and QSL FR-063 (G-2), FR-083 (OR-1), FR-085 (G-5), FR-106, FR-107, FR-114, FR-115 and FR-116 (FE-1 to FE-3, §12). Fix: add `depends_on` edges for the QSpec ids. Add `relates_to` edges for the QSL FRs that a G or OR row names as its criterion or oracle. | ADR-016 front matter |
| FND-005 | low | The Status line says the `/spec-review all` set is "SR-786 to SR-793". Only SR-786 (this document) and SR-787 (integrity) exist in `spec/reviews/state-model-mapping/`. The ticket's acceptance includes "Changed normative scope completes /spec-review all". Fix: write SR-788 to SR-793 (failure-domain, dependency, evidence, risk-complexity, scope-boundary, ears-conformance) before merge, or change the Status line to list only the analyses that were run. | ADR-016 Status |
| FND-006 | low | G-5's interface column says "a `model` resolver over `RelationshipEnd`". It names no function, module or return type, while every other G row names its interface exactly. The ticket's acceptance says no interface decision is left to feature implementation. Fix: name the function and signature (for example `model::relationship::resolve_end(&EffectiveView, &RelationshipEnd) -> Result<…, ModelRefusal>`) and the refusal code for a dangling end (TC-230). | ADR-016 §9 G-5; FR-085 |

## Disposition

Every finding above is fixed on `spec/19-arch40-mapping` in the ADR-016 rewrite and its listed amendments (ADR-011 M-6c and §8; ADR-012 §2, §3, §5.1, §13.5; ADR-013 O-13, T-6, QC-21, O-16, §6; FR-089; FR-120; `spec/tests.md`), except as noted below.

No finding is declined.
