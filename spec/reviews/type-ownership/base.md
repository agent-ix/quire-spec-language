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

## Round 2 (commit 0042691)

Re-checked after the round-1 revision. The id kinds are now `R-01` to `R-10`,
`O-01` to `O-27`, `C-01` to `C-25`, `S-1` to `S-6`, `QC-1` to `QC-7`, `Q209-1`
to `Q209-7`, `Q210-1` to `Q210-4`, `Q222-1` and `Q222-2`, `Q229-1`, and `OQ-1`
to `OQ-4`. Each runs in sequence with no gaps and is defined once. Every
cross-reference resolves. The Mermaid block still contains no `;`. A scan for
compatibility, fallback, migration and transitional wording finds none in the
decision text. `quire validate --strict --summary` on ADR-013, spec.md and the
eight review files reports 10/10 grammar-clean.

- FND-001: unchanged, not applicable.
- FND-002: resolved. The Decision section states the citation form and now
  lists every local id kind.

Round-2 verdict: ACCEPT WITH FINDINGS (FND-001 only).

## After round 2

The round-2 medium and low findings in all eight reviews are resolved in the
ADR. The same commit adds coordinator-requested answers: ADR-012 §13.2
questions (OQ-5, O-10, O-14, O-17, O-19; O-20 decided in #222) and the nine
ADR-011 questions to #211 (§3.1, T-1 to T-9, with QC-10 to QC-12 and Q209-8).
§3.1 was added after round 2 and has had no separate review round; the two-round
limit applies.
