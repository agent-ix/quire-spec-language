// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-492 "Slot key": the identity of a premise slot, wider than the
//! FR-201 obligation identity on purpose. Two derived items that share an
//! anchor node (the four `operation-contract` roles of one guarantee) or
//! differ only in fairness or `stutter_closure` have different keys.
//!
//! An item-backed slot's key is read from the premise's FR-331 request item
//! ([`read_premise_item`](super::read_premise_item)) and is written back as
//! the same item-shaped members ([`SlotKey::to_wire`]), so the slot
//! `identity` of a request is exactly what the reader reads from the item.

use qsl_foundation::node_wire::{node_key_from_json, node_key_to_json, NodeWireError};
use qsl_foundation::source::provenance::OccurrenceKey;
use quire_exact::{NodeKey, Origin, Role};
use quire_semantic_value::fairness::FairnessConstraint;
use serde_json::{json, Map, Value};

use super::declaration::SlotRole;
use super::json::{array, canonical_text, check_members, count, object, require, string, Fault};
use crate::identity::ObligationIdentity;
use crate::proof_result::child;

/// FR-290's closed capability-kind vocabulary (`quire.capability-kind/v1`).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ClaimKind {
    /// `value-validity`.
    ValueValidity,
    /// `operation-contract`.
    OperationContract,
    /// `finite-replay`.
    FiniteReplay,
    /// `temporal-satisfaction`.
    TemporalSatisfaction,
    /// `global-conformance`.
    GlobalConformance,
    /// `monitorability`.
    Monitorability,
    /// `local-projection`.
    LocalProjection,
    /// `refinement`.
    Refinement,
    /// `realizability`.
    Realizability,
    /// `composition`.
    Composition,
    /// `probabilistic-satisfaction`.
    ProbabilisticSatisfaction,
    /// `schedulability`.
    Schedulability,
}

impl ClaimKind {
    /// Every kind.
    pub const ALL: [Self; 12] = [
        Self::ValueValidity,
        Self::OperationContract,
        Self::FiniteReplay,
        Self::TemporalSatisfaction,
        Self::GlobalConformance,
        Self::Monitorability,
        Self::LocalProjection,
        Self::Refinement,
        Self::Realizability,
        Self::Composition,
        Self::ProbabilisticSatisfaction,
        Self::Schedulability,
    ];

    /// The kind's wire spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ValueValidity => "value-validity",
            Self::OperationContract => "operation-contract",
            Self::FiniteReplay => "finite-replay",
            Self::TemporalSatisfaction => "temporal-satisfaction",
            Self::GlobalConformance => "global-conformance",
            Self::Monitorability => "monitorability",
            Self::LocalProjection => "local-projection",
            Self::Refinement => "refinement",
            Self::Realizability => "realizability",
            Self::Composition => "composition",
            Self::ProbabilisticSatisfaction => "probabilistic-satisfaction",
            Self::Schedulability => "schedulability",
        }
    }

    /// The kind `text` spells.
    #[ix_trace_rs::string_edge]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == text)
    }
}

/// QSpec FR-368 "Item kinds": a temporal item's `item_kind`.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum TemporalItemKind {
    /// An authored clause.
    Formula,
    /// The derived deadlock-freedom item.
    DeadlockFreedom,
    /// The derived time-lock-freedom item.
    TimeLockFreedom,
    /// The derived race-freedom item.
    RaceFreedom,
    /// A refinement item.
    Refinement,
}

impl TemporalItemKind {
    /// The `item_kind` spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Formula => "formula",
            Self::DeadlockFreedom => "deadlock-freedom",
            Self::TimeLockFreedom => "time-lock-freedom",
            Self::RaceFreedom => "race-freedom",
            Self::Refinement => "refinement",
        }
    }

    #[ix_trace_rs::string_edge]
    fn parse(text: &str) -> Option<Self> {
        [
            Self::Formula,
            Self::DeadlockFreedom,
            Self::TimeLockFreedom,
            Self::RaceFreedom,
            Self::Refinement,
        ]
        .into_iter()
        .find(|kind| kind.as_str() == text)
    }
}

/// The `role` of an `operation-contract` item's `correspondence` member.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CorrespondenceRole {
    /// `initialization`.
    Initialization,
    /// `transition-simulation`.
    TransitionSimulation,
    /// `frame`.
    Frame,
    /// `no-divergence`.
    NoDivergence,
}

impl CorrespondenceRole {
    /// The role's spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Initialization => "initialization",
            Self::TransitionSimulation => "transition-simulation",
            Self::Frame => "frame",
            Self::NoDivergence => "no-divergence",
        }
    }

    #[ix_trace_rs::string_edge]
    fn parse(text: &str) -> Option<Self> {
        [
            Self::Initialization,
            Self::TransitionSimulation,
            Self::Frame,
            Self::NoDivergence,
        ]
        .into_iter()
        .find(|role| role.as_str() == text)
    }
}

/// A model operation as QSpec FR-490 keys it: the context node and the
/// operation name.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct OperationRef {
    /// The context node.
    pub context: NodeKey,
    /// The operation name.
    pub operation: String,
}

/// The `correspondence` member of an `operation-contract` item (QSpec
/// FR-331 "Items").
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Correspondence {
    /// Which correspondence or progress role the item has.
    pub role: CorrespondenceRole,
    /// The operation; absent exactly for `initialization`.
    pub operation: Option<OperationRef>,
}

/// An extent's classification (QSpec FR-331 "Items"); the bounds themselves
/// are not part of the slot key.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ExtentClass {
    /// `bounded`.
    Bounded,
    /// `unbounded`.
    Unbounded,
}

impl ExtentClass {
    /// The classification's spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bounded => "bounded",
            Self::Unbounded => "unbounded",
        }
    }
}

/// One fairness constraint of an item's resolved fairness set, as its
/// canonical JSON text: equal exactly when the constraints are.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct FairnessKey(String);

impl FairnessKey {
    /// The key of `constraint`, through the same canonical writer a
    /// request's fairness member uses.
    ///
    /// # Errors
    ///
    /// [`KeyWireError`] when the constraint cannot be written.
    pub fn of(constraint: &FairnessConstraint) -> Result<Self, KeyWireError> {
        let bytes = quire_canonical::to_vec(constraint, quire_canonical::Limits::new(u64::MAX))
            .map_err(|_| KeyWireError)?;
        let value: Value = serde_json::from_slice(&bytes).map_err(|_| KeyWireError)?;
        Ok(Self(canonical_text(&value)))
    }

    pub(crate) fn to_value(&self) -> Result<Value, KeyWireError> {
        serde_json::from_str(&self.0).map_err(|_| KeyWireError)
    }
}

/// One entry of a composition item's `premises` member.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PremiseKey {
    /// The premise slot's role.
    pub role: SlotRole,
    /// The premise slot's key, as canonical JSON text.
    pub key: String,
}

/// The `temporal` members of a temporal item that belong to its slot key.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TemporalKey {
    /// The item's `subject` member as canonical JSON text: initial-state
    /// snapshots, universes and proof bounds.
    pub subject: String,
    /// The item kind.
    pub item_kind: TemporalItemKind,
    /// The resolved fairness set, sorted.
    pub fairness: Vec<FairnessKey>,
    /// Whether the clause is checked over the subject's stutter closure.
    pub stutter_closure: bool,
}

/// The slot key of a request-item-backed slot.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ItemKey {
    /// The claim: its node and occurrence.
    pub claim: OccurrenceKey,
    /// The FR-290 kind.
    pub kind: ClaimKind,
    /// A temporal item's subject, item kind, fairness and `stutter_closure`.
    pub temporal: Option<TemporalKey>,
    /// An `operation-contract` item's correspondence.
    pub correspondence: Option<Correspondence>,
    /// A composition item's premises, in order.
    pub premises: Vec<PremiseKey>,
    /// The extent classification.
    pub extent: ExtentClass,
}

impl ItemKey {
    /// The FR-201 obligation identity of the item: its claim node and
    /// occurrence ordinal. The model owning the claim is not on the wire, so
    /// the subject member carries it.
    pub fn identity(&self) -> ObligationIdentity {
        ObligationIdentity::from_claim_occurrence(&self.claim)
    }
}

/// The key of a slot that has no request item: its role and the members
/// that produced it.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct LocalKey {
    /// The slot's role.
    pub role: SlotRole,
    /// The producing members, as text (an environment member's name, a
    /// constraint's canonical text, a component and clause).
    pub members: Vec<String>,
}

/// QSpec FR-492 "Slot key".
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum SlotKey {
    /// A slot backed by a request item.
    Item(ItemKey),
    /// Any other slot.
    Local(LocalKey),
}

/// A key member could not be written (an allocation failure in the node
/// writer, or a malformed stored text).
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("a slot key member could not be written")]
pub struct KeyWireError;

impl From<NodeWireError> for KeyWireError {
    fn from(_: NodeWireError) -> Self {
        Self
    }
}

fn node_value(node: &NodeKey) -> Result<Value, KeyWireError> {
    Ok(node_key_to_json(node)?)
}

fn operation_value(operation: &OperationRef) -> Result<Value, KeyWireError> {
    Ok(json!({"context": node_value(&operation.context)?, "operation": operation.operation}))
}

impl SlotKey {
    /// The key as the slot `identity` of a request: for an item-backed slot
    /// the item-shaped slot-key members [`read_premise_item`] reads back,
    /// for any other slot `{role, members}`.
    ///
    /// # Errors
    ///
    /// [`KeyWireError`] when a member cannot be written.
    ///
    /// [`read_premise_item`]: super::read_premise_item
    pub fn to_wire(&self) -> Result<Value, KeyWireError> {
        match self {
            Self::Item(key) => key.to_wire(),
            Self::Local(key) => Ok(json!({"role": key.role.as_str(), "members": key.members})),
        }
    }
}

impl ItemKey {
    /// The item-shaped slot-key members (no limits, candidates, options or
    /// wall-clock members).
    ///
    /// # Errors
    ///
    /// [`KeyWireError`] when a member cannot be written.
    pub fn to_wire(&self) -> Result<Value, KeyWireError> {
        let mut item = Map::new();
        item.insert("kind".to_owned(), json!(self.kind.as_str()));
        item.insert(
            "claim".to_owned(),
            json!({
                "node": node_value(self.claim.node())?,
                "occurrence": {
                    "role": self.claim.origin().role().as_str(),
                    "ordinal": self.claim.origin().ordinal(),
                },
            }),
        );
        item.insert(
            "extent".to_owned(),
            json!({"classification": self.extent.as_str()}),
        );
        if let Some(temporal) = &self.temporal {
            let mut members = Map::new();
            members.insert(
                "subject".to_owned(),
                serde_json::from_str(&temporal.subject).map_err(|_| KeyWireError)?,
            );
            members.insert("item_kind".to_owned(), json!(temporal.item_kind.as_str()));
            let fairness: Result<Vec<Value>, KeyWireError> = temporal
                .fairness
                .iter()
                .map(FairnessKey::to_value)
                .collect();
            members.insert("fairness".to_owned(), Value::Array(fairness?));
            if temporal.stutter_closure {
                members.insert("stutter_closure".to_owned(), Value::Bool(true));
            }
            item.insert("temporal".to_owned(), Value::Object(members));
        }
        if let Some(correspondence) = &self.correspondence {
            let mut members = Map::new();
            members.insert("role".to_owned(), json!(correspondence.role.as_str()));
            if let Some(operation) = &correspondence.operation {
                members.insert("operation".to_owned(), operation_value(operation)?);
            }
            item.insert("correspondence".to_owned(), Value::Object(members));
        }
        if !self.premises.is_empty() {
            let premises: Result<Vec<Value>, KeyWireError> = self
                .premises
                .iter()
                .map(|premise| {
                    Ok(json!({
                        "role": premise.role.as_str(),
                        "key": serde_json::from_str::<Value>(&premise.key)
                            .map_err(|_| KeyWireError)?,
                    }))
                })
                .collect();
            item.insert("premises".to_owned(), Value::Array(premises?));
        }
        Ok(Value::Object(item))
    }
}

/// A structural node id member, `wrong-value-kind` when it is no object.
pub(crate) fn read_node(value: &Value, pointer: &str) -> Result<NodeKey, Fault> {
    object(value, pointer)?;
    node_key_from_json(value).map_err(|_| Fault::invalid(pointer.to_owned()))
}

#[ix_trace_rs::string_edge]
fn read_occurrence(value: &Value, pointer: &str) -> Result<Origin, Fault> {
    const ROLES: [&str; 6] = [
        "declaration",
        "type",
        "expression",
        "anchor",
        "claim",
        "generated",
    ];
    let members = object(value, pointer)?;
    check_members(members, pointer, &["ordinal", "role"], &[])?;
    let role_pointer = child(pointer, "role");
    let role = string(require(members, "role", pointer)?, &role_pointer)?;
    if !ROLES.contains(&role) {
        return Err(Fault::invalid(role_pointer));
    }
    let ordinal = count(
        require(members, "ordinal", pointer)?,
        &child(pointer, "ordinal"),
    )?;
    Ok(Origin::new(Role::new(role), ordinal))
}

fn read_fairness_entry(value: &Value, pointer: &str) -> Result<FairnessKey, Fault> {
    let members = object(value, pointer)?;
    check_members(members, pointer, &["granularity", "kind", "operation"], &[])?;
    for (name, allowed) in [
        ("granularity", ["whole", "each"]),
        ("kind", ["weak", "strong"]),
    ] {
        let member_pointer = child(pointer, name);
        let text = string(require(members, name, pointer)?, &member_pointer)?;
        if !allowed.contains(&text) {
            return Err(Fault::invalid(member_pointer));
        }
    }
    let operation_pointer = child(pointer, "operation");
    match require(members, "operation", pointer)? {
        Value::String(name) if !name.is_empty() => {}
        Value::Object(_) => {}
        Value::String(_) => return Err(Fault::invalid(operation_pointer)),
        _ => return Err(Fault::wrong_kind(operation_pointer)),
    }
    Ok(FairnessKey(canonical_text(value)))
}

fn read_operation(value: &Value, pointer: &str) -> Result<OperationRef, Fault> {
    let members = object(value, pointer)?;
    check_members(members, pointer, &["context", "operation"], &[])?;
    let context = read_node(
        require(members, "context", pointer)?,
        &child(pointer, "context"),
    )?;
    let operation_pointer = child(pointer, "operation");
    let operation = string(require(members, "operation", pointer)?, &operation_pointer)?;
    if operation.is_empty() {
        return Err(Fault::invalid(operation_pointer));
    }
    Ok(OperationRef {
        context,
        operation: operation.to_owned(),
    })
}

#[ix_trace_rs::string_edge]
fn read_temporal(value: &Value, pointer: &str) -> Result<TemporalKey, Fault> {
    let members = object(value, pointer)?;
    // Every other temporal member (limits, horizon, sampling) is not a
    // slot-key member and is skipped.
    for name in ["fairness", "item_kind", "subject"] {
        require(members, name, pointer)?;
    }
    let subject_pointer = child(pointer, "subject");
    let subject = require(members, "subject", pointer)?;
    object(subject, &subject_pointer)?;
    let kind_pointer = child(pointer, "item_kind");
    let item_kind = string(require(members, "item_kind", pointer)?, &kind_pointer)?;
    let item_kind =
        TemporalItemKind::parse(item_kind).ok_or_else(|| Fault::invalid(kind_pointer))?;
    let fairness_pointer = child(pointer, "fairness");
    let mut fairness = Vec::new();
    for (index, entry) in array(require(members, "fairness", pointer)?, &fairness_pointer)?
        .iter()
        .enumerate()
    {
        fairness.push(read_fairness_entry(
            entry,
            &format!("{fairness_pointer}/{index}"),
        )?);
    }
    fairness.sort();
    let stutter_closure = match members.get("stutter_closure") {
        None => false,
        Some(Value::Bool(true)) if item_kind == TemporalItemKind::Formula => true,
        Some(Value::Bool(_)) => return Err(Fault::invalid(child(pointer, "stutter_closure"))),
        Some(_) => return Err(Fault::wrong_kind(child(pointer, "stutter_closure"))),
    };
    Ok(TemporalKey {
        subject: canonical_text(subject),
        item_kind,
        fairness,
        stutter_closure,
    })
}

#[ix_trace_rs::string_edge]
fn read_correspondence(value: &Value, pointer: &str) -> Result<Correspondence, Fault> {
    let members = object(value, pointer)?;
    check_members(members, pointer, &["role"], &["operation"])?;
    let role_pointer = child(pointer, "role");
    let role = string(require(members, "role", pointer)?, &role_pointer)?;
    let role = CorrespondenceRole::parse(role).ok_or_else(|| Fault::invalid(role_pointer))?;
    let operation_pointer = child(pointer, "operation");
    let operation = match (role, members.get("operation")) {
        (CorrespondenceRole::Initialization, None) => None,
        (CorrespondenceRole::Initialization, Some(_)) => {
            return Err(Fault::invalid(operation_pointer))
        }
        (_, None) => return Err(Fault::missing(operation_pointer)),
        (_, Some(operation)) => Some(read_operation(operation, &operation_pointer)?),
    };
    Ok(Correspondence { role, operation })
}

#[ix_trace_rs::string_edge]
fn read_premises(value: &Value, pointer: &str) -> Result<Vec<PremiseKey>, Fault> {
    let mut premises = Vec::new();
    for (index, entry) in array(value, pointer)?.iter().enumerate() {
        let entry_pointer = format!("{pointer}/{index}");
        let members = object(entry, &entry_pointer)?;
        check_members(members, &entry_pointer, &["key", "role"], &[])?;
        let role_pointer = child(&entry_pointer, "role");
        let role = string(require(members, "role", &entry_pointer)?, &role_pointer)?;
        let role = match SlotRole::parse(role) {
            Some(
                role @ (SlotRole::ComponentGuarantee
                | SlotRole::Rely
                | SlotRole::LivenessDependency
                | SlotRole::Fairness
                | SlotRole::Environment),
            ) => role,
            _ => return Err(Fault::invalid(role_pointer)),
        };
        let key_pointer = child(&entry_pointer, "key");
        let key = require(members, "key", &entry_pointer)?;
        object(key, &key_pointer)?;
        premises.push(PremiseKey {
            role,
            key: canonical_text(key),
        });
    }
    Ok(premises)
}

/// Read the slot-key members of a request item `item`, relative to
/// `pointer`. Members that are not slot-key members are skipped.
#[ix_trace_rs::string_edge]
pub(crate) fn read_item_key(item: &Map<String, Value>, pointer: &str) -> Result<ItemKey, Fault> {
    let kind_pointer = child(pointer, "kind");
    let kind = require(item, "kind", pointer)?;
    let kind = ClaimKind::parse(string(kind, &kind_pointer)?)
        .ok_or_else(|| Fault::invalid(kind_pointer.clone()))?;

    let claim_pointer = child(pointer, "claim");
    let claim = object(require(item, "claim", pointer)?, &claim_pointer)?;
    check_members(claim, &claim_pointer, &["node", "occurrence"], &[])?;
    let node = read_node(
        require(claim, "node", &claim_pointer)?,
        &child(&claim_pointer, "node"),
    )?;
    let origin = read_occurrence(
        require(claim, "occurrence", &claim_pointer)?,
        &child(&claim_pointer, "occurrence"),
    )?;

    let extent_pointer = child(pointer, "extent");
    let extent = object(require(item, "extent", pointer)?, &extent_pointer)?;
    let class_pointer = child(&extent_pointer, "classification");
    let class = string(
        require(extent, "classification", &extent_pointer)?,
        &class_pointer,
    )?;
    let extent = match class {
        "bounded" => ExtentClass::Bounded,
        "unbounded" => ExtentClass::Unbounded,
        _ => return Err(Fault::invalid(class_pointer)),
    };

    let temporal_pointer = child(pointer, "temporal");
    let temporal = match (kind, item.get("temporal")) {
        (ClaimKind::TemporalSatisfaction, Some(temporal)) => {
            Some(read_temporal(temporal, &temporal_pointer)?)
        }
        (ClaimKind::TemporalSatisfaction, None) => return Err(Fault::missing(temporal_pointer)),
        (_, None) => None,
        (_, Some(_)) => return Err(Fault::invalid(temporal_pointer)),
    };
    let correspondence_pointer = child(pointer, "correspondence");
    let correspondence = match (kind, item.get("correspondence")) {
        (ClaimKind::OperationContract, Some(value)) => {
            Some(read_correspondence(value, &correspondence_pointer)?)
        }
        (_, None) => None,
        (_, Some(_)) => return Err(Fault::invalid(correspondence_pointer)),
    };
    let premises_pointer = child(pointer, "premises");
    let premises = match (kind, item.get("premises")) {
        (ClaimKind::Composition, Some(value)) => read_premises(value, &premises_pointer)?,
        (_, None) => Vec::new(),
        (_, Some(_)) => return Err(Fault::invalid(premises_pointer)),
    };
    Ok(ItemKey {
        claim: OccurrenceKey::new(node, origin),
        kind,
        temporal,
        correspondence,
        premises,
        extent,
    })
}
