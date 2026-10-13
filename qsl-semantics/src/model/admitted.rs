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
use crate::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, OriginalDeclaration,
};
use crate::model::intake::{member_identity_name, type_identity_segment};
use crate::model::key::DeclarationKey;
use crate::model::normalize::{MemberPath, MemberRoot, ModelRefusal, ModelRefusalCause};
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
    /// A record value type: named fields and no identity (FR-208).
    RecordValueType,
    /// A variant type: a closed set of named cases (FR-208).
    VariantType,
    /// One case of a variant type.
    VariantCase,
    /// An event type with a Timestamp occurrence field (FR-208).
    EventType,
    /// A state machine with states and transitions (FR-208).
    StateMachine,
    /// A process type with an object universe (FR-208).
    Process,
    /// An authored clause member.
    Clause,
    /// An original namespace.
    Namespace,
    /// An original persistence interface.
    PersistenceInterface,
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
    /// Authored intake provenance; absent only for source-less synthetic records.
    pub original: Option<&'a OriginalDeclaration>,
}

/// A classified domain package, indexed by FR-154 identity form.
#[derive(Clone, Debug)]
pub struct AdmittedPackage {
    package: DomainPackage,
    systems: SystemsClassification,
    /// Artifact id of each type-definition or population node -> record index.
    nodes: BTreeMap<String, usize>,
    /// Owner node identity -> member name -> record index.
    members: BTreeMap<String, BTreeMap<String, usize>>,
}

/// The declaration's own `identity` member. A field, operation, clause or
/// relationship is a member of its declaring type, at its position in that
/// type's array (records keep the reader's order); a population is its own
/// declaration; every other record is a type definition.
fn identity_member(records: &[DomainPackageRecord], record: &DomainPackageRecord) -> MemberPath {
    // Position among the earlier records of the same kind and owner.
    let position = |same: &dyn Fn(&DomainPackageRecord) -> bool| {
        records
            .iter()
            .take_while(|other| !std::ptr::eq(*other, record))
            .filter(|other| same(other))
            .count()
    };
    let member = |array: &str, index: usize| {
        MemberPath::type_definition()
            .name(array)
            .index(index)
            .name("identity")
    };
    match record {
        DomainPackageRecord::FieldMember(field) => member(
            "fields",
            position(
                &|other| matches!(other, DomainPackageRecord::FieldMember(o) if o.owner == field.owner),
            ),
        ),
        DomainPackageRecord::OperationMember(operation) => member(
            "operations",
            position(
                &|other| matches!(other, DomainPackageRecord::OperationMember(o) if o.owner == operation.owner),
            ),
        ),
        // A clause owned by an operation is named at the operation, as
        // normalize names it: the record does not say pre or post.
        DomainPackageRecord::Clause(clause)
            if records.iter().any(|other| {
                matches!(other, DomainPackageRecord::OperationMember(o) if o.key == clause.owner)
            }) =>
        {
            MemberPath::new(MemberRoot::Operation)
        }
        DomainPackageRecord::Clause(clause) => member(
            "clauses",
            position(
                &|other| matches!(other, DomainPackageRecord::Clause(o) if o.owner == clause.owner),
            ),
        ),
        DomainPackageRecord::Relationship(relationship) => match &relationship.declared_at {
            Some(slot) => member("relationships", slot.index),
            // A relationship without a slot is a systems connection, a
            // top-level type node.
            None => MemberPath::type_definition().name("identity"),
        },
        DomainPackageRecord::Population(_) => {
            MemberPath::new(MemberRoot::PopulationDefinition).name("identity")
        }
        _ => MemberPath::type_definition().name("identity"),
    }
}

fn identity_refusal(
    package: &DomainPackage,
    record: &DomainPackageRecord,
    detail: String,
) -> ModelRefusal {
    ModelRefusal {
        code: Code::InvalidModelBinding,
        cause: crate::model::normalize::malformed_at(
            package,
            record.key(),
            identity_member(&package.records, record),
        ),
        detail,
    }
}

impl AdmittedPackage {
    /// Classifies `package` under FR-152 and indexes its declarations.
    ///
    /// Refuses when two records share one key (`conflicting-binding`), when
    /// a record's key names another package or has neither FR-154 identity
    /// form (`malformed-declaration`), when a member's owner is not a type
    /// node of the package (`unknown-owner`), or when the kind mapping
    /// reports any refusal. All refusals are reported; none of the package is
    /// admitted.
    pub fn admit(package: DomainPackage, meter: &mut Meter) -> Result<Self, AdmitFailure> {
        let identity = package.model_selection.identity.clone();
        let mut refusals = Vec::new();
        let mut nodes = BTreeMap::new();
        let mut member_records = Vec::new();
        let mut seen = BTreeSet::new();
        for (index, record) in package.records.iter().enumerate() {
            let key = record.key();
            if !seen.insert(key) {
                refusals.push(ModelRefusal {
                    code: Code::InvalidModelBinding,
                    cause: ModelRefusalCause::ConflictingBinding {
                        key: key.clone(),
                        member: None,
                    },
                    detail: format!("{} is declared more than once", key.node),
                });
            } else if key.package != identity {
                refusals.push(identity_refusal(
                    &package,
                    record,
                    format!(
                        "{} is keyed under domain package {:?}, not the selected {identity:?}",
                        key.node, key.package
                    ),
                ));
            } else if let Some(artifact) = type_identity_segment(&identity, &key.node) {
                nodes.insert(artifact.to_owned(), index);
            } else {
                member_records.push(index);
            }
        }
        // Members index under their owner once every type node is known. A
        // field or operation names its owner; any other member-form record
        // (an inline relationship) is owned by its identity's prefix. Either
        // way the owner must be an indexed type node of this package.
        let owners: BTreeSet<&str> = nodes
            .values()
            .map(|index: &usize| package.records[*index].key().node.as_str())
            .collect();
        let mut members: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
        for index in member_records {
            let record = &package.records[index];
            let key = record.key();
            let owner_and_name = match record {
                DomainPackageRecord::FieldMember(member) => {
                    member_identity_name(&member.owner.node, &key.node)
                        .map(|name| (member.owner.node.as_str(), name))
                }
                DomainPackageRecord::OperationMember(member) => {
                    member_identity_name(&member.owner.node, &key.node)
                        .map(|name| (member.owner.node.as_str(), name))
                }
                DomainPackageRecord::Relationship(relationship) => relationship
                    .source
                    .role
                    .as_deref()
                    .map(|name| (relationship.source.type_identity.node.as_str(), name)),
                DomainPackageRecord::Clause(member) => {
                    member_identity_name(&member.owner.node, &key.node)
                        .map(|name| (member.owner.node.as_str(), name))
                }
                // A case is a member of its variant type, named by its
                // declared case name; its own node identity is the
                // `variant` slot, not `<owner>/<name>`.
                DomainPackageRecord::Variant(case) => {
                    Some((case.owner.node.as_str(), case.case.as_str()))
                }
                DomainPackageRecord::Enum(_)
                | DomainPackageRecord::ObjectType(_)
                | DomainPackageRecord::RecordValueType(_)
                | DomainPackageRecord::EventType(_)
                | DomainPackageRecord::StateMachine(_)
                | DomainPackageRecord::Process(_)
                | DomainPackageRecord::Namespace(_)
                | DomainPackageRecord::PersistenceInterface(_)
                | DomainPackageRecord::ScalarType(_)
                | DomainPackageRecord::Component(_)
                | DomainPackageRecord::Endpoint(_)
                | DomainPackageRecord::Allocation(_)
                | DomainPackageRecord::Population(_) => {
                    key.node.rsplit_once('/').and_then(|(owner, _)| {
                        member_identity_name(owner, &key.node).map(|name| (owner, name))
                    })
                }
            };
            let Some((owner, name)) = owner_and_name else {
                refusals.push(identity_refusal(
                    &package,
                    record,
                    format!(
                        "{} is neither ix://{identity}/<artifact id> nor <owner>/<name>",
                        key.node
                    ),
                ));
                continue;
            };
            if !owners.contains(owner) {
                refusals.push(ModelRefusal {
                    code: Code::DanglingReference,
                    cause: ModelRefusalCause::UnknownOwner {
                        member: key.clone(),
                        owner: DeclarationKey {
                            package: identity.clone(),
                            node: owner.to_owned(),
                        },
                    },
                    detail: format!(
                        "member {} names owner {owner}, which is not a declaration of this package",
                        key.node
                    ),
                });
                continue;
            }
            members
                .entry(owner.to_owned())
                .or_default()
                .insert(name.to_owned(), index);
        }
        let systems = classify(&package, meter).map_err(AdmitFailure::Incomplete)?;
        refusals.extend(systems.refusals.iter().cloned());
        if !refusals.is_empty() {
            return Err(AdmitFailure::Refused(refusals));
        }
        Ok(Self {
            package,
            systems,
            nodes,
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
        self.nodes.get(artifact_id).map(|index| self.at(*index))
    }

    /// The type-definition or population node keyed `key`, if the package
    /// declares one: `key` must name this package and have the
    /// `ix://<package>/<artifact id>` form.
    pub fn declaration_by_key(&self, key: &DeclarationKey) -> Option<Declaration<'_>> {
        let identity = &self.package.model_selection.identity;
        if &key.package != identity {
            return None;
        }
        self.declaration(type_identity_segment(identity, &key.node)?)
    }

    /// The member `name` of the declaration keyed `owner`, if the package
    /// declares one.
    pub fn member(&self, owner: &DeclarationKey, name: &str) -> Option<Declaration<'_>> {
        if owner.package != self.package.model_selection.identity {
            return None;
        }
        self.members
            .get(owner.node.as_str())
            .and_then(|members| members.get(name))
            .map(|index| self.at(*index))
    }

    /// Every admitted declaration, in the package's record order.
    pub fn declarations(&self) -> impl Iterator<Item = Declaration<'_>> {
        (0..self.package.records.len()).map(|index| self.at(index))
    }

    fn at(&self, index: usize) -> Declaration<'_> {
        let record = &self.package.records[index];
        Declaration {
            key: record.key(),
            record,
            kind: self.kind(record),
            original: self.package.original(record.key()),
        }
    }

    fn kind(&self, record: &DomainPackageRecord) -> DeclarationKind {
        match record {
            DomainPackageRecord::ObjectType(object) if object.interface_features.is_some() => {
                DeclarationKind::Interface
            }
            DomainPackageRecord::ObjectType(_) => DeclarationKind::ObjectType,
            DomainPackageRecord::FieldMember(_) => DeclarationKind::Field,
            DomainPackageRecord::RecordValueType(_) => DeclarationKind::RecordValueType,
            DomainPackageRecord::Enum(_) => DeclarationKind::VariantType,
            DomainPackageRecord::Variant(_) => DeclarationKind::VariantCase,
            DomainPackageRecord::EventType(_) => DeclarationKind::EventType,
            DomainPackageRecord::StateMachine(_) => DeclarationKind::StateMachine,
            DomainPackageRecord::Process(_) => DeclarationKind::Process,
            DomainPackageRecord::Clause(_) => DeclarationKind::Clause,
            DomainPackageRecord::Namespace(_) => DeclarationKind::Namespace,
            DomainPackageRecord::PersistenceInterface(_) => DeclarationKind::PersistenceInterface,
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
            presence: quire_exact::Presence::Required,
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
    #[trace("TC-145", "FR-056-AC-5")]
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

    /// A malformed member key is reported at the member's own `identity`
    /// slot in its declaring type's array, not at the owner's identity.
    #[test]
    fn a_malformed_member_key_names_its_own_identity_member() {
        use crate::model::domain_package::{
            ClauseRecord, DeclaredSlot, RelationshipDirection, RelationshipEnd, RelationshipRecord,
        };
        let multiplicity = Multiplicity {
            lower: 0,
            upper: None,
            ordered: false,
            unique: false,
        };
        let end = |role: Option<&str>| RelationshipEnd {
            type_identity: key("Order"),
            role: role.map(str::to_owned),
            multiplicity,
        };
        let Err(AdmitFailure::Refused(refusals)) = admit(vec![
            order(key("Order")),
            field(key("Order/paid"), key("Order")),
            field(key("Order-first"), key("Order")),
            field(key("Order-second"), key("Order")),
            DomainPackageRecord::Clause(ClauseRecord {
                key: key("Order-clause"),
                owner: key("Order"),
                language: "quire".to_owned(),
                text: "true".to_owned(),
            }),
            DomainPackageRecord::Relationship(RelationshipRecord {
                key: key("relationship/bad"),
                declared_at: Some(DeclaredSlot {
                    owner: key("Order"),
                    index: 3,
                }),
                source: end(None),
                target: end(None),
                direction: RelationshipDirection::SourceToTarget,
            }),
        ]) else {
            panic!("the package refuses")
        };
        let members: Vec<(String, String)> = refusals
            .iter()
            .map(|refusal| {
                let ModelRefusalCause::IntakeMalformedDeclaration { node, member, .. } =
                    &refusal.cause
                else {
                    panic!("expected a malformed declaration, got {refusal:?}")
                };
                (node.clone(), member.to_string())
            })
            .collect();
        assert_eq!(
            members,
            [
                (
                    key("Order-first").node,
                    "TypeDefinition.fields[1].identity".to_owned()
                ),
                (
                    key("Order-second").node,
                    "TypeDefinition.fields[2].identity".to_owned()
                ),
                (
                    key("Order-clause").node,
                    "TypeDefinition.clauses[0].identity".to_owned()
                ),
                (
                    key("relationship/bad").node,
                    "TypeDefinition.relationships[3].identity".to_owned()
                ),
            ]
        );
    }

    /// A record with no array slot is named at the node the refusal is
    /// about: a systems connection at its type node and an operation-owned
    /// clause at its operation (as normalize names it).
    #[test]
    fn slotless_records_name_the_node_they_are_about() {
        use crate::model::domain_package::{
            ClauseRecord, OperationEffect, OperationMemberRecord, RelationshipDirection,
            RelationshipEnd, RelationshipRecord,
        };
        let multiplicity = Multiplicity {
            lower: 0,
            upper: None,
            ordered: false,
            unique: false,
        };
        let end = |role: Option<&str>| RelationshipEnd {
            type_identity: key("Order"),
            role: role.map(str::to_owned),
            multiplicity,
        };
        let relationship = |name: &str, role: Option<&str>| {
            DomainPackageRecord::Relationship(RelationshipRecord {
                key: key(name),
                declared_at: None,
                source: end(role),
                target: end(None),
                direction: RelationshipDirection::SourceToTarget,
            })
        };
        let Err(AdmitFailure::Refused(refusals)) = admit(vec![
            order(key("Order")),
            DomainPackageRecord::OperationMember(OperationMemberRecord {
                key: key("Order/open"),
                owner: key("Order"),
                parameters: Vec::new(),
                result: None,
                requires: Vec::new(),
                ensures: Vec::new(),
                effect: OperationEffect::default(),
                has_body: false,
                redefines: None,
            }),
            DomainPackageRecord::Clause(ClauseRecord {
                key: key("Order/open-clause"),
                owner: key("Order/open"),
                language: "quire".to_owned(),
                text: "true".to_owned(),
            }),
            relationship("connection/bad", None),
        ]) else {
            panic!("the package refuses")
        };
        let members: Vec<(String, String)> = refusals
            .iter()
            .map(|refusal| {
                let ModelRefusalCause::IntakeMalformedDeclaration { node, member, .. } =
                    &refusal.cause
                else {
                    panic!("expected a malformed declaration, got {refusal:?}")
                };
                (node.clone(), member.to_string())
            })
            .collect();
        assert_eq!(
            members,
            [
                (key("Order/open-clause").node, "Operation".to_owned()),
                (
                    key("connection/bad").node,
                    "TypeDefinition.identity".to_owned()
                ),
            ]
        );
    }

    /// A member is indexed under the owner its record names, which must be a
    /// type node of the package: a field naming an undeclared owner refuses
    /// `unknown-owner`, and a field whose key is not `<named owner>/<name>`
    /// refuses `malformed-declaration`, even when the key's prefix is a
    /// declared type.
    #[trace("TC-145", "FR-056-AC-5")]
    #[test]
    fn members_index_under_their_named_owner_only() {
        let Err(AdmitFailure::Refused(refusals)) = admit(vec![
            order(key("Order")),
            order(key("Shipment")),
            field(key("Ghost/paid"), key("Ghost")),
            field(key("Shipment/paid"), key("Order")),
        ]) else {
            panic!("the package refuses")
        };
        let causes: Vec<&str> = refusals
            .iter()
            .map(|refusal| refusal.cause.as_str())
            .collect();
        assert_eq!(causes, ["unknown-owner", "malformed-declaration"]);
        assert_eq!(refusals[0].code, Code::DanglingReference);

        let package = admit(vec![
            order(key("Order")),
            field(key("Order/paid"), key("Order")),
        ])
        .expect("admits");
        assert!(package.member(&key("Order"), "paid").is_some());
    }

    /// An identity refusal keeps the origin the package retains for the
    /// record's key (FR-154). The reader keys every record under the
    /// selection's identity, so only a package assembled by hand reaches
    /// these refusals; the origin is placed on it to test the plumbing.
    #[test]
    fn an_identity_refusal_keeps_the_records_origin() {
        let foreign = DeclarationKey {
            package: "test/other".to_owned(),
            node: "ix://test/orders/Shipment".to_owned(),
        };
        let mut package = DomainPackage::new(
            DomainPackageRef::fixture("n01"),
            vec![order(foreign.clone())],
        );
        package.originals.insert(
            foreign,
            crate::model::domain_package::OriginalDeclaration {
                meaning: None,
                origin: serde_json::json!({"source": {
                    "sourceIdentity": "ix://test/orders/spec",
                    "path": "spec.qspec",
                    "startLine": 7,
                    "startColumn": 3,
                }}),
                member_name: None,
            },
        );
        let Err(AdmitFailure::Refused(refusals)) = AdmittedPackage::admit(
            package,
            &mut Meter::new(ModelNormalizationLimits::default()),
        ) else {
            panic!("the foreign key refuses")
        };
        assert_eq!(refusals.len(), 1, "{refusals:?}");
        let ModelRefusalCause::IntakeMalformedDeclaration { artifact, span, .. } =
            &refusals[0].cause
        else {
            panic!("expected a malformed declaration: {refusals:?}");
        };
        assert_eq!(artifact.as_deref(), Some("ix://test/orders/spec"));
        let span = span.expect("the retained position");
        assert_eq!((span.start.line, span.start.column), (7, 3));
    }
}
