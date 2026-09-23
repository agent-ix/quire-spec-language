// SPDX-License-Identifier: AGPL-3.0-or-later
//! `complete::package` checks that need no parsed source. The tests that parse
//! a source and then resolve it are integration tests
//! (`tests/it/complete_package.rs`), since parsing is the caller's step
//! (QSL-181).
use std::collections::BTreeSet;

use crate::complete::{CapabilityId, Definition, DefinitionRole, PackageError, ReaderAuthority};
use ix_trace_rs::trace;

const READER_AUTHORITY: ReaderAuthority = ReaderAuthority::fixture();

#[trace("TC-180", "FR-131-AC-1", "FR-131-AC-2")]
#[test]
fn capability_inventory_round_trips_through_one_shared_family_authority() {
    let inventory = CapabilityId::complete_inventory();
    assert_eq!(inventory.len(), 176);
    for capability in &inventory {
        assert_eq!(
            CapabilityId::complete(capability.as_str()).unwrap(),
            *capability
        );
    }
    for outside in [
        "V1-SRC-000",
        "V1-SRC-016",
        "V1-TYPE-032",
        "V1-TOOL-011",
        "V1-OTHER-001",
    ] {
        assert!(matches!(
            CapabilityId::complete(outside),
            Err(PackageError::UnknownCapability(value)) if value == outside
        ));
    }
}

#[trace("TC-180", "FR-131-AC-2")]
#[test]
fn definition_digest_is_the_exact_artifact_byte_digest() {
    let bytes = b"exact definition bytes\n";
    let definition = Definition::from_exact_bytes(
        &READER_AUTHORITY,
        "fixed",
        "1",
        DefinitionRole::Runtime,
        BTreeSet::new(),
        BTreeSet::new(),
        bytes,
    )
    .unwrap();
    assert_eq!(definition.exact_bytes(), bytes);
    assert_eq!(
        definition.exact().digest().digest().to_string(),
        "sha256:8a942381b82e9165c44d9b427a128d53d1241c2353eb9fbb9bfe0c445b10ce72"
    );
}
