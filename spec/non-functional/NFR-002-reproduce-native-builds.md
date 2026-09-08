---
id: NFR-002
title: "Reproduce the native compiler from pinned inputs"
type: NFR
quality_attribute: portability
relationships:
  - target: "ix://agent-ix/quire-spec-language/FR-001"
    type: constrains
  - target: "ix://agent-ix/quire-spec-language/FR-002"
    type: constrains
  - target: "ix://agent-ix/quire-spec-language/FR-010"
    type: constrains
---
# NFR-002: Reproduce the native compiler from pinned inputs

## Statement

When the recorded native build command is run, the repository shall build without a Node or JVM runtime dependency.

## Scope

Rust crate, lockfile, toolchain, CI and optional producer checks.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Unpinned direct Rust dependencies | 0 | 0 | Manifest/lock inspection |
| Required Node/JVM processes in native parse or format | 0 | 0 | CLI execution |
| Required optional Cargo features | 0 | 0 | Minimal-feature build |

## Verification

Run the pinned toolchain with the locked minimal-feature build, tests, formatter and Clippy. The optional Filament model producer check uses its explicitly selected external toolchain and is not part of native runtime qualification.

## Dependencies

- [FR-001](../functional/FR-001-read-exact-source.md)
- [FR-002](../functional/FR-002-parse-native-units.md)
- [FR-010](../functional/FR-010-report-native-outcomes.md)
