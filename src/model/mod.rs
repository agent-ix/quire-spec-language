// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-150-153: the `quire.model.complete/v1` model graph — original/effective
//! declaration provenance, inheritance and conformance, systems-model binding
//! and closed model environments.
//!
//! [`domain_package::DomainPackage`] is the
//! `model-effective-declaration.schema.json`/`model-complete.md` domain
//! package shape FR-154 defines; every other rung in this module is a pure
//! function of that value. [`intake`] admits Semantic IR 2.0.0 document
//! bytes against a caller's selection (FR-154's four-check table) and lifts
//! them from a spec bundle in the first place; reading an admitted
//! document's IR nodes into [`domain_package::DomainPackageRecord`]s is not
//! wired yet, so every [`domain_package::DomainPackage`] in this tree is
//! still caller-constructed.

pub mod accounting;
pub mod checked_dispatch;
pub mod conformance;
pub mod dispatch;
pub mod domain_package;
pub mod intake;
pub mod key;
pub mod normalize;
pub mod population;
pub mod refusal;
pub mod systems;
