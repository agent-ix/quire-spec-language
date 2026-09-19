---
id: SR-466
title: "Base checklist review of ADR-011 stage DAG and dependency architecture"
type: SpecReview
analysis: base
scope: "spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/spec.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---
# SR-466: Base checklist review of ADR-011

## Summary

Reviewed commit 944a1c8 on `task/209-stage-dag` against the quoin spec-review
checklist. ADR-011 is an architecture decision record with no US, FR, AC, TC,
option or constraint rows, so the user-story, functional-requirement and six
test-coverage rules have no subject. The applicable gates are ID format and
uniqueness, cross-references, link validity and terminology. The authoring
agent ran this base pass. Independent reviewers ran the seven analyses
(SR-467 to SR-473).

Verdict: ACCEPT WITH FINDINGS (no blocking findings).

## Method

- `ADR-011` matches `^[A-Z]{2,4}-[0-9]+$`. The number was reserved for #209 by
  the coordinator.
- Local item ids are each defined once. They are: stages `S0`…`S8` (with
  `S6a`, `S6b`), side inputs `I1`…`I3`, edges `E1`…`E9`, bypasses
  `FB-01`…`FB-12`, seams `SEAM-1`…`SEAM-5`, extraction `X-1` and module moves
  `M-1`…`M-6`. They are sequential with no gaps, and every other occurrence is
  a reference.
- The relationship targets `ADR-010` and `IT-010` resolve to
  `spec/decisions/ADR-010-observed-architecture-baseline.md` and
  `spec/integration/IT-010-config-version-numeric-backends.md`.
- `spec/spec.md` gains one `contains` relationship and one index row. Both
  resolve to the new file.
- Mermaid blocks contain no `;`.
- `quire validate --scope <repo> <ADR-011> spec/spec.md --strict --summary`:
  2/2 grammar-clean.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The checklist rules for US, FR and TC quality and the six test-coverage rules do not apply, because the ADR carries no acceptance criteria. This is recorded as not applicable, not as passed. Its decisions are verified by #212 (change scenarios) and #226 (drift gates). | ADR-011 |
| FND-002 | low | The local id schemes (`S`, `E`, `FB-`, `SEAM-`, `X-`, `M-`) are not catalog id kinds. Other records should cite them as `ADR-011 FB-03` and so on, following ADR-010 Decision 5. Fix: state the citation form in the Decision section. | ADR-011 Decision |

## Round 2 (HEAD 5cbd853)

Re-ran the applicable base gates on the revision.

- FND-002 is resolved: Decision 11 states the citation form.
- The revision adds the `ix://agent-ix/quire-specification/AD-016`
  relationship, the `R` and `tool` layers, the new module `semantic_value`,
  M-6 slices `M-6a`…`M-6d`, and scenario rows 8 and 9. Local ids remain
  defined once and sequential.
- Mermaid blocks contain no `;`.
- `quire validate --scope <repo> <ADR-011> spec/spec.md --strict --summary`:
  2/2 grammar-clean.

Round-2 verdict: ACCEPT WITH FINDINGS (FND-001 remains a not-applicable note).

## Author disposition after round 2

The two-round limit is reached, so the round-2 findings were resolved in the
ADR without a third review round. Each item below names the reviewer finding
and where it is now resolved.

| Finding | Resolution in ADR-011 |
| --- | --- |
| SR-472 FND-009, SR-469 FND-015 (high): no legal carrier from `route` to E7 | §2.1 capability routing: the driver builds the registry value and passes candidate sets to CG. The crossing types are #213 value types in a crate below both QSL and CG, or data; #211 decides. No QSL library module calls CG (§6.1). |
| SR-469 FND-014 (high): allow-list omits approved external edges | §6.1: FCD from `model::intake` only; new `I3` row for `quire_source` with quire-rs under the feature; layer 6 reaches quire-rs only through `quire_source`. |
| SR-470 FND-014, SR-468 FND-017 (high): kernel ↔ layer-3 cycle | §6.1 "K is a leaf" rule names every edge X-1 must cut; the per-type cut is handed to #211 against AD-016 WP5a. |
| SR-467 FND-012, FND-013; SR-468 FND-018 | §1 I2: one binding, two outputs (import view, S4 package); the only view constructor is in `library`. |
| SR-468 FND-002, FND-009; SR-467 FND-003 | §2.1 E5 cites the binding; E9 executor key is looked up from the obligation identity (key form to #211); non-completed S6a results settle `inconclusive`. |
| SR-468 FND-019, SR-470 FND-015 | `package_identity` maps to layer 3 `library`; `containment` gains a row. |
| SR-471 FND-009, FND-010 | M-6d lands after the skeleton is green and moves CG's dev pin; `state` and `temporal` are deleted in M-6c and return in #220 and #222. |
| SR-472 FND-010, FND-001, FND-005 leftovers | Scenario 6 defers to ADR-012 §7; scenario 7 uses a `BackendDescriptor`; §2.3 no longer assigns RT and CG gates. |
| SR-470 FND-004, FND-009, FND-016 | Direction and lock checks become Owner question 9; census check moves to #219. |
| SR-468 FND-010, FND-012, FND-016, FND-020 | M-5 after M-3; M-4 ticket in Owner question 6; native-linked-package/1 submodules in SEAM-1; #29, #133, #131, #132 in the ticket table; Terms names three producers. |

Coordinator input folded into the same pass: the answers to ADR-012 §13.1,
consistency with ADR-012 L1-D1 ticket edges, and "the #134 vocabulary".
