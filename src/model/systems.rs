// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-152 systems-model binding: kind mapping
//! (`quire.model.systems.kind-mapping/v1`) and the connection/allocation
//! admission rules (`quire.model.systems.connection/v1`,
//! `quire.model.systems.allocation/v1`).
//!
//! Scope decisions, recorded rather than left implicit:
//!
//! - This module resolves a systems-model element **by [`DeclarationKey`]
//!   directly**, not by FR-152's own `resolve(kind, name)` binder request
//!   over a qualified, aliased name (`M::Sys::pump`). Qualified-name
//!   resolution needs the profile/model-alias binder this crate's
//!   expression-grammar layer owns, not `crate::model` (the same "pure
//!   function of a caller-supplied value" boundary every other module in
//!   this crate keeps) — a caller that already has that binder can still
//!   use this module's kind mapping as the last step.
//! - **Navigation is entirely out of scope.** FR-152's "Navigation" section
//!   (`systems.resolve`, `model.navigate`, runtime traversal returning
//!   `Option<Reference<U>>`/`Set<Reference<U>>`/`Bag<Reference<U>>`,
//!   canonical reference-key ordering, `cardinality_out_of_bound`) needs an
//!   object population to traverse. Nothing in `crate::model` builds one
//!   yet (that is FR-153's own territory); this module resolves and admits
//!   *declarations*, never runtime references.
//! - This module does not compute an effective-declaration identity for any
//!   systems-model kind: `crate::model::normalize` does not fold
//!   components/endpoints/relationships into its `EffectiveView` (see its
//!   own module docs' exhaustive per-variant no-op arms for these three
//!   `DomainPackageRecord` variants). What this module resolves is the original
//!   producer identity and its kind only.
//! - Work-unit costs mirror the flat-one-per-charge choice
//!   [`crate::model::conformance`]/[`crate::model::dispatch`] already
//!   record, for the same reason: FR-152's own worked figures (e.g. Y04's
//!   `f(Flow)`/`f(Flow2)`) price `normalize`'s derivation-fact accounting,
//!   which this module would have to re-walk to reproduce exactly.
#![allow(
    clippy::large_enum_variant,
    reason = "cold refusal path; ModelRefusalCause carries DeclarationKeys inline"
)]
#![allow(
    clippy::result_large_err,
    reason = "cold refusal path; ModelRefusalCause carries DeclarationKeys inline, matching state::evaluation's typed-failure precedent"
)]

use std::collections::{HashMap, HashSet};

use crate::diagnostic::Code;
use crate::model::accounting::{Charge, ChargePoint, Incomplete, Meter};
use crate::model::conformance::{generals_by_specific, multiplicity_conforms, type_conforms};
use crate::model::domain_package::{
    AllocationRecord, ComponentRecord, DomainPackage, DomainPackageRecord, EndpointRecord,
    PortDirection, RelationshipRecord,
};
use crate::model::key::DeclarationKey;
use crate::model::normalize::{ModelRefusal, ModelRefusalCause};

/// FR-152's five disjoint systems-model kinds, plus `None` for a record
/// that resolved to no kind at all (missing a capability, cascaded from a
/// refused dependency, or — for a relationship whose two ends both name
/// object types — a navigation relationship, which is not an error).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Kind {
    /// A component with the `part-signature` capability.
    Part,
    /// An endpoint with `port-direction` whose owning component is a Part.
    Port,
    /// An object type export with `interfaceFeatures`.
    Interface,
    /// A relationship whose two ends are both Ports.
    Connection,
    /// An [`AllocationRecord`].
    Allocation,
    /// No kind: a missing capability, a cascaded dependency refusal, or a
    /// navigation-only relationship.
    None,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Part => "Part",
            Self::Port => "Port",
            Self::Interface => "Interface",
            Self::Connection => "Connection",
            Self::Allocation => "Allocation",
            Self::None => "none",
        }
    }
}

/// The full kind mapping over one [`DomainPackage`]: every component's, endpoint's
/// and relationship's resolved [`Kind`], plus every interface object type.
/// Every map here is keyed on the full [`DeclarationKey`], never the display
/// identity alone (PR #140 F2, PR #144 review): two records that share a
/// display identity but differ in revision must classify, and resolve,
/// independently rather than one silently overwriting the other.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SystemsClassification {
    component_kinds: HashMap<DeclarationKey, Kind>,
    endpoint_kinds: HashMap<DeclarationKey, Kind>,
    /// Both Connection and Allocation edges' resolved kinds, keyed
    /// together: the two producer records ([`RelationshipRecord`],
    /// [`AllocationRecord`]) are disjoint shapes with disjoint keys, so one
    /// lookup map over both is unambiguous and matches [`Self::actual_kind`]'s
    /// single-kind-per-key contract.
    edge_kinds: HashMap<DeclarationKey, Kind>,
    interface_types: HashSet<DeclarationKey>,
    components: HashMap<DeclarationKey, ComponentRecord>,
    endpoints: HashMap<DeclarationKey, EndpointRecord>,
    relationships: HashMap<DeclarationKey, RelationshipRecord>,
    allocations: HashMap<DeclarationKey, AllocationRecord>,
    object_types: HashSet<DeclarationKey>,
    /// Every kind-mapping cascade refusal, in emission order (ascending
    /// producer key within each of components, then endpoints, then
    /// relationships, then allocations).
    pub refusals: Vec<ModelRefusal>,
}

impl SystemsClassification {
    /// The resolved kind for any exact key this classification knows about
    /// (a component, endpoint, relationship or interface object type);
    /// [`Kind::None`] for anything else, including a dangling key.
    pub fn actual_kind(&self, key: &DeclarationKey) -> Kind {
        if self.interface_types.contains(key) {
            return Kind::Interface;
        }
        if let Some(kind) = self.component_kinds.get(key) {
            return *kind;
        }
        if let Some(kind) = self.endpoint_kinds.get(key) {
            return *kind;
        }
        if let Some(kind) = self.edge_kinds.get(key) {
            return *kind;
        }
        Kind::None
    }

    /// The full key this classification actually stored for `key`'s exact
    /// match under `kind`, fetched from the record store `kind` names
    /// (finding #7): never `key` itself echoed straight back, so a caller
    /// asserting the resolved key against the request key is asserting a
    /// real fact about what the domain package declared, not a tautology.
    fn stored_key(&self, kind: Kind, key: &DeclarationKey) -> Option<DeclarationKey> {
        match kind {
            Kind::Interface => self.interface_types.get(key).cloned(),
            Kind::Part => self.components.get(key).map(|record| record.key.clone()),
            Kind::Port => self.endpoints.get(key).map(|record| record.key.clone()),
            Kind::Connection => self.relationships.get(key).map(|record| record.key.clone()),
            Kind::Allocation => self.allocations.get(key).map(|record| record.key.clone()),
            Kind::None => None,
        }
    }
}

fn wrong_export(required: Kind, actual: Kind, item: &str) -> ModelRefusal {
    ModelRefusal {
        code: Code::InvalidModelBinding,
        cause: ModelRefusalCause::WrongExport,
        detail: format!(
            "{item}: required kind {}, actual kind {}",
            required.as_str(),
            actual.as_str()
        ),
    }
}

fn unsupplied(capability: &'static str, item: &str) -> ModelRefusal {
    ModelRefusal {
        code: Code::InvalidModelBinding,
        cause: ModelRefusalCause::UnsuppliedProducerRecord,
        detail: format!("{item} does not supply the {capability} capability"),
    }
}

/// An endpoint's owning component, or a relationship end's endpoint, names a
/// key absent from the domain package entirely (finding #6): distinct from a key
/// that IS declared but resolves to [`Kind::None`], which is a real
/// [`wrong_export`] refusal, not a dangling one.
fn dangling(cause: ModelRefusalCause, missing: &str, item: &str) -> ModelRefusal {
    ModelRefusal {
        code: Code::DanglingReference,
        cause,
        detail: format!("{item} names {missing}, which is not declared in this domain package"),
    }
}

fn charge_kind(meter: &mut Meter) -> Result<(), Incomplete> {
    meter.charge(Charge::new(ChargePoint::SystemsKind))
}

/// Runs `quire.model.systems.kind-mapping/v1` over every component, then
/// every endpoint, then every relationship in `domain_package`, each ascending by
/// producer key, charging `systems.kind` once per record before resolving
/// it. Exhaustive under the limits: every no-kind cascade is reported, in
/// this same order, never stopping at the first.
pub fn classify(
    domain_package: &DomainPackage,
    meter: &mut Meter,
) -> Result<SystemsClassification, Incomplete> {
    let mut components: Vec<&ComponentRecord> = Vec::new();
    let mut endpoints: Vec<&EndpointRecord> = Vec::new();
    let mut relationships: Vec<&RelationshipRecord> = Vec::new();
    let mut allocations: Vec<&AllocationRecord> = Vec::new();
    let mut object_types: HashSet<DeclarationKey> = HashSet::new();
    let mut interface_types: HashSet<DeclarationKey> = HashSet::new();

    for record in &domain_package.records {
        match record {
            DomainPackageRecord::Component(component) => components.push(component),
            DomainPackageRecord::Endpoint(endpoint) => endpoints.push(endpoint),
            DomainPackageRecord::Relationship(relationship) => relationships.push(relationship),
            DomainPackageRecord::Allocation(allocation) => allocations.push(allocation),
            DomainPackageRecord::ObjectType(object_type) => {
                object_types.insert(object_type.key.clone());
                if object_type.interface_features.is_some() {
                    interface_types.insert(object_type.key.clone());
                }
            }
            DomainPackageRecord::FieldMember(_)
            | DomainPackageRecord::ScalarType(_)
            | DomainPackageRecord::OperationMember(_)
            | DomainPackageRecord::Population(_) => {}
        }
    }
    components.sort_by_key(|component| component.key.clone());
    endpoints.sort_by_key(|endpoint| endpoint.key.clone());
    relationships.sort_by_key(|relationship| relationship.key.clone());
    allocations.sort_by_key(|allocation| allocation.key.clone());

    let mut component_kinds: HashMap<DeclarationKey, Kind> = HashMap::new();
    let mut endpoint_kinds: HashMap<DeclarationKey, Kind> = HashMap::new();
    let mut edge_kinds: HashMap<DeclarationKey, Kind> = HashMap::new();
    let mut refusals: Vec<ModelRefusal> = Vec::new();

    for component in &components {
        charge_kind(meter)?;
        let kind = if component.has_part_signature {
            Kind::Part
        } else {
            refusals.push(unsupplied("part-signature", &component.key.node));
            Kind::None
        };
        component_kinds.insert(component.key.clone(), kind);
    }

    for endpoint in &endpoints {
        charge_kind(meter)?;
        let kind = match &endpoint.direction {
            None => {
                refusals.push(unsupplied("port-direction", &endpoint.key.node));
                Kind::None
            }
            Some(_) => match component_kinds.get(&endpoint.owning_component) {
                None => {
                    refusals.push(dangling(
                        ModelRefusalCause::UnknownComponent {
                            item: endpoint.key.clone(),
                            missing: endpoint.owning_component.clone(),
                        },
                        &endpoint.owning_component.node,
                        &endpoint.key.node,
                    ));
                    Kind::None
                }
                Some(&Kind::Part) => Kind::Port,
                Some(&owner_kind) => {
                    refusals.push(wrong_export(Kind::Part, owner_kind, &endpoint.key.node));
                    Kind::None
                }
            },
        };
        endpoint_kinds.insert(endpoint.key.clone(), kind);
    }

    for relationship in &relationships {
        charge_kind(meter)?;
        let source_is_type = object_types.contains(&relationship.source.type_identity);
        let target_is_type = object_types.contains(&relationship.target.type_identity);
        let kind = if source_is_type && target_is_type {
            Kind::None // navigation relationship: no kind, not an error.
        } else {
            let mut ends_ok = true;
            for (end, label) in [
                (&relationship.source, "source"),
                (&relationship.target, "target"),
            ] {
                match endpoint_kinds.get(&end.type_identity) {
                    None => {
                        refusals.push(dangling(
                            ModelRefusalCause::UnknownEndpoint {
                                end: label,
                                relationship: relationship.key.clone(),
                                missing: end.type_identity.clone(),
                            },
                            &end.type_identity.node,
                            &format!("{} end of {}", label, relationship.key.node),
                        ));
                        ends_ok = false;
                    }
                    Some(&Kind::Port) => {}
                    Some(&end_kind) => {
                        refusals.push(wrong_export(
                            Kind::Port,
                            end_kind,
                            &format!("{} end of {}", label, relationship.key.node),
                        ));
                        ends_ok = false;
                    }
                }
            }
            if ends_ok {
                Kind::Connection
            } else {
                Kind::None
            }
        };
        edge_kinds.insert(relationship.key.clone(), kind);
    }

    for allocation in &allocations {
        charge_kind(meter)?;
        // `quire.model.systems.kind-mapping/v1`: an Allocation names a
        // source and target element and nothing else, so no capability or
        // end-resolution check applies at classification time. Actual
        // target-is-a-Part admission is `check_allocation`'s own job.
        edge_kinds.insert(allocation.key.clone(), Kind::Allocation);
    }

    Ok(SystemsClassification {
        component_kinds,
        endpoint_kinds,
        edge_kinds,
        interface_types,
        components: components
            .into_iter()
            .map(|c| (c.key.clone(), c.clone()))
            .collect(),
        endpoints: endpoints
            .into_iter()
            .map(|e| (e.key.clone(), e.clone()))
            .collect(),
        relationships: relationships
            .into_iter()
            .map(|r| (r.key.clone(), r.clone()))
            .collect(),
        allocations: allocations
            .into_iter()
            .map(|a| (a.key.clone(), a.clone()))
            .collect(),
        object_types,
        refusals,
    })
}

/// A binder request's result: `key` classified to exactly `required`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedElement {
    /// The producer key this classification actually stored for the
    /// matched record — fetched from that record, never the request's
    /// `key` echoed straight back (finding #7; see
    /// [`SystemsClassification::stored_key`]). This module resolves by
    /// key, not by qualified name — see the module docs.
    pub key: DeclarationKey,
    /// The resolved kind, always equal to the request's `required` kind.
    pub kind: Kind,
}

/// Resolves `key` as `required`, refusing `wrong-export` (a different real
/// kind) or `unsupplied-producer-record` (no kind at all) otherwise. Equal
/// names never substitute for a kind match — this module never inspects a
/// display name at all, only the full key and the classification.
pub fn resolve_kind(
    classification: &SystemsClassification,
    required: Kind,
    key: &DeclarationKey,
) -> Result<ResolvedElement, ModelRefusal> {
    let actual = classification.actual_kind(key);
    if actual == required {
        let resolved_key = classification
            .stored_key(actual, key)
            .unwrap_or_else(|| key.clone());
        Ok(ResolvedElement {
            key: resolved_key,
            kind: actual,
        })
    } else if actual == Kind::None {
        Err(unsupplied_or_wrong(required, key))
    } else {
        Err(wrong_export(required, actual, &key.node))
    }
}

fn unsupplied_or_wrong(required: Kind, key: &DeclarationKey) -> ModelRefusal {
    ModelRefusal {
        code: Code::InvalidModelBinding,
        cause: ModelRefusalCause::UnsuppliedProducerRecord,
        detail: format!(
            "{} does not resolve to the required kind {}",
            key.node,
            required.as_str()
        ),
    }
}

/// One failing connection condition: `{condition, code, cause, detail}`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConditionFailure {
    /// The condition name, e.g. `"port-direction"`, `"interface-type"`,
    /// `"multiplicity"`.
    pub condition: &'static str,
    /// The stable top-level code.
    pub code: Code,
    /// The FR-152-specific cause tag.
    pub cause: ModelRefusalCause,
    /// A human-readable detail naming the offending declarations.
    pub detail: String,
}

/// The substantive result of one connection admission check.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConnectionOutcome {
    /// Every condition held.
    Admitted,
    /// At least one condition failed; every failure, in table order.
    Refused(Vec<ConditionFailure>),
}

/// The outcome of one [`check_connection`] attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConnectionCheckOutcome {
    /// Every condition charge was admitted; see [`ConnectionOutcome`].
    Completed(ConnectionOutcome),
    /// A real defect (the relationship did not classify as a Connection, or
    /// an end's port has no known direction/multiplicity) refused the
    /// check outright.
    Refused(ModelRefusal),
    /// A `ModelNormalizationLimitsV1` counter was exhausted mid-check.
    Incomplete(Incomplete),
}

fn end_port<'a>(
    classification: &'a SystemsClassification,
    end_key: &DeclarationKey,
) -> Option<&'a EndpointRecord> {
    classification.endpoints.get(end_key)
}

/// Runs `quire.model.systems.connection/v1` over `relationship_key`, which
/// must already classify as [`Kind::Connection`]. Charges
/// `systems.connection-condition` for each of the three table conditions,
/// in table order, and reports every failure (never stop-at-first).
pub fn check_connection(
    domain_package: &DomainPackage,
    classification: &SystemsClassification,
    relationship_key: &DeclarationKey,
    meter: &mut Meter,
) -> ConnectionCheckOutcome {
    if classification.actual_kind(relationship_key) != Kind::Connection {
        return ConnectionCheckOutcome::Refused(ModelRefusal {
            code: Code::InvalidModelBinding,
            cause: ModelRefusalCause::WrongExport,
            detail: format!(
                "{} is not a Connection; the connection rule does not apply",
                relationship_key.node
            ),
        });
    }
    let Some(relationship) = classification.relationships.get(relationship_key) else {
        return ConnectionCheckOutcome::Refused(ModelRefusal {
            code: Code::DanglingReference,
            cause: ModelRefusalCause::UnknownRelationship {
                relationship: relationship_key.clone(),
            },
            detail: format!("{} is not a declared relationship", relationship_key.node),
        });
    };
    let Some(source_port) = end_port(classification, &relationship.source.type_identity) else {
        return ConnectionCheckOutcome::Refused(ModelRefusal {
            code: Code::DanglingReference,
            cause: ModelRefusalCause::UnknownSourcePort {
                port: relationship.source.type_identity.clone(),
            },
            detail: format!(
                "{} names an end that is not a declared endpoint",
                relationship.source.type_identity.node
            ),
        });
    };
    let Some(target_port) = end_port(classification, &relationship.target.type_identity) else {
        return ConnectionCheckOutcome::Refused(ModelRefusal {
            code: Code::DanglingReference,
            cause: ModelRefusalCause::UnknownTargetPort {
                port: relationship.target.type_identity.clone(),
            },
            detail: format!(
                "{} names an end that is not a declared endpoint",
                relationship.target.type_identity.node
            ),
        });
    };

    let mut failures = Vec::new();

    // Condition 1: port-direction.
    if let Err(incomplete) = meter.charge(Charge::new(ChargePoint::SystemsConnectionCondition)) {
        return ConnectionCheckOutcome::Incomplete(incomplete);
    }
    let (flow_source, flow_target, direction_ok) = match relationship.direction {
        crate::model::domain_package::RelationshipDirection::SourceToTarget => (
            source_port,
            target_port,
            matches!(
                source_port.direction,
                Some(PortDirection::Out | PortDirection::InOut)
            ) && matches!(
                target_port.direction,
                Some(PortDirection::In | PortDirection::InOut)
            ),
        ),
        crate::model::domain_package::RelationshipDirection::TargetToSource => (
            target_port,
            source_port,
            matches!(
                target_port.direction,
                Some(PortDirection::Out | PortDirection::InOut)
            ) && matches!(
                source_port.direction,
                Some(PortDirection::In | PortDirection::InOut)
            ),
        ),
        crate::model::domain_package::RelationshipDirection::Bidirectional => (
            source_port,
            target_port,
            matches!(source_port.direction, Some(PortDirection::InOut))
                && matches!(target_port.direction, Some(PortDirection::InOut)),
        ),
        crate::model::domain_package::RelationshipDirection::Undirected => {
            (source_port, target_port, false)
        }
    };
    if !direction_ok {
        failures.push(ConditionFailure {
            condition: "port-direction",
            code: Code::InvalidModelBinding,
            cause: ModelRefusalCause::PortDirection {
                source: source_port.key.clone(),
                target: target_port.key.clone(),
            },
            detail: format!(
                "direction {:?}: source {} is not compatible with target {}",
                relationship.direction, source_port.key.node, target_port.key.node
            ),
        });
    }

    // Condition 2: flow-source interface type conforms to flow-target
    // interface type (bidirectional: the two interface types are equal).
    if let Err(incomplete) = meter.charge(Charge::new(ChargePoint::SystemsConnectionCondition)) {
        return ConnectionCheckOutcome::Incomplete(incomplete);
    }
    let generals = generals_by_specific(domain_package);
    let interface_ok = if matches!(
        relationship.direction,
        crate::model::domain_package::RelationshipDirection::Bidirectional
    ) {
        flow_source.value_type.node == flow_target.value_type.node
    } else {
        match type_conforms(&generals, &flow_source.value_type, &flow_target.value_type) {
            Ok(conforms) => conforms,
            Err(refusal) => return ConnectionCheckOutcome::Refused(refusal),
        }
    };
    if !interface_ok {
        failures.push(ConditionFailure {
            condition: "interface-type",
            code: Code::IllTyped,
            cause: ModelRefusalCause::TypeMismatch,
            detail: format!(
                "{} does not conform to {}",
                flow_source.value_type.node, flow_target.value_type.node
            ),
        });
    }

    // Condition 3: each relationship end's multiplicity conforms to its
    // port's multiplicity.
    if let Err(incomplete) = meter.charge(Charge::new(ChargePoint::SystemsConnectionCondition)) {
        return ConnectionCheckOutcome::Incomplete(incomplete);
    }
    for (end, port, label) in [
        (&relationship.source, source_port, "source"),
        (&relationship.target, target_port, "target"),
    ] {
        if !multiplicity_conforms(&end.multiplicity, &port.multiplicity) {
            failures.push(ConditionFailure {
                condition: "multiplicity",
                code: Code::IllTyped,
                cause: ModelRefusalCause::MultiplicityNarrowing {
                    from: end.multiplicity,
                    to: port.multiplicity,
                },
                detail: format!(
                    "{label} end {:?} does not conform to port {:?}",
                    end.multiplicity, port.multiplicity
                ),
            });
        }
    }

    if failures.is_empty() {
        ConnectionCheckOutcome::Completed(ConnectionOutcome::Admitted)
    } else {
        ConnectionCheckOutcome::Completed(ConnectionOutcome::Refused(failures))
    }
}

/// The outcome of one [`check_allocation`] attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AllocationCheckOutcome {
    /// The allocation's target resolved as a Part.
    Admitted,
    /// The allocation's target did not resolve as a Part.
    Refused(ModelRefusal),
    /// A `ModelNormalizationLimitsV1` counter was exhausted mid-check.
    Incomplete(Incomplete),
}

/// Runs `quire.model.systems.allocation/v1` over `relationship_key`. Refuses
/// `wrong-export` outright if `relationship_key` does not itself classify as
/// [`Kind::Allocation`] (finding #5; mirrors [`check_connection`]'s own
/// leading kind guard), before charging `systems.allocation` once; the
/// target element must be a Part.
pub fn check_allocation(
    classification: &SystemsClassification,
    relationship_key: &DeclarationKey,
    meter: &mut Meter,
) -> AllocationCheckOutcome {
    if classification.actual_kind(relationship_key) != Kind::Allocation {
        return AllocationCheckOutcome::Refused(ModelRefusal {
            code: Code::InvalidModelBinding,
            cause: ModelRefusalCause::WrongExport,
            detail: format!(
                "{} is not an Allocation; the allocation rule does not apply",
                relationship_key.node
            ),
        });
    }
    if let Err(incomplete) = meter.charge(Charge::new(ChargePoint::SystemsAllocation)) {
        return AllocationCheckOutcome::Incomplete(incomplete);
    }
    let Some(allocation) = classification.allocations.get(relationship_key) else {
        return AllocationCheckOutcome::Refused(ModelRefusal {
            code: Code::DanglingReference,
            cause: ModelRefusalCause::UnknownRelationship {
                relationship: relationship_key.clone(),
            },
            detail: format!("{} is not a declared allocation", relationship_key.node),
        });
    };
    let target_kind = classification.actual_kind(&allocation.target_element);
    if target_kind == Kind::Part {
        AllocationCheckOutcome::Admitted
    } else {
        AllocationCheckOutcome::Refused(wrong_export(
            Kind::Part,
            target_kind,
            &allocation.target_element.node,
        ))
    }
}
