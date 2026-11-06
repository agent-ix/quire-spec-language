// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-533 "What the bare view reads from the package": the IEEE profile flag
//! of a bytes-route world is the lock's selection of the profile law.

use super::*;
use crate::{BarePackage, BareSupply};

/// Whether the bytes-route world of `package` holds the IEEE profile, and
/// whether its lock selects the profile law.
fn ieee_flags(package: &CheckedPackage) -> (bool, bool) {
    let emission = emit(package);
    let Read::Verified { admitted, .. } = read_back(&emission) else {
        panic!("the emitted package reads back");
    };
    let rows = emission
        .identity_rows(
            &admitted,
            &mut quire_contract_model::NativeJoinBudget::new(10_000_000, 10_000_000),
        )
        .expect("final identity rows");
    let world = BarePackage::new(&admitted)
        .with_supply(
            BareSupply {
                identities: &rows,
                equality: None,
            },
            &mut crate::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the fixture budget admits the supply")
        .world()
        .expect("the world reads");
    let ieee = DefinitionRole::IeeeProfile.reference();
    let ieee = json!({"authority": ieee.authority, "identity": ieee.identity});
    let selected = wire(&emission)["lock"]["definition_selections"]
        .as_array()
        .unwrap()
        .contains(&ieee);
    (world.has_ieee_profile(), selected)
}

/// FR-533 "What the bare view reads": a package whose lock selects the IEEE profile law has the
/// profile in its bytes-route world, and one whose lock does not has none, so
/// a float conversion on the bytes route is not refused as an unresolved
/// profile.
#[trace("TC-1333")]
#[test]
fn the_bare_world_holds_the_ieee_profile_exactly_when_the_lock_selects_it() {
    assert_eq!(
        ieee_flags(&golden::float_add_of(BuiltinType::Float64, "nearest-even")),
        (true, true)
    );
    assert_eq!(ieee_flags(&recursive_f()), (false, false));
}
