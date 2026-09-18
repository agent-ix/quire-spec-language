// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-152 systems-model binding: kind mapping
//! (`quire.model.systems.kind-mapping/v1`) and the connection/allocation
//! admission rules (`quire.model.systems.connection/v1`,
//! `quire.model.systems.allocation/v1`).
//!
//! Scope decisions, recorded rather than left implicit:
//!
//! - This module resolves a systems-model element **by [`ProducerKey`]
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
//!   `BundleRecord` variants). What this module resolves is the original
//!   producer identity and its kind only.
//! - Work-unit costs mirror the flat-one-per-charge choice
//!   [`crate::model::conformance`]/[`crate::model::dispatch`] already
//!   record, for the same reason: FR-152's own worked figures (e.g. Y04's
//!   `f(Flow)`/`f(Flow2)`) price `normalize`'s derivation-fact accounting,
//!   which this module would have to re-walk to reproduce exactly.

use std::collections::HashMap;

use crate::diagnostic::Code;
use crate::model::accounting::{Charge, ChargePoint, Incomplete, Meter};
use crate::model::bundle::{
    Bundle, BundleRecord, ComponentRecord, EndpointRecord, Multiplicity, PortDirection,
    RelationshipRecord,
};
use crate::model::conformance::{generals_by_specific, multiplicity_conforms, type_conforms};
use crate::model::key::ProducerKey;
use crate::model::normalize::ModelRefusal;

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
    /// A non-allocation relationship whose two ends are both Ports.
    Connection,
    /// A relationship whose `semantics.category` is exactly `"allocation"`.
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

/// The full kind mapping over one [`Bundle`]: every component's, endpoint's
/// and relationship's resolved [`Kind`], plus every interface object type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SystemsClassification {
    component_kinds: HashMap<String, Kind>,
    endpoint_kinds: HashMap<String, Kind>,
    relationship_kinds: HashMap<String, Kind>,
    interface_types: std::collections::HashSet<String>,
    components: HashMap<String, ComponentRecord>,
    endpoints: HashMap<String, EndpointRecord>,
    relationships: HashMap<String, RelationshipRecord>,
    object_types: std::collections::HashSet<String>,
    /// Every kind-mapping cascade refusal, in emission order (ascending
    /// producer key within each of components, then endpoints, then
    /// relationships).
    pub refusals: Vec<ModelRefusal>,
}

impl SystemsClassification {
    /// The resolved kind for any identity this classification knows about
    /// (a component, endpoint, relationship or interface object type);
    /// [`Kind::None`] for anything else, including a dangling identity.
    pub fn actual_kind(&self, identity: &str) -> Kind {
        if self.interface_types.contains(identity) {
            return Kind::Interface;
        }
        if let Some(kind) = self.component_kinds.get(identity) {
            return *kind;
        }
        if let Some(kind) = self.endpoint_kinds.get(identity) {
            return *kind;
        }
        if let Some(kind) = self.relationship_kinds.get(identity) {
            return *kind;
        }
        Kind::None
    }
}

fn wrong_export(required: Kind, actual: Kind, item: &str) -> ModelRefusal {
    ModelRefusal {
        code: Code::InvalidModelBinding,
        cause: "wrong-export",
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
        cause: "unsupplied-producer-record",
        detail: format!("{item} does not supply the {capability} capability"),
    }
}

fn charge_kind(meter: &mut Meter) -> Result<(), Incomplete> {
    meter.charge(Charge::new(ChargePoint::SystemsKind))
}

/// Runs `quire.model.systems.kind-mapping/v1` over every component, then
/// every endpoint, then every relationship in `bundle`, each ascending by
/// producer key, charging `systems.kind` once per record before resolving
/// it. Exhaustive under the limits: every no-kind cascade is reported, in
/// this same order, never stopping at the first.
pub fn classify(bundle: &Bundle, meter: &mut Meter) -> Result<SystemsClassification, Incomplete> {
    let mut components: Vec<&ComponentRecord> = Vec::new();
    let mut endpoints: Vec<&EndpointRecord> = Vec::new();
    let mut relationships: Vec<&RelationshipRecord> = Vec::new();
    let mut object_types: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut interface_types: std::collections::HashSet<String> = std::collections::HashSet::new();

    for record in &bundle.records {
        match record {
            BundleRecord::Component(component) => components.push(component),
            BundleRecord::Endpoint(endpoint) => endpoints.push(endpoint),
            BundleRecord::Relationship(relationship) => relationships.push(relationship),
            BundleRecord::ObjectType(object_type) => {
                object_types.insert(object_type.key.identity.clone());
                if object_type.interface_features.is_some() {
                    interface_types.insert(object_type.key.identity.clone());
                }
            }
            BundleRecord::FieldMember(_)
            | BundleRecord::Generalization(_)
            | BundleRecord::ScalarType(_)
            | BundleRecord::OperationMember(_)
            | BundleRecord::Redefinition(_)
            | BundleRecord::Subsetting(_) => {}
        }
    }
    components.sort_by_key(|component| component.key.clone());
    endpoints.sort_by_key(|endpoint| endpoint.key.clone());
    relationships.sort_by_key(|relationship| relationship.key.clone());

    let mut component_kinds: HashMap<String, Kind> = HashMap::new();
    let mut endpoint_kinds: HashMap<String, Kind> = HashMap::new();
    let mut relationship_kinds: HashMap<String, Kind> = HashMap::new();
    let mut refusals: Vec<ModelRefusal> = Vec::new();

    for component in &components {
        charge_kind(meter)?;
        let kind = if component.has_part_signature {
            Kind::Part
        } else {
            refusals.push(unsupplied("part-signature", &component.key.identity));
            Kind::None
        };
        component_kinds.insert(component.key.identity.clone(), kind);
    }

    for endpoint in &endpoints {
        charge_kind(meter)?;
        let kind = match &endpoint.direction {
            None => {
                refusals.push(unsupplied("port-direction", &endpoint.key.identity));
                Kind::None
            }
            Some(_) => {
                let owner_kind = component_kinds
                    .get(&endpoint.owning_component.identity)
                    .copied()
                    .unwrap_or(Kind::None);
                if owner_kind == Kind::Part {
                    Kind::Port
                } else {
                    refusals.push(wrong_export(Kind::Part, owner_kind, &endpoint.key.identity));
                    Kind::None
                }
            }
        };
        endpoint_kinds.insert(endpoint.key.identity.clone(), kind);
    }

    for relationship in &relationships {
        charge_kind(meter)?;
        let kind = if relationship.category == "allocation" {
            Kind::Allocation
        } else {
            let source_is_type = object_types.contains(&relationship.source.type_identity.identity);
            let target_is_type = object_types.contains(&relationship.target.type_identity.identity);
            if source_is_type && target_is_type {
                Kind::None // navigation relationship: no kind, not an error.
            } else {
                let mut ends_ok = true;
                for (end, label) in [
                    (&relationship.source, "source"),
                    (&relationship.target, "target"),
                ] {
                    let end_kind = endpoint_kinds
                        .get(&end.type_identity.identity)
                        .copied()
                        .unwrap_or(Kind::None);
                    if end_kind != Kind::Port {
                        refusals.push(wrong_export(
                            Kind::Port,
                            end_kind,
                            &format!("{} end of {}", label, relationship.key.identity),
                        ));
                        ends_ok = false;
                    }
                }
                if ends_ok {
                    Kind::Connection
                } else {
                    Kind::None
                }
            }
        };
        relationship_kinds.insert(relationship.key.identity.clone(), kind);
    }

    Ok(SystemsClassification {
        component_kinds,
        endpoint_kinds,
        relationship_kinds,
        interface_types,
        components: components
            .into_iter()
            .map(|c| (c.key.identity.clone(), c.clone()))
            .collect(),
        endpoints: endpoints
            .into_iter()
            .map(|e| (e.key.identity.clone(), e.clone()))
            .collect(),
        relationships: relationships
            .into_iter()
            .map(|r| (r.key.identity.clone(), r.clone()))
            .collect(),
        object_types,
        refusals,
    })
}

/// A binder request's result: `key` classified to exactly `required`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedElement {
    /// The resolved producer key (identical to the request's key: this
    /// module resolves by key, not by qualified name — see the module
    /// docs).
    pub key: ProducerKey,
    /// The resolved kind, always equal to the request's `required` kind.
    pub kind: Kind,
}

/// Resolves `key` as `required`, refusing `wrong-export` (a different real
/// kind) or `unsupplied-producer-record` (no kind at all) otherwise. Equal
/// names never substitute for a kind match — this module never inspects a
/// display name at all, only `key.identity` and the classification.
pub fn resolve_kind(
    classification: &SystemsClassification,
    required: Kind,
    key: &ProducerKey,
) -> Result<ResolvedElement, ModelRefusal> {
    let actual = classification.actual_kind(&key.identity);
    if actual == required {
        Ok(ResolvedElement {
            key: key.clone(),
            kind: actual,
        })
    } else if actual == Kind::None {
        Err(unsupplied_or_wrong(required, key))
    } else {
        Err(wrong_export(required, actual, &key.identity))
    }
}

fn unsupplied_or_wrong(required: Kind, key: &ProducerKey) -> ModelRefusal {
    ModelRefusal {
        code: Code::InvalidModelBinding,
        cause: "unsupplied-producer-record",
        detail: format!(
            "{} does not resolve to the required kind {}",
            key.identity,
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
    pub cause: &'static str,
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
    end_identity: &str,
) -> Option<&'a EndpointRecord> {
    classification.endpoints.get(end_identity)
}

/// Runs `quire.model.systems.connection/v1` over `relationship_key`, which
/// must already classify as [`Kind::Connection`]. Charges
/// `systems.connection-condition` for each of the three table conditions,
/// in table order, and reports every failure (never stop-at-first).
pub fn check_connection(
    bundle: &Bundle,
    classification: &SystemsClassification,
    relationship_key: &ProducerKey,
    meter: &mut Meter,
) -> ConnectionCheckOutcome {
    if classification.actual_kind(&relationship_key.identity) != Kind::Connection {
        return ConnectionCheckOutcome::Refused(ModelRefusal {
            code: Code::InvalidModelBinding,
            cause: "wrong-export",
            detail: format!(
                "{} is not a Connection; the connection rule does not apply",
                relationship_key.identity
            ),
        });
    }
    let Some(relationship) = classification.relationships.get(&relationship_key.identity) else {
        return ConnectionCheckOutcome::Refused(ModelRefusal {
            code: Code::DanglingReference,
            cause: "unknown-relationship",
            detail: format!(
                "{} is not a declared relationship",
                relationship_key.identity
            ),
        });
    };
    let Some(source_port) = end_port(classification, &relationship.source.type_identity.identity)
    else {
        return ConnectionCheckOutcome::Refused(ModelRefusal {
            code: Code::DanglingReference,
            cause: "unknown-source-port",
            detail: format!(
                "{} names an end that is not a declared endpoint",
                relationship.source.type_identity.identity
            ),
        });
    };
    let Some(target_port) = end_port(classification, &relationship.target.type_identity.identity)
    else {
        return ConnectionCheckOutcome::Refused(ModelRefusal {
            code: Code::DanglingReference,
            cause: "unknown-target-port",
            detail: format!(
                "{} names an end that is not a declared endpoint",
                relationship.target.type_identity.identity
            ),
        });
    };

    let mut failures = Vec::new();

    // Condition 1: port-direction.
    if let Err(incomplete) = meter.charge(Charge::new(ChargePoint::SystemsConnectionCondition)) {
        return ConnectionCheckOutcome::Incomplete(incomplete);
    }
    let (flow_source, flow_target, direction_ok) = match relationship.direction {
        crate::model::bundle::RelationshipDirection::SourceToTarget => (
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
        crate::model::bundle::RelationshipDirection::TargetToSource => (
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
        crate::model::bundle::RelationshipDirection::Bidirectional => (
            source_port,
            target_port,
            matches!(source_port.direction, Some(PortDirection::InOut))
                && matches!(target_port.direction, Some(PortDirection::InOut)),
        ),
        crate::model::bundle::RelationshipDirection::Undirected => {
            (source_port, target_port, false)
        }
    };
    if !direction_ok {
        failures.push(ConditionFailure {
            condition: "port-direction",
            code: Code::InvalidModelBinding,
            cause: "port-direction",
            detail: format!(
                "direction {:?}: source {} is not compatible with target {}",
                relationship.direction, source_port.key.identity, target_port.key.identity
            ),
        });
    }

    // Condition 2: flow-source interface type conforms to flow-target
    // interface type (bidirectional: the two interface types are equal).
    if let Err(incomplete) = meter.charge(Charge::new(ChargePoint::SystemsConnectionCondition)) {
        return ConnectionCheckOutcome::Incomplete(incomplete);
    }
    let generals = generals_by_specific(bundle);
    let interface_ok = if matches!(
        relationship.direction,
        crate::model::bundle::RelationshipDirection::Bidirectional
    ) {
        flow_source.value_type.identity == flow_target.value_type.identity
    } else {
        match type_conforms(
            &generals,
            &flow_source.value_type.identity,
            &flow_target.value_type.identity,
        ) {
            Ok(conforms) => conforms,
            Err(refusal) => return ConnectionCheckOutcome::Refused(refusal),
        }
    };
    if !interface_ok {
        failures.push(ConditionFailure {
            condition: "interface-type",
            code: Code::IllTyped,
            cause: "type-mismatch",
            detail: format!(
                "{} does not conform to {}",
                flow_source.value_type.identity, flow_target.value_type.identity
            ),
        });
    }

    // Condition 3: each relationship end's multiplicity conforms to its
    // port's multiplicity.
    if let Err(incomplete) = meter.charge(Charge::new(ChargePoint::SystemsConnectionCondition)) {
        return ConnectionCheckOutcome::Incomplete(incomplete);
    }
    let mut multiplicity_ok = true;
    for (end, port, label) in [
        (&relationship.source, source_port, "source"),
        (&relationship.target, target_port, "target"),
    ] {
        if !multiplicity_conforms(&end.multiplicity, &port.multiplicity) {
            multiplicity_ok = false;
            failures.push(ConditionFailure {
                condition: "multiplicity",
                code: Code::IllTyped,
                cause: "multiplicity-narrowing",
                detail: format!(
                    "{label} end {:?} does not conform to port {:?}",
                    end.multiplicity, port.multiplicity
                ),
            });
        }
    }
    let _ = multiplicity_ok;

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

/// Runs `quire.model.systems.allocation/v1` over `relationship_key`, which
/// must already classify as [`Kind::Allocation`]. Charges
/// `systems.allocation` once; the target element must be a Part.
pub fn check_allocation(
    classification: &SystemsClassification,
    relationship_key: &ProducerKey,
    meter: &mut Meter,
) -> AllocationCheckOutcome {
    if let Err(incomplete) = meter.charge(Charge::new(ChargePoint::SystemsAllocation)) {
        return AllocationCheckOutcome::Incomplete(incomplete);
    }
    let Some(relationship) = classification.relationships.get(&relationship_key.identity) else {
        return AllocationCheckOutcome::Refused(ModelRefusal {
            code: Code::DanglingReference,
            cause: "unknown-relationship",
            detail: format!(
                "{} is not a declared relationship",
                relationship_key.identity
            ),
        });
    };
    let target_identity = &relationship.target.type_identity.identity;
    let target_kind = classification.actual_kind(target_identity);
    if target_kind == Kind::Part {
        AllocationCheckOutcome::Admitted
    } else {
        AllocationCheckOutcome::Refused(wrong_export(Kind::Part, target_kind, target_identity))
    }
}

#[allow(dead_code)] // Referenced only by doc comments today; kept for the
                    // Multiplicity import's own doc-link target stability.
fn _unused(_: &Multiplicity) {}
