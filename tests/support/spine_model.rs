// SPDX-License-Identifier: AGPL-3.0-or-later
//! `tests/fixtures/spine-model.native` with its `model M` digest filled in.
//!
//! The fixture names the domain package it selects as
//! `sha256-jcs:{model_digest}`; [`unit`] replaces the token with the
//! `sha256-jcs` digest of [`DOCUMENT`], computed when the test runs, so the
//! fixture never records a frozen digest of its sibling document.

/// The domain package document the unit's `model M` selects.
pub const DOCUMENT: &[u8] = include_bytes!("../fixtures/spine-model.semantic-ir.json");

/// The spine-model unit, selecting [`DOCUMENT`] by its digest.
pub fn unit() -> String {
    let packages = qsl_semantics::model::intake::package_input([DOCUMENT]);
    unit_selecting(packages.keys().next().expect("one document"))
}

/// The spine-model unit, selecting the document whose `sha256-jcs` digest
/// is `digest`.
pub fn unit_selecting(digest: &[u8; 32]) -> String {
    include_str!("../fixtures/spine-model.native")
        .replace("{model_digest}", &qsl_semantics::model::key::hex(digest))
}
