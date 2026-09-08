---
id: NFR-001
title: "Bound syntax work and storage"
type: NFR
quality_attribute: performance_efficiency
relationships:
  - target: "ix://agent-ix/quire-spec-language/FR-001"
    type: constrains
  - target: "ix://agent-ix/quire-spec-language/FR-002"
    type: constrains
  - target: "ix://agent-ix/quire-spec-language/FR-003"
    type: constrains
  - target: "ix://agent-ix/quire-spec-language/FR-004"
    type: constrains
---
# NFR-001: Bound syntax work and storage

## Statement

When the next syntax-processing operation would exceed a selected resource ceiling, the compiler shall return a resource_exhausted diagnostic without performing that operation.

## Scope

Native source intake, tokenization, parsing, formatting and source-map validation.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Default source/output ceiling | 1048576 bytes | 1048576 bytes | Boundary tests |
| Default token ceiling | 100000 tokens | 100000 tokens | Boundary tests |
| Default syntax-node ceiling | 50000 nodes | 50000 nodes | Boundary tests |
| Default recursive/delimiter ceiling | 64 levels | 64 levels | Depth tests |
| Source-map segment ceiling | 50000 segments | 50000 segments | Boundary tests |

## Verification

Run the native boundary and malformed-input tests. Check a 20000-operator flat chain on a 512 KiB stack. Caller-supplied ceilings may be lower; an implementation ceiling is not a domain bound.

## Dependencies

- [FR-001](../functional/FR-001-read-exact-source.md)
- [FR-002](../functional/FR-002-parse-native-units.md)
- [FR-003](../functional/FR-003-format-native-source.md)
- [FR-004](../functional/FR-004-verify-source-maps.md)
