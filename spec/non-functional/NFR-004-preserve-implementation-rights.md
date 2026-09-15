---
id: NFR-004
title: "Preserve implementation and dependency grants"
type: NFR
quality_attribute: compliance
relationships:
  - target: "ix://agent-ix/quire-spec-language/FR-002"
    type: constrains
  - target: "ix://agent-ix/quire-spec-language/FR-009"
    type: constrains
  - target: "ix://agent-ix/quire-spec-language/FR-011"
    type: constrains
---
# NFR-004: Preserve implementation and dependency grants

## Statement

When distribution of new implementation is requested, its owner shall record the rights of every included source, fixture and dependency before distribution.

## Scope

Native source, build/check scripts, generated content and optional producer fixtures.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| New implementation files without selected grant | 0 | 0 | Included-file inventory |
| Third-party grants silently replaced | 0 | 0 | Dependency/notice review |

## Verification

Check the owner-approved AGPL-3.0-or-later terms for new implementation and preserve inherited dependency grants. Publication remains separately reviewed; private specification prose terms remain unresolved.

## Dependencies

- [FR-002](../functional/FR-002-parse-native-units.md)
- [FR-009](../functional/FR-009-lower-qualified-projections.md)
- [FR-011](../functional/FR-011-integrate-opaque-extraction.md)
