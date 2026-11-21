// SPDX-License-Identifier: AGPL-3.0-or-later
//! Retained authenticated dependency artifacts and their own witnesses (FR-850).

use std::sync::Arc;
use filament_contract_model::CheckedPackageV2;
use crate::{ReadEqualityRows, ReadIdentityRows};

/// One admitted library, shared by all import paths through the closure.
#[derive(Debug)]
pub struct DependencyArtifact {
    /// Selected library identity.
    pub identity: String,
    /// The package admitted with its direct dependencies.
    pub package: Arc<CheckedPackageV2>,
    /// Exact original package bytes, retained for replay.
    pub bytes: Vec<u8>,
    /// Original identity witness bytes.
    pub identity_bytes: Vec<u8>,
    /// Original equality witness bytes, including an empty witness.
    pub equality_bytes: Vec<u8>,
    /// Identity rows joined against this library.
    pub identities: ReadIdentityRows,
    /// Equality rows joined against this library.
    pub equality: ReadEqualityRows,
}
