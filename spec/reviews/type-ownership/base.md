---
id: SR-482
title: "Base checklist review of ADR-013 canonical type, package and conversion ownership"
type: SpecReview
analysis: base
scope: "spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md, spec/spec.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
---
# SR-482: Base checklist review of ADR-013

## Summary

Reviewed commit 660aa25 on `task/211-type-ownership` against the quoin
spec-review checklist. ADR-013 is a design decision record. It has no US, FR, AC,
TC, option or constraint rows, so the user-story, functional-requirement and six
test-coverage rules have nothing to check. The gates that apply are ID format and
uniqueness, cross-references, link validity and terminology. The authoring agent
ran this base pass. Independent reviewers ran the seven analyses (SR-483 to
SR-489).

Verdict: ACCEPT WITH FINDINGS (no blocking findings).

## Method

- ID format: `ADR-013` matches `^[A-Z]{2,4}-[0-9]+$`. The coordinator reserved
  it for #211.
- Local item ids are each defined once, in sequence, with no gaps:
  - `R-01` to `R-10`, in §1;
  - `O-01` to `O-27`, as §3 headings;
  - `C-01` to `C-16`, in the §4 table;
  - `Q209-1` to `Q209-7`, `Q210-1` to `Q210-4`, and `OQ-1` to `OQ-3`, in §8.
- Every other occurrence of these ids is a cross-reference. Every cross-reference
  resolves to a defined id.
- The relationship targets resolve:
  - `ADR-010` resolves to `spec/decisions/ADR-010-observed-architecture-baseline.md`.
  - `ADR-009` resolves to `spec/decisions/ADR-009-graph-path-witness-content.md`.
  - `ix://agent-ix/quire-specification/AD-016` resolves to the accepted QSpec
    assurance record on origin/main.
- `spec/spec.md` gains one `contains` relationship and one index row. Both
  resolve to the new file.
- The one Mermaid block, the §4 flowchart, contains no `;`.
- Terminology: `CanonicalDigest`, `DeclarationKey` and `CheckedPackage` keep
  their existing names. No rename is proposed.
- Wording is current-state. No compatibility, migration or fallback design
  appears. Old-version support appears only as refusal (§5).
- `quire validate --scope <repo> <ADR-013> spec/spec.md --strict --summary`
  reports 2/2 grammar-clean.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The checklist rules for US, FR and TC quality and the six test-coverage rules do not apply, because the ADR has no acceptance criteria. They are recorded as not applicable, not as passed. Normative requirement text belongs to #213 and #231. | ADR-013 |
| FND-002 | low | The item-id schemes `R-`, `O-`, `C-`, `Q209-`, `Q210-` and `OQ-` are local to ADR-013 and are not catalog id kinds. Layer 2 tickets must cite them in the form `ADR-013 O-nn` to stay unambiguous next to ADR-010 `OBS-` and `DA-` ids. Fixed in round 1: the Decision section states this citation form. | ADR-013 §3, §4, §8 |
