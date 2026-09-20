// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-058-AC-3 (#215): deliberately empty. This crate stands in for
//! quire-contract-ir's real `quire-contract-model` package via the
//! intentionally incompatible fixture's `[patch]`, and declares none of the
//! types quire-spec-language's real source imports from it (`Diagnostic`,
//! `SourceIdentity`, `SourceSpan`, `AnchorName`, `ValueDeclaration`,
//! `DiagnosticCode`, `ExecutionPoint`, and others). Building
//! quire-spec-language against it is expected to fail to compile with
//! unresolved-import errors naming these missing types.
