---
id: NFR-021
title: "Reproduce protocol results from exact inputs"
type: NFR
quality_attribute: reliability
relationships:
  - target: ix://agent-ix/quire-specification/FR-059
    type: constrains
  - target: ix://agent-ix/quire-specification/FR-061
    type: constrains
---
## Statement

For identical protocol, request, binding, observation, progress, closure and implementation identities, the protocol implementation SHALL produce identical ordered semantic results independently of input map order and permitted concurrent-event presentation order.

## Scope

Linked protocol identity, global replay/conformance and shared result generation.
Unordered semantic sets have one declared canonical ordering; authored sequences
and causal order are preserved.

## Rationale

Reproducible assessment is required before local output can become retained
assurance evidence. Sorting wall-clock events or JSON keys cannot replace the
protocol's actual causal and ordered/unordered distinctions.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Semantic result drift across equivalent presentations | 0 differing fields | 0 | Test (TC-063) |
| Identity drift after changing one semantic dependency | 100% detected | 100% | Test (TC-063) |
| Reproductions missing exact input/backend provenance | 0 | 0 | Test (TC-063) |

## Verification

Replay the same admitted facts under permutations of map-like inputs and
causally independent events, then compare the typed result. Independently change
each semantic dependency and verify identity changes while presentation-only
permutations do not.

## Dependencies

- [FR-059](../functional/FR-059-assess-finite-global-conformance.md).
- [FR-061](../functional/FR-061-report-orthogonal-results.md).
