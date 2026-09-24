// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-056/FR-036: a domain package whose declarations the composed model
//! linker may bind native references to.
//!
//! [`AdmittedPackage::admit`] takes the records intake read
//! ([`crate::model::intake::read_records`]) under their admitted selection,
//! applies FR-152's kind mapping ([`crate::model::systems::classify`]) and
//! indexes every declaration by its FR-154 identity form: a type-definition
//! node by its artifact id (`ix://<package>/<artifact id>`), a member node by
//! its owner's key and member name (`<owner identity>/<name>`). Any
//! declaration refusal leaves the whole package unadmitted (FR-056), so a
//! value of this type always carries a complete, classified declaration set.
//!
//! [`DeclarationKind`] is decided by the record's own shape and its FR-152
//! classification, never by a kind name or module.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::accounting::{Incomplete, Meter};
use crate::model::domain_package::{DomainPackage, DomainPackageRecord, DomainPackageRef};
use crate::model::intake::{member_identity_name, type_identity_segment};
use crate::model::key::DeclarationKey;
use crate::model::normalize::{ModelRefusal, ModelRefusalCause};
use crate::model::systems::{classify, Kind, SystemsClassification};
use qsl_foundation::diagnostic::Code;

/// What one admitted declaration is: its record shape, refined by FR-152's
/// kind mapping for the systems records.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DeclarationKind {
    /// An object type with no `interfaceFeatures`.
    ObjectType,
    /// FR-152's Interface: an object type with `interfaceFeatures`.
    Interface,
    /// A value type bound to a native value type.
    ValueType,
    /// A field member of an object type.
    Field,
    /// An operation member of an object type.
    Operation,
    /// FR-152's Part.
    Part,
    /// FR-152's Port.
    Port,
    /// FR-152's Connection: a relationship whose two ends are Ports.
    Connection,
    /// A navigation relationship: both ends name object types.
    Relationship,
    /// FR-152's Allocation.
    Allocation,
    /// An FR-153 population declaration.
    Population,
}

/// Why a domain package could not be admitted for linking.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdmitFailure {
    /// The FR-152 kind mapping ran out of its `systems.kind` budget.
    Incomplete(Incomplete),
    /// Declaration refusals, in FR-152 kind-mapping order after any
    /// identity refusal; the package admits no declaration.
    Refused(Vec<ModelRefusal>),
}

/// One admitted declaration: its key, its record and its kind.
#[derive(Clone, Copy, Debug)]
pub struct Declaration<'a> {
    /// The record's own declaration key.
    pub key: &'a DeclarationKey,
    /// The record intake read for this IR node.
    pub record: &'a DomainPackageRecord,
    /// The declaration's kind.
    pub kind: DeclarationKind,
}

/// A classified domain package, indexed by FR-154 identity form.
#[derive(Clone, Debug)]
pub struct AdmittedPackage {
    package: DomainPackage,
    systems: SystemsClassification,
    /// Artifact id of each type-definition or population node -> record index.
    types: BTreeMap<String, usize>,
    /// (owner node identity, member name) -> record index.
    members: BTreeMap<(String, String), usize>,
}

fn identity_refusal(key: &DeclarationKey, detail: String) -> ModelRefusal {
    ModelRefusal {
        code: Code::InvalidModelBinding,
        cause: ModelRefusalCause::IntakeMalformedDeclaration {
            node: key.node.clone(),
            artifact: None,
            span: None,
        },
        detail,
    }
}

impl AdmittedPackage {
    /// Classifies `package` under FR-152 and indexes its declarations.
    ///
    /// Refuses when two records share one key (`conflicting-binding`), when
    /// a record's key names another package or has neither FR-154 identity
    /// form (`malformed-declaration`), or when the kind mapping reports any
    /// refusal. All refusals are reported; none of the package is admitted.
    pub fn admit(package: DomainPackage, meter: &mut Meter) -> Result<Self, AdmitFailure> {
        let identity = package.model_selection.identity.clone();
        let mut refusals = Vec::new();
        let mut types = BTreeMap::new();
        let mut members = BTreeMap::new();
        let mut seen = BTreeSet::new();
        for (index, record) in package.records.iter().enumerate() {
            let key = record.key();
            if !seen.insert(key) {
                refusals.push(ModelRefusal {
                    code: Code::InvalidModelBinding,
                    cause: ModelRefusalCause::ConflictingBinding { key: key.clone() },
                    detail: format!("{} is declared more than once", key.node),
                });
                continue;
            }
            if key.package != identity {
                refusals.push(identity_refusal(
                    key,
                    format!(
                        "{} is keyed under domain package {:?}, not the selected {identity:?}",
                        key.node, key.package
                    ),
                ));
                continue;
            }
            if let Some(artifact) = type_identity_segment(&identity, &key.node) {
                types.insert(artifact.to_owned(), index);
                continue;
            }
            let member = key.node.rsplit_once('/').and_then(|(owner, _)| {
                member_identity_name(owner, &key.node).map(|name| (owner.to_owned(), name))
            });
            match member {
                Some((owner, name)) => {
                    members.insert((owner, name.to_owned()), index);
                }
                None => refusals.push(identity_refusal(
                    key,
                    format!(
                        "{} is neither ix://{identity}/<artifact id> nor <owner>/<name>",
                        key.node
                    ),
                )),
            }
        }
        let systems = classify(&package, meter).map_err(AdmitFailure::Incomplete)?;
        refusals.extend(systems.refusals.iter().cloned());
        if !refusals.is_empty() {
            return Err(AdmitFailure::Refused(refusals));
        }
        Ok(Self {
            package,
            systems,
            types,
            members,
        })
    }

    /// The selection this package was admitted under.
    pub fn selection(&self) -> &DomainPackageRef {
        &self.package.model_selection
    }

    /// The admitted package itself.
    pub fn package(&self) -> &DomainPackage {
        &self.package
    }

    /// Number of admitted declarations.
    pub fn len(&self) -> usize {
        self.package.records.len()
    }

    /// Whether the package declares nothing.
    pub fn is_empty(&self) -> bool {
        self.package.records.is_empty()
    }

    /// The type-definition or population node named by `artifact_id`
    /// (`ix://<package>/<artifact_id>`), if the package declares one.
    pub fn declaration(&self, artifact_id: &str) -> Option<Declaration<'_>> {
        self.types.get(artifact_id).map(|index| self.at(*index))
    }

    /// The member `name` of the declaration keyed `owner`, if the package
    /// declares one.
    pub fn member(&self, owner: &DeclarationKey, name: &str) -> Option<Declaration<'_>> {
        if owner.package != self.package.model_selection.identity {
            return None;
        }
        self.members
            .get(&(owner.node.clone(), name.to_owned()))
            .map(|index| self.at(*index))
    }

    fn at(&self, index: usize) -> Declaration<'_> {
        let record = &self.package.records[index];
        Declaration {
            key: record.key(),
            record,
            kind: self.kind(record),
        }
    }

    fn kind(&self, record: &DomainPackageRecord) -> DeclarationKind {
        match record {
            DomainPackageRecord::ObjectType(object) if object.interface_features.is_some() => {
                DeclarationKind::Interface
            }
            DomainPackageRecord::ObjectType(_) => DeclarationKind::ObjectType,
            DomainPackageRecord::FieldMember(_) => DeclarationKind::Field,
            DomainPackageRecord::ScalarType(_) => DeclarationKind::ValueType,
            DomainPackageRecord::OperationMember(_) => DeclarationKind::Operation,
            // `admit` refuses every kind-mapping refusal, so an admitted
            // component is a Part and an admitted endpoint a Port.
            DomainPackageRecord::Component(_) => DeclarationKind::Part,
            DomainPackageRecord::Endpoint(_) => DeclarationKind::Port,
            DomainPackageRecord::Relationship(relationship) => {
                // A relationship key classifies only as a Connection or as
                // no kind (navigation); keys are unique, so no other kind's
                // record shares it.
                match self.systems.actual_kind(&relationship.key) {
                    Kind::Connection => DeclarationKind::Connection,
                    Kind::None | Kind::Part | Kind::Port | Kind::Interface | Kind::Allocation => {
                        DeclarationKind::Relationship
                    }
                }
            }
            DomainPackageRecord::Allocation(_) => DeclarationKind::Allocation,
            DomainPackageRecord::Population(_) => DeclarationKind::Population,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::accounting::ModelNormalizationLimits;
    use crate::model::domain_package::{
        FieldMemberRecord, Multiplicity, NativeValueType, ObjectTypeRecord, ValueTypeRef,
    };
    use ix_trace_rs::trace;

    fn key(node: &str) -> DeclarationKey {
        DeclarationKey::fixture(format!("ix://test/orders/{node}"))
    }

    fn order(key: DeclarationKey) -> DomainPackageRecord {
        DomainPackageRecord::ObjectType(ObjectTypeRecord {
            key,
            interface_features: None,
            abstract_type: false,
            supertypes: Vec::new(),
        })
    }

    fn field(key: DeclarationKey, owner: DeclarationKey) -> DomainPackageRecord {
        DomainPackageRecord::FieldMember(FieldMemberRecord {
            key,
            owner,
            value_type: ValueTypeRef::Native(NativeValueType::Boolean),
            multiplicity: Multiplicity {
                lower: 1,
                upper: Some(1),
                ordered: false,
                unique: false,
            },
            subsets: Vec::new(),
            redefines: None,
        })
    }

    fn admit(records: Vec<DomainPackageRecord>) -> Result<AdmittedPackage, AdmitFailure> {
        AdmittedPackage::admit(
            DomainPackage::new(DomainPackageRef::fixture("n01"), records),
            &mut Meter::new(ModelNormalizationLimits::default()),
        )
    }

    /// A type node is found by its artifact id and a member by its owner's
    /// key and name; neither is found by the other's form.
    #[trace("TC-148", "FR-036-AC-9")]
    #[test]
    fn indexes_type_nodes_by_artifact_id_and_members_by_owner() {
        let package = admit(vec![
            order(key("Order")),
            field(key("Order/paid"), key("Order")),
        ])
        .expect("admits");
        let order = package.declaration("Order").expect("type node");
        assert_eq!(
            (order.key, order.kind),
            (&key("Order"), DeclarationKind::ObjectType)
        );
        let paid = package.member(&key("Order"), "paid").expect("member");
        assert_eq!(
            (paid.key, paid.kind),
            (&key("Order/paid"), DeclarationKind::Field)
        );
        assert!(package.declaration("paid").is_none());
        assert!(package.member(&key("Order"), "Order").is_none());
        let foreign_owner = DeclarationKey {
            package: "test/other".to_owned(),
            node: key("Order").node,
        };
        assert!(package.member(&foreign_owner, "paid").is_none());
    }

    /// A duplicate key, a record keyed under another package and a node in
    /// neither identity form each refuse, all reported, none admitted.
    #[trace("TC-148", "FR-036-AC-9")]
    #[test]
    fn refuses_duplicate_foreign_and_malformed_keys_together() {
        let foreign = DeclarationKey {
            package: "test/other".to_owned(),
            node: "ix://test/orders/Shipment".to_owned(),
        };
        let Err(AdmitFailure::Refused(refusals)) = admit(vec![
            order(key("Order")),
            order(key("Order")),
            order(foreign),
            order(DeclarationKey::fixture("ix://test/orders/a/b-c")),
        ]) else {
            panic!("the package refuses")
        };
        let causes: Vec<&str> = refusals
            .iter()
            .map(|refusal| refusal.cause.as_str())
            .collect();
        assert_eq!(
            causes,
            [
                "conflicting-binding",
                "malformed-declaration",
                "malformed-declaration"
            ]
        );
        assert!(refusals[1].detail.contains("test/other"));
    }
}
