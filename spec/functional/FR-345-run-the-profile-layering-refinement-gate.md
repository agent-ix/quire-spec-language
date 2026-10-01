---
id: FR-345
title: "Run the profile-layering refinement gate: a parent profile's refusal is its child's refusal"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-034
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-341
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-344
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-110
    type: traces_to
  - target: ix://agent-ix/quire-specification/AD-003
    type: depends_on
---
# FR-345: Run the profile-layering refinement gate: a parent profile's refusal is its child's refusal

## Description

QSL SHALL provide `xtask refinement layering <corpus>`, a test gate that
checks, for every layering case in a corpus, that a unit its parent profile
refuses is still refused by its child profile (ADR-017 RF-3; QSpec
capability row V1-TOOL-012). A layering case is one unit and one QSpec
AD-003 profile edge (a parent definition and a child definition that
requires it). The gate compiles the unit through spine `compile` once under
each side, classifies each result by FR-341, compares the two classes and
reports through FR-344.

Like FR-340's gate, it is a behavioural comparison. It records no
requirement record, requests no backend and settles no claim.

Which AD-003 edges a case may name, and how a case supplies the selection
that compiles its unit under each side, are fixed by QSpec's answer to
ADR-017 Q-5 (see References). This requirement fixes the comparison and the
report for every case once its two compile results exist.

## Inputs

Per case: the parent side's and the child side's spine `compile` results
for the case's unit, and the case's name: the unit's `RawSourceRef`
(ADR-013 O-12) and the edge's parent and child definition identities.

## Outputs

One case result per case, passed to FR-344's report. A `regression`
carries the case's name, the edge and the parent's codes.

## Behavior

- The gate SHALL classify each side's compile result by FR-341: parent
  class `R`, child class `C`.
- The comparison SHALL return the result of the first of these rows, in
  order, that matches:

| `R` | `C` | Case result |
| --- | --- | --- |
| `tool failure` | any | `tool failure` |
| any | `tool failure` | `tool failure` |
| `refused` | `refused` | `holds`, reporting both sides' codes |
| `refused` | `admitted` or `prohibited` | `regression` |
| `refused` | `unsupported` | `unresolved (unsupported)` |
| `refused` | `incomplete` | `unresolved (incomplete)` |
| `incomplete` | any | `unresolved (incomplete)` |
| otherwise | any | `not applicable` |

- A parent `prohibited` class SHALL give `not applicable`: a child profile
  admits forms its parent prohibits (QSpec AD-003), so a prohibition is not
  a refusal the child preserves.
- The gate SHALL report and exit by FR-344.
- Each case SHALL be named by its unit's `RawSourceRef` and its edge's two
  definition identities, never by a display string or a directory name.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-345-AC-1 | For every combination of `R` and `C` class, the comparison returns the result the table gives; each expected result in the test is a literal, not computed by the gate's code. | Test (TC-867) |
| FR-345-AC-2 | A case whose parent compile refuses with a typed refusal and whose child compile admits is a `regression` naming the unit's `RawSourceRef`, the edge's two identities and the parent's codes; FR-344 reports verdict violation, exit 10. | Test (TC-867) |
| FR-345-AC-3 | A case whose parent class is `prohibited` and whose child admits is `not applicable`. | Test (TC-867) |
| FR-345-AC-4 | Over a layering corpus whose cases QSpec's Q-5 answer admits, one seeded case its parent refuses and its child admits fails the gate naming that case and its edge, and every other case holds or is not applicable. | Test (TC-868) |

## Dependencies

- FR-341 (compile classification), FR-344 (report, verdict and exit).
- FR-110 (a header selects only the `root` row today, so the two sides'
  selections are supplied as Q-5 fixes).
- QSpec AD-003 (the accepted profile hierarchy).

## References

- ADR-017 §2 RF-3, RF-4, RF-5; §6 Q-5.
- QSpec half: STD-119 (ADR-017 Q-5, the layering V1-TOOL-012 covers and
  the AD-003 edges V1 requires); STD-117 (the V1-TOOL-012 re-trace).
- Implementation ticket QSL-39; specification ticket QSL-387.

## Status

Specified; not yet implemented -- TC-867 planned; TC-868 waits on STD-119
for its corpus.
