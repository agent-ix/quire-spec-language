// SPDX-License-Identifier: AGPL-3.0-or-later
//! `tests/fixtures/spine-model.native`, the unit that selects the domain
//! package [`DOCUMENT`] by the identity the document declares.

/// The domain package document the unit's `model M` selects.
pub(crate) const DOCUMENT: &[u8] = include_bytes!("../../../tests/fixtures/spine-model.semantic-ir.json");

/// The spine-model unit.
pub(crate) fn unit() -> String {
    include_str!("../../../tests/fixtures/spine-model.native").to_owned()
}
