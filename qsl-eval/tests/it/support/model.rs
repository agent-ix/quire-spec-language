// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-094: an admitted domain package declaring one object type, so `check`
//! can key a `Reference<T>` over that type's model node.

use qsl_semantics::check::AdmittedModel;
use qsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, ObjectTypeRecord,
};
use qsl_semantics::model::key::DeclarationKey;
use quire_exact::EffectiveId;

/// The domain package `package` declaring the object type `label`, under
/// the effective identity `object`.
pub fn object_model(package: &str, label: &str, object: EffectiveId) -> AdmittedModel {
    let declaration = DeclarationKey::fixture(label);
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture(package),
        vec![DomainPackageRecord::ObjectType(ObjectTypeRecord {
            key: declaration.clone(),
            interface_features: None,
            abstract_type: false,
            supertypes: Vec::new(),
        })],
    );
    AdmittedModel::fixture(&domain_package, [(object, declaration)])
}
