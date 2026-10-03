---
id: NFR-005
title: "Keep owned executable verification paths in Rust"
type: NFR
quality_attribute: maintainability
---
# NFR-005: Keep owned executable verification paths in Rust

## Statement

When an owned fixture or qualification check is executed, the repository shall execute its verification logic in Rust.

## Scope

The repository's fixture assertions and qualification checks. Their
verification logic is the code that compares a fixture or vector with its
expected result. Standard Cargo launch commands do not embed a second-language
verifier. An implementation in another repository or a Rust wrapper is not an
exemption. The `conformance` Makefile target's shell `grep` only checks that
each Rust conformance test printed its summary line, so that a test filter
matching nothing cannot pass. It compares no fixture or vector; the Rust tests
do.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Fixture or qualification checks whose verification logic is not Rust | 0 | 0 | inspection |

## Verification

Inspect the fixture and qualification checks: each runs as a Rust test or a
Rust tool. Trace actual test functions to TC and AC identities
with imported `ix_trace_rs::trace` and canonical `#[trace("TC-...", "FR-...-AC-...")]`
attributes under the installed Quire/module grammar. Legacy doc-comment tags
are not the convention for new tests. `#[cfg(test)]` controls compilation only.
Quire checks actual bindings separately from the macro's argument-shape check.
Use tempfile only for isolated Rust test fixtures; it offers MIT OR
Apache-2.0 and retains its original grant.
Use the existing shared ix-trace-rs marker as a dev-dependency tracking its
`main` branch, retaining its AGPL-3.0-or-later grant. No local marker implementation or
second trace grammar is introduced.

## Dependencies

- [NFR-002](NFR-002-reproduce-native-builds.md) retains the pinned native build contract.
- [NFR-004](NFR-004-preserve-implementation-rights.md) governs dependency and fixture rights.
