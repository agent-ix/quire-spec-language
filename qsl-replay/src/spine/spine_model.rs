// SPDX-License-Identifier: AGPL-3.0-or-later
//! Test support: `tests/fixtures/spine-model.native` with its `model M`
//! digest token `{model_digest}` filled in with the `sha256-jcs` digest of
//! `spine-model.semantic-ir.json`, computed when the test runs.

/// The domain package document the unit's `model M` selects.
pub(crate) const DOCUMENT: &[u8] =
    include_bytes!("../../../tests/fixtures/spine-model.semantic-ir.json");

/// The spine-model unit, selecting [`DOCUMENT`] by its digest.
pub(crate) fn unit() -> String {
    let packages = qsl_semantics::model::intake::package_input([DOCUMENT]);
    unit_selecting(packages.keys().next().expect("one document"))
}

/// The spine-model unit, selecting the document whose `sha256-jcs` digest
/// is `digest`.
pub(crate) fn unit_selecting(digest: &[u8; 32]) -> String {
    include_str!("../../../tests/fixtures/spine-model.native")
        .replace("{model_digest}", &qsl_semantics::model::key::hex(digest))
}
