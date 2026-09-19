// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-058-AC-3: intentionally incompatible fixture. This crate exists only so
//! that `cargo build` compiles `quire-spec-language` (and, transitively, the
//! `quire-contract-ir` dependency this manifest patches to
//! `stub-quire-contract-model/`, a deliberately empty stand-in) declared in
//! `Cargo.toml`; it is never itself linked into anything else. Building it is
//! expected to fail.

pub use quire_spec_language as _incompatible_fixture_dependency;
