---
id: SR-937
title: "Spec review of PR #551 (tool-pin text removed from FRs, TC-179, ADR-012 and ADR-013)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@cfef8e790f508ebbe8ffbdba796072805841ef1f; spec/decisions/ADR-012-semantic-family-extension-contracts.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md, spec/functional/FR-069-implement-typed-proof-result-envelope.md, spec/functional/FR-072-implement-typed-replay-result.md, spec/functional/FR-098-execute-a-replay-request.md, spec/functional/FR-122-replay-a-state-clause-counterexample.md, spec/test-cases/TC-179-proof-result-round-trip.md, spec/tests.md"
review_set: subset
---

## Summary

Ticket: QSL-351 (ToolPin part only). Base review of the spec diff.

- ADR-013 O-24 (Public type) and O-27 (Public type) drop the pin;
  O-24's remaining members match `ProofResultEnvelope` (category, record,
  `backend`, `inconclusive_cause`), and O-27's match the arm results.
- ADR-013 QC-8 drops "the executor toolchain pin in the result (O-27)" from
  its QSpec ask list; Q210-1 and ADR-012 §13.5's Q210-1 row now say the packet
  and request carry the `backend` member, not a capability. The answer to
  Q210-1 is unchanged; the pin was not part of the reasoning.
- Remaining "tool pin" text in the spec belongs to CG/AD-016 or the backend:
  ADR-011:348 (proof-metadata column definition), ADR-011:362-363 (E7/E8,
  CG's Kani pin), ADR-013:767 and ADR-014 B-5 (AD-016 Kani tool pin),
  ADR-017:682 (Verus manifest and tool pin as CG input). None claims a QSL
  type carries a pin, so leaving them is correct.
- Statement grammar: FR-069-AC-3 and FR-098-AC-2 stay single testable
  statements after the edit; FR-072 and FR-122 lists stay grammatical.
- `quire validate --scope .` over the seven changed spec files exits 0
  (warnings are module-level, pre-existing).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean. The edits remove only the pin clause; no requirement, AC id or ADR
decision changes meaning, and the left-over pin mentions are owned by CG.
