---
id: FR-004
title: "Verify extracted-body correspondence"
type: FR
relationships:
  - target: "ix://agent-ix/quire-spec-language/US-002"
    type: implements
---
# FR-004: Verify extracted-body correspondence

## Description

When an extracted body map is supplied, the compiler shall verify its correspondence against immutable original bytes.

## Inputs

Original/body sources, original region, ordered segments, explicit layout policy and segment budget.

## Outputs

SourceMap yielding original ranges or a map diagnostic.

## Behavior

The map covers every body byte once and permits only selected layout deletion. Locations preserve discontiguous regions. Foreign source identity, revision, path or digest cannot be used with the map. The source-map API does not decode Markdown or prove fence ownership.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-004-AC-1 | Exact normalized segments yield the corresponding original ranges. | Test |
| FR-004-AC-2 | Deleting an interior token receives invalid_source_map. | Test |
| FR-004-AC-3 | A foreign source passed to map_span receives invalid_source_map. | Test |
| FR-004-AC-4 | An exceeded segment budget receives resource_exhausted. | Test |

## Dependencies

- [US-002](../usecase/US-002-link-exact-models.md) supplies the user need.
- [Detailed contract or implementation evidence](../../src/source_map.rs) supplies the scoped context.

## Status

Draft. Specification review and prerequisite acceptance remain distinct from existing code/tests. No acceptance criterion is claimed satisfied solely because this artifact has been authored.
