// SPDX-License-Identifier: AGPL-3.0-or-later
//! `tests/fixtures/spine-model.native`, the unit that selects the domain
//! package [`DOCUMENT`] by the identity the document declares.

/// The domain package document the unit's `model M` selects.
pub const DOCUMENT: &[u8] = include_bytes!("../fixtures/spine-model.semantic-ir.json");

/// The spine-model unit.
pub fn unit() -> String {
    include_str!("../fixtures/spine-model.native").to_owned()
}
