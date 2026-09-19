---
id: SR-474
title: "Base checklist review of ADR-012 semantic-family extension contracts"
type: SpecReview
analysis: base
scope: "spec/decisions/ADR-012-semantic-family-extension-contracts.md, spec/spec.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: reviews
---
# SR-474: Base checklist review of ADR-012

## Summary

Reviewed commit 048deb3 on `task/210-family-extension` against the quoin
spec-review checklist. ADR-012 is a design decision record with no US, FR, AC,
TC, option or constraint rows. The user-story, functional-requirement and six
test-coverage rules therefore have no subject. The applicable gates are ID
format and uniqueness, cross-references, link validity, terminology, and the
#210 acceptance bullets. The authoring agent ran this base pass. Independent
reviewers ran the seven analyses (SR-475 to SR-481).

Verdict: ACCEPT WITH FINDINGS (no blocking findings).

## Method

- ID format: `ADR-012` matches `^[A-Z]{2,4}-[0-9]+$`. It was reserved for #210
  by the program coordinator. No `origin/*` branch or open PR in
  agent-ix/quire-spec-language used it at authoring time.
- Local item ids: seams `S1` to `S9` (§5.1) are sequential and each is defined
  once. Section numbers run §1 to §14 with no gaps.
- Relationship targets: `ADR-010` resolves to
  `spec/decisions/ADR-010-observed-architecture-baseline.md`. QSpec `AD-016`,
  `FR-290` and `FR-340` resolve on quire-specification `origin/main`.
- `spec/spec.md` gains one `contains` relationship and one index row, and both
  resolve to the new file.
- Mermaid blocks contain no `;`.
- #210 acceptance bullets, each traced:
  - bounded change set for a sum/case form and a scoped frame clause: §12.1
    and §12.2;
  - no routine consumes a complete grammar: §4.1 and §4.3;
  - no dispatch on display text, string tags, registration order or ambient
    state: §7.1, §7.2 and §9;
  - absent capability gives a specified structured outcome and diagnostic:
    §7.3 and §5.2;
  - contracts cover check, package, lower, execute or prove, witness and
    replay: §8;
  - `/specify` and `/spec-review all`: this set.
- `quire validate --scope <repo> <ADR-012> spec/spec.md --strict --summary`:
  2/2 grammar-clean.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The checklist rules for US, FR and TC quality and the six test-coverage rules do not apply, because the ADR carries no acceptance criteria. They are recorded as not applicable, not as passed. The ADR's obligations become testable through #213, #214 and #185, which own the FRs. | ADR-012 §14 |
| FND-002 | low | Seam ids `S1` to `S9` are local to ADR-012. Other records must cite them as `ADR-012 S<n>` to stay unambiguous. Fix: state the citation form. | ADR-012 §5.1 |

## Round 2

The seven analyses were rerun against commit 8fb238b, which resolved every
round-1 high finding (`requires-bound` reachability and the single CG
`negotiate_*` settlement point). All seven round-2 verdicts are ACCEPT WITH
FINDINGS, with no open high finding. The author then addressed the remaining
medium items in the ADR: every requested item reaches `negotiate_*` once; the
mode vocabulary is decided in #222; the OBS-004 copy removal belongs to #185;
the solver-absence fault-injection test and the `xtask string-edge` scan have
owners; the AD-016 arrow-7 key change needs a QSpec issue; the unnumbered CG
and IR tickets are an owner question. Remaining low findings are recorded in
each analysis file for #212.

Round 2 verdict: ACCEPT WITH FINDINGS

## Author closure (after the PR review)

Every finding this record left open or partial has one closing line. "Fixed"
names the ADR-012 section in the commit that carries this section. "Routed"
names the owner that holds the remaining work.

| ID | Closure |
| --- | --- |
| FND-001 | Accepted as stated: the ADR carries no acceptance criteria, and #213, #214 and #185 own the FRs that make its obligations testable (§14.1). |
| FND-002 | Fixed: the Context citation rule reserves `ADR-012 S<n>` for this record's seams S1–S9 and `ADR-011 S<n>` for ADR-011 stages. |
| Round 2 text | Superseded: the CG and IR tickets are numbered (Codegen #86, Contract IR #141), and the arrow-7 key is a typed `QualifiedName` with AD-016 arrow 7 kept, so no QSpec issue is needed (§8, §13.2 Q1). |
