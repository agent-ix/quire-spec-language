---
id: NFR-005
title: "Keep owned executable verification paths in Rust"
type: NFR
quality_attribute: maintainability
relationships:
  - target: "ix://agent-ix/quire-spec-language/FR-012"
    type: constrains
---
# NFR-005: Keep owned executable verification paths in Rust

## Statement

When an owned fixture or qualification check is executed, the repository shall execute its verification logic in Rust.

## Scope

The four existing audit helpers, their replacements, CI fixture assertions,
native rule-syntax checks and optional producer-verification entry points.
Standard Cargo/CI launch commands do not embed a second-language verifier.
External executable producer languages require a separate explicit owner
disposition; an implementation in another repository or a Rust wrapper is not
an exemption. This artifact records the campaign constraint without changing
the external Filament implementation or its original rights.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Owned Python audit executables or CI invocations | 0 | 0 | inspection |
| Non-Rust verification logic launched by the audit command | 0 | 0 | integration-testing |
| Unapproved external producer launches | 0 | 0 | negative-abuse-testing |

## Verification

Inspect CI, executable files and documented commands after removing the four
Python helpers. Execute Rust self-test and model-bytes checks without Python or
Node in the command's environment; run the optional packet/syntax modes against
the selected local fixtures. Trace actual test functions to TC and AC identities
with imported `ix_trace_rs::trace` and canonical `#[trace("TC-...", "FR-...-AC-...")]`
attributes under the installed Quire/module grammar. Legacy doc-comment tags
are not the convention for new tests. `#[cfg(test)]` controls compilation only.
Quire checks actual bindings separately from the macro's argument-shape check.
Check the producer refusal mode and preserve historical producer provenance.
Use pinned serde 1.0.229 and thiserror 2.0.20 only for the Rust audit target's
strict JSON/error envelope, and tempfile 3.27.0 only for isolated Rust test
fixtures; all offer MIT OR Apache-2.0 and retain their original grants.
Use the existing shared ix-trace-rs marker as a dev-dependency at commit
2ce4ebf47f726b9d76388220545cd0abda8a5cfb (release tag v0.1.1; crate version
0.1.0), retaining its AGPL-3.0-or-later grant. No local marker implementation or
second trace grammar is introduced.

## Dependencies

- [FR-012](../functional/FR-012-audit-fixtures-in-rust.md) defines the observable audits.
- [NFR-002](NFR-002-reproduce-native-builds.md) retains the pinned native build contract.
- [NFR-004](NFR-004-preserve-implementation-rights.md) governs dependency and fixture rights.
