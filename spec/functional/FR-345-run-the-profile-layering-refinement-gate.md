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
  - target: ix://agent-ix/quire-specification/FR-453
    type: depends_on
---
# FR-345: Run the profile-layering refinement gate: a parent profile's refusal is its child's refusal

## Description

QSL SHALL provide `xtask refinement layering <corpus>`, a test gate that
checks, for every layering case in a corpus, that a unit its parent profile
refuses is still refused by its child profile (ADR-017 RF-3; QSpec FR-453;
QSpec capability row V1-TOOL-012). A layering case is one QSpec AD-003
`requires` edge and two units, the parent unit and the child unit, that are
byte-equal except for the identity string of one header `profile`
declaration (QSpec FR-453). The gate compiles each unit through spine
`compile`, classifies each result by FR-341, compares the two classes and
reports through FR-344. It also checks that each parent layer is a real user
subset: it admits its witness unit and prohibits its child's distinguishing
form.

Like FR-340's gate, it is a behavioural comparison. It records no
requirement record, requests no backend and settles no claim.

## Inputs

- Per case: the parent unit, the child unit, and the edge's parent and child
  layer identities.
- Per layer: a witness unit that selects only that layer.
- Per edge: a distinguishing unit that uses one form the child adds over
  the parent.

## Outputs

One case result per case, one result per layer for its witness and one per
edge for its distinguishing unit, all passed to FR-344's report. A
`regression` carries the case's name, the edge and the parent's codes; a
layer or edge regression names the layer or edge.

## Behavior

### Edges and selection

The gate SHALL cover exactly QSpec AD-003's five `requires` edges between
header-selectable layers (QSpec FR-453):

| Edge | Parent | Child |
| --- | --- | --- |
| E1 | `quire.state.core/v1` | `quire.state.queries/v1` |
| E2 | `quire.state.queries/v1` | `quire.state.graph/v1` |
| E3 | `quire.state.core/v1` | `quire.value.complete/v1` |
| E4 | `quire.state.graph/v1` | `quire.model.complete/v1` |
| E5 | `quire.value.complete/v1` | `quire.model.complete/v1` |

- Each unit's header profile names its side's layer, and FR-110's layer
  selection resolves it, so each side compiles under exactly its layer's
  admitted-form set.
- A case whose two units differ in any byte outside that identity string,
  or whose pair of layers is not one of E1 to E5, SHALL be `tool failure`
  naming the case.
- Both units of every case SHALL be compiled by the running build; no
  outcome is read from the corpus.

### Comparison

- The gate SHALL classify the parent unit's compile result by FR-341 as `R`
  and the child unit's as `C`.
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

### Each layer is a subset users pick

- **Witness.** For each of the five layers, the gate SHALL compile the
  layer's witness unit. The layer result is `holds` when its class is
  `admitted` and `regression` naming the layer and its codes otherwise.
- **Proper subset.** For each edge, the gate SHALL compile the edge's
  distinguishing unit under the parent and under the child. The edge result
  is `holds` when the parent's class is `prohibited`, naming the parent
  layer, and the child's is `admitted`; it is `regression` naming the edge
  otherwise.

### Report

- The gate SHALL report and exit by FR-344, over every case, layer and edge
  result.
- Each case SHALL be named by its two units' `RawSourceRef`s and its edge's
  two layer identities, never by a display string or a directory name.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-345-AC-1 | For every combination of `R` and `C` class, the comparison returns the result the table gives; each expected result in the test is a literal, not computed by the gate's code. | Test (TC-867) |
| FR-345-AC-2 | A case whose parent compile refuses with a typed refusal and whose child compile admits is a `regression` naming both units' `RawSourceRef`s, the edge's two identities and the parent's codes; FR-344 reports verdict violation, exit 10. | Test (TC-867) |
| FR-345-AC-3 | A case whose parent class is `prohibited` and whose child admits is `not applicable`. | Test (TC-867) |
| FR-345-AC-4 | Over a layering corpus with cases on each of E1 to E5, one seeded case per edge that its parent refuses and its child admits fails the gate naming that case and its edge, and every other case holds or is not applicable. | Test (TC-868) |
| FR-345-AC-5 | A case whose two units differ outside the header identity string, and a case naming state core and complete model, are each `tool failure` naming the case; verdict tool failure, exit 30. | Test (TC-868) |
| FR-345-AC-6 | Each of the five layers admits its witness unit; a layer that refuses it gives a `regression` naming the layer; verdict violation, exit 10. | Test (TC-868) |
| FR-345-AC-7 | On each of E1 to E5, the parent prohibits the distinguishing unit naming the parent layer and the child admits it; a parent that admits it gives a `regression` naming the edge; verdict violation, exit 10. | Test (TC-868) |

## Dependencies

- FR-341 (compile classification), FR-344 (report, verdict and exit).
- FR-110 (layer selection: a header naming any AD-003 layer resolves to it,
  and S3 admits under its admitted-form set).
- QSpec AD-003 (the profile hierarchy and its `requires` edges) and QSpec
  FR-453 (the edges, the two-unit case and the user-subset check).

## References

- ADR-017 §2 RF-3, RF-4, RF-5; §6 Q-5.
- QSpec FR-453 (STD-119, owner ruling option A: every AD-003 layer
  header-selectable, all five edges compared); QSpec FR-452, FR-290 and the
  V1-TOOL-012 re-trace (STD-117).
- Implementation ticket QSL-39; specification ticket QSL-387.

## Status

Specified; not yet implemented -- TC-867 and TC-868 planned.
