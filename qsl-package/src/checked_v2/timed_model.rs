// SPDX-License-Identifier: AGPL-3.0-or-later
//! Typed projection of QSpec FR-322's admitted timed-model graph.

use std::collections::BTreeMap;
use std::str::FromStr;

use quire_contract_model::{
    CheckedDeclaration, CheckedNodeId, CheckedNodeOwner, CheckedOccurrence, CheckedPackageV2,
    CheckedSemanticNodeV2, NominalIdentityPreimage,
};
use quire_exact::{Integer, Rational};
use serde_json::Value;

/// One admitted timed model, reconstructed from its closed graph nodes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedTimedModel {
    /// The root node's identity and checked wire metadata.
    pub metadata: TimedNodeMetadata,
    /// The model node this temporal model describes.
    pub model: CheckedNodeId,
    /// The time dimension node.
    pub time_dimension: CheckedNodeId,
    /// Its unique time source.
    pub source: CheckedTimeSource,
    /// Clock places in the root's declared order.
    pub clock_places: Vec<CheckedClockPlace>,
    /// Clock resets in the root's declared order.
    pub resets: Vec<CheckedClockReset>,
    /// Guard declarations and their checked predicates.
    pub guards: Vec<CheckedTimedPredicate>,
    /// Invariant declarations and their checked predicates.
    pub invariants: Vec<CheckedTimedPredicate>,
    /// Urgency declarations in the root's declared order.
    pub urgencies: Vec<CheckedUrgency>,
    /// Normalized constraints in the root's declared order.
    pub normalized_constraints: Vec<CheckedClockConstraint>,
    /// Primitive scalar type identities used by timed-node literals.
    pub literal_types: TimedLiteralTypes,
}

/// Checked outer metadata retained for one timed node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimedNodeMetadata {
    /// Stable node identity.
    pub node_id: CheckedNodeId,
    /// Schema version.
    pub schema_version: Box<str>,
    /// Semantic type identity.
    pub semantic_type: CheckedNodeId,
    /// Sorted dependency identities.
    pub dependencies: Vec<CheckedNodeId>,
    /// Source occurrence identities.
    pub occurrences: Vec<CheckedOccurrence>,
    /// Recursion group, absent for timed nodes.
    pub recursion_group: Option<Box<str>>,
    /// Nominal identity preimage, absent for timed nodes.
    pub nominal_identity_preimage: Option<NominalIdentityPreimage>,
    /// Declaration name, present for the root.
    pub declaration: Option<CheckedDeclaration>,
    /// Structural owner, present for the root.
    pub owner: Option<CheckedNodeOwner>,
}

/// Primitive scalar types needed to write timed-node literals.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimedLiteralTypes {
    /// Text literal type.
    pub text: CheckedNodeId,
    /// Integer literal type.
    pub integer: Option<CheckedNodeId>,
}

/// One checked guard or time invariant.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedTimedPredicate {
    /// The node's checked wire metadata.
    pub metadata: TimedNodeMetadata,
    /// The checked Boolean predicate node.
    pub predicate: CheckedNodeId,
}

/// A dense or tick time source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedTimeSource {
    /// The node's checked wire metadata.
    pub metadata: TimedNodeMetadata,
    /// `dense` or `tick`.
    pub kind: String,
    /// The selected time unit node.
    pub unit: CheckedNodeId,
    /// Tick model, when the source is `tick`.
    pub model: Option<CheckedNodeId>,
    /// Tick operation anchor, when the source is `tick`.
    pub operation: Option<CheckedNodeId>,
    /// Positive tick period in reduced `numerator/denominator` form.
    pub period: Option<String>,
    /// Rational type of the tick period.
    pub period_type: Option<CheckedNodeId>,
}

/// A field, tuple, union or collection-slot step in a clock-place path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClockPathStep {
    /// A named field.
    Field(String),
    /// A tuple element.
    Tuple(u64),
    /// A named union case.
    Union(String),
    /// A collection slot, selected by index or key.
    Slot {
        /// The collection kind.
        collection: String,
        /// A key node reference or a fixed index.
        selector: ClockSlotSelector,
        /// Declared lower bound.
        minimum: u64,
        /// Declared upper bound, or `None` when unbounded.
        maximum: Option<u64>,
    },
}

/// A clock-place collection slot selector.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClockSlotSelector {
    /// A key value node.
    Key(CheckedNodeId),
    /// A fixed sequence index.
    Index(u64),
}

/// One resolved row for a clock-place path step.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedClockPathStep {
    /// The zero-based path position.
    pub path_step: u64,
    /// The declaring type identity.
    pub declaring_type: String,
    /// The member identity.
    pub member_identity: String,
    /// The selected member's declared type.
    pub member_type: String,
    /// The collection kind, or `none` for a non-collection step.
    pub collection_kind: String,
    /// The declared lower bound, empty when not a collection step.
    pub minimum: String,
    /// The declared upper bound, empty when not a collection step.
    pub maximum: String,
}

/// One clock place and its resolved declaration path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedClockPlace {
    /// The node's checked wire metadata.
    pub metadata: TimedNodeMetadata,
    /// The path from its owning model object to the clock field.
    pub path: Vec<ClockPathStep>,
    /// The IR-admitted path resolution rows.
    pub resolved_path: Vec<ResolvedClockPathStep>,
    /// The model node this place belongs to.
    pub model: CheckedNodeId,
    /// Guard nodes associated with this place.
    pub guards: Vec<CheckedNodeId>,
}

/// A checked clock reset with an exact normalized nonnegative value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedClockReset {
    /// The node's checked wire metadata.
    pub metadata: TimedNodeMetadata,
    /// The clock place being reset.
    pub place: CheckedNodeId,
    /// The reduced reset quantity.
    pub value: String,
    /// Quantity type, which is the selected unit node.
    pub value_type: CheckedNodeId,
}

/// A data-predicate or operation urgency.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CheckedUrgency {
    /// `urgent when predicate`.
    When {
        /// The urgency node.
        metadata: TimedNodeMetadata,
        /// The checked predicate node.
        predicate: CheckedNodeId,
    },
    /// `urgent operation name`.
    Operation {
        /// The urgency node.
        metadata: TimedNodeMetadata,
        /// The model node.
        model: CheckedNodeId,
        /// The declared operation name.
        operation_name: String,
    },
}

/// One exact normalized clock bound.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedClockConstraint {
    /// The node's checked wire metadata.
    pub metadata: TimedNodeMetadata,
    /// The clock place it constrains.
    pub place: CheckedNodeId,
    /// `lt`, `le`, `gt` or `ge`.
    pub comparison: String,
    /// The reduced rational bound.
    pub bound: String,
    /// The source's selected time unit for this normalized bound.
    pub unit: CheckedNodeId,
    /// Quantity type, which is the selected unit node.
    pub bound_type: CheckedNodeId,
}

impl CheckedTimedModel {
    /// Reconstruct every admitted timed-model root and its typed children.
    ///
    /// IR admission has already checked each closed body and all typed edges;
    /// this projection nevertheless returns an error if its internal view no
    /// longer agrees with that admitted shape.
    pub fn from_admitted(package: &CheckedPackageV2) -> Result<Vec<Self>, &'static str> {
        let nodes = package
            .graph()
            .nodes
            .iter()
            .filter(|node| node.node_tag.as_ref() == "timed_model")
            .cloned()
            .collect::<Vec<_>>();
        Self::from_nodes(&nodes)
    }

    /// Reconstruct typed models from generated wire nodes in this crate's
    /// round-trip conformance tests.
    pub(crate) fn from_nodes(
        wire_nodes: &[CheckedSemanticNodeV2],
    ) -> Result<Vec<Self>, &'static str> {
        let nodes = wire_nodes
            .iter()
            .filter(|node| node.node_tag.as_ref() == "timed_model")
            .map(|node| (node.node_id.clone(), node))
            .collect::<BTreeMap<_, _>>();
        let mut roots = nodes
            .values()
            .filter(|node| node.semantic_form.as_ref() == "timed_model")
            .copied()
            .collect::<Vec<_>>();
        roots.sort_by(|a, b| a.node_id.digest.cmp(&b.node_id.digest));
        if roots.is_empty() {
            return Ok(Vec::new());
        }
        let literal_types = TimedLiteralTypes {
            text: find_literal_type(&nodes, "text").ok_or("timed nodes lack a text type")?,
            integer: find_literal_type(&nodes, "integer"),
        };
        roots
            .into_iter()
            .map(|root| project_root(root, &nodes, &literal_types))
            .collect()
    }

    /// Write all timed nodes from this typed model in FR-322 graph order.
    pub fn write_nodes(&self) -> Result<Vec<CheckedSemanticNodeV2>, &'static str> {
        let mut nodes = Vec::with_capacity(
            1 + self.clock_places.len()
                + self.resets.len()
                + self.guards.len()
                + self.invariants.len()
                + self.urgencies.len()
                + self.normalized_constraints.len(),
        );
        nodes.push(write_source(&self.source, &self.literal_types));
        nodes.extend(
            self.clock_places
                .iter()
                .map(|place| write_place(place, &self.literal_types))
                .collect::<Result<Vec<_>, _>>()?,
        );
        nodes.extend(self.resets.iter().map(write_reset));
        nodes.extend(
            self.guards
                .iter()
                .map(|guard| write_predicate(&guard.metadata, "clock_guard", &guard.predicate)),
        );
        nodes.extend(self.invariants.iter().map(|invariant| {
            write_predicate(&invariant.metadata, "time_invariant", &invariant.predicate)
        }));
        nodes.extend(
            self.urgencies
                .iter()
                .map(|urgency| write_urgency(urgency, &self.literal_types)),
        );
        nodes.extend(
            self.normalized_constraints
                .iter()
                .map(|constraint| write_constraint(constraint, &self.literal_types)),
        );
        nodes.push(write_root(self));
        Ok(nodes)
    }
}

impl TimedNodeMetadata {
    fn from_node(node: &CheckedSemanticNodeV2) -> Self {
        Self {
            node_id: node.node_id.clone(),
            schema_version: node.schema_version.clone(),
            semantic_type: node.semantic_type.clone(),
            dependencies: node.dependencies.clone(),
            occurrences: node.occurrences.clone(),
            recursion_group: node.recursion_group.clone(),
            nominal_identity_preimage: node.nominal_identity_preimage.clone(),
            declaration: node.declaration.clone(),
            owner: node.owner.clone(),
        }
    }

    fn node(&self, form: &str, body: Value) -> CheckedSemanticNodeV2 {
        CheckedSemanticNodeV2 {
            node_id: self.node_id.clone(),
            schema_version: self.schema_version.clone(),
            node_tag: "timed_model".into(),
            semantic_form: form.into(),
            semantic_type: self.semantic_type.clone(),
            dependencies: self.dependencies.clone(),
            occurrences: self.occurrences.clone(),
            recursion_group: self.recursion_group.clone(),
            nominal_identity_preimage: self.nominal_identity_preimage.clone(),
            declaration: self.declaration.clone(),
            owner: self.owner.clone(),
            body,
        }
    }
}

fn write_source(source: &CheckedTimeSource, types: &TimedLiteralTypes) -> CheckedSemanticNodeV2 {
    let mut fields = vec![
        binding_term("kind", text_literal(&types.text, &source.kind)),
        binding_term("unit", reference_term(&source.unit)),
    ];
    if let (Some(model), Some(operation), Some(period), Some(period_type)) = (
        &source.model,
        &source.operation,
        &source.period,
        &source.period_type,
    ) {
        fields.extend([
            binding_term("model", reference_term(model)),
            binding_term("operation", reference_term(operation)),
            binding_term(
                "period",
                typed_string_literal(period_type, "rational", period),
            ),
        ]);
    }
    source.metadata.node("time_source", aggregate_term(fields))
}

fn write_place(
    place: &CheckedClockPlace,
    types: &TimedLiteralTypes,
) -> Result<CheckedSemanticNodeV2, &'static str> {
    let path = place
        .path
        .iter()
        .map(|step| -> Result<Value, &'static str> {
            Ok(match step {
                ClockPathStep::Field(name) => aggregate_term(vec![
                    binding_term("kind", text_literal(&types.text, "field")),
                    binding_term("name", text_literal(&types.text, name)),
                ]),
                ClockPathStep::Tuple(index) => aggregate_term(vec![
                    binding_term("kind", text_literal(&types.text, "tuple")),
                    binding_term("index", integer_literal(types, *index)?),
                ]),
                ClockPathStep::Union(case) => aggregate_term(vec![
                    binding_term("kind", text_literal(&types.text, "union")),
                    binding_term("case", text_literal(&types.text, case)),
                ]),
                ClockPathStep::Slot {
                    collection,
                    selector,
                    minimum,
                    maximum,
                } => {
                    let selector = match selector {
                        ClockSlotSelector::Key(key) => reference_term(key),
                        ClockSlotSelector::Index(index) => integer_literal(types, *index)?,
                    };
                    let maximum = match maximum {
                        Some(value) => integer_literal(types, *value)?,
                        None => text_literal(&types.text, "unbounded"),
                    };
                    aggregate_term(vec![
                        binding_term("kind", text_literal(&types.text, "slot")),
                        binding_term("collection", text_literal(&types.text, collection)),
                        binding_term("selector", selector),
                        binding_term("minimum", integer_literal(types, *minimum)?),
                        binding_term("maximum", maximum),
                    ])
                }
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let resolved_path = place
        .resolved_path
        .iter()
        .map(|step| -> Result<Value, &'static str> {
            Ok(aggregate_term(vec![
                binding_term("path_step", integer_literal(types, step.path_step)?),
                binding_term(
                    "declaring_type",
                    text_literal(&types.text, &step.declaring_type),
                ),
                binding_term(
                    "member_identity",
                    text_literal(&types.text, &step.member_identity),
                ),
                binding_term("member_type", text_literal(&types.text, &step.member_type)),
                binding_term(
                    "collection_kind",
                    text_literal(&types.text, &step.collection_kind),
                ),
                binding_term("minimum", text_literal(&types.text, &step.minimum)),
                binding_term("maximum", text_literal(&types.text, &step.maximum)),
            ]))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(place.metadata.node(
        "clock_place",
        aggregate_term(vec![
            binding_term("model", reference_term(&place.model)),
            binding_term("path", aggregate_term(path)),
            binding_term("resolved_path", aggregate_term(resolved_path)),
            binding_term("guards", reference_terms(&place.guards)),
        ]),
    ))
}

fn write_reset(reset: &CheckedClockReset) -> CheckedSemanticNodeV2 {
    reset.metadata.node(
        "clock_reset",
        aggregate_term(vec![
            binding_term("place", reference_term(&reset.place)),
            binding_term(
                "value",
                typed_string_literal(&reset.value_type, "quantity", &reset.value),
            ),
        ]),
    )
}

fn write_predicate(
    metadata: &TimedNodeMetadata,
    form: &str,
    predicate: &CheckedNodeId,
) -> CheckedSemanticNodeV2 {
    metadata.node(
        form,
        aggregate_term(vec![binding_term("predicate", reference_term(predicate))]),
    )
}

fn write_urgency(urgency: &CheckedUrgency, types: &TimedLiteralTypes) -> CheckedSemanticNodeV2 {
    match urgency {
        CheckedUrgency::When {
            metadata,
            predicate,
        } => metadata.node(
            "urgency",
            aggregate_term(vec![
                binding_term("kind", text_literal(&types.text, "when")),
                binding_term("predicate", reference_term(predicate)),
            ]),
        ),
        CheckedUrgency::Operation {
            metadata,
            model,
            operation_name,
        } => metadata.node(
            "urgency",
            aggregate_term(vec![
                binding_term("kind", text_literal(&types.text, "operation")),
                binding_term("model", reference_term(model)),
                binding_term("operation_name", text_literal(&types.text, operation_name)),
            ]),
        ),
    }
}

fn write_constraint(
    constraint: &CheckedClockConstraint,
    types: &TimedLiteralTypes,
) -> CheckedSemanticNodeV2 {
    constraint.metadata.node(
        "clock_constraint",
        aggregate_term(vec![
            binding_term("place", reference_term(&constraint.place)),
            binding_term(
                "comparison",
                text_literal(&types.text, &constraint.comparison),
            ),
            binding_term(
                "bound",
                typed_string_literal(&constraint.bound_type, "quantity", &constraint.bound),
            ),
        ]),
    )
}

fn write_root(model: &CheckedTimedModel) -> CheckedSemanticNodeV2 {
    let references =
        |ids: Vec<&CheckedNodeId>| aggregate_term(ids.into_iter().map(reference_term).collect());
    model.metadata.node(
        "timed_model",
        aggregate_term(vec![
            binding_term("model", reference_term(&model.model)),
            binding_term("time_dimension", reference_term(&model.time_dimension)),
            binding_term("source", reference_term(&model.source.metadata.node_id)),
            binding_term(
                "clock_places",
                references(
                    model
                        .clock_places
                        .iter()
                        .map(|place| &place.metadata.node_id)
                        .collect(),
                ),
            ),
            binding_term(
                "resets",
                references(
                    model
                        .resets
                        .iter()
                        .map(|reset| &reset.metadata.node_id)
                        .collect(),
                ),
            ),
            binding_term(
                "guards",
                references(
                    model
                        .guards
                        .iter()
                        .map(|guard| &guard.metadata.node_id)
                        .collect(),
                ),
            ),
            binding_term(
                "invariants",
                references(
                    model
                        .invariants
                        .iter()
                        .map(|item| &item.metadata.node_id)
                        .collect(),
                ),
            ),
            binding_term(
                "urgencies",
                references(
                    model
                        .urgencies
                        .iter()
                        .map(urgency_metadata)
                        .map(|metadata| &metadata.node_id)
                        .collect(),
                ),
            ),
            binding_term(
                "constraints",
                references(
                    model
                        .normalized_constraints
                        .iter()
                        .map(|constraint| &constraint.metadata.node_id)
                        .collect(),
                ),
            ),
        ]),
    )
}

fn urgency_metadata(urgency: &CheckedUrgency) -> &TimedNodeMetadata {
    match urgency {
        CheckedUrgency::When { metadata, .. } | CheckedUrgency::Operation { metadata, .. } => {
            metadata
        }
    }
}

fn reference_term(id: &CheckedNodeId) -> Value {
    serde_json::json!({"term": "reference", "target": id})
}

fn reference_terms(ids: &[CheckedNodeId]) -> Value {
    aggregate_term(ids.iter().map(reference_term).collect())
}

fn binding_term(name: &str, value: Value) -> Value {
    serde_json::json!({"term": "binding", "name": name, "value": value})
}

fn aggregate_term(members: Vec<Value>) -> Value {
    serde_json::json!({"term": "aggregate", "members": members})
}

fn text_literal(type_id: &CheckedNodeId, value: &str) -> Value {
    typed_string_literal(type_id, "text", value)
}

fn typed_string_literal(type_id: &CheckedNodeId, kind: &str, value: &str) -> Value {
    serde_json::json!({
        "term": "literal",
        "type": type_id,
        "value_kind": kind,
        "value": value,
    })
}

fn integer_literal(types: &TimedLiteralTypes, value: u64) -> Result<Value, &'static str> {
    let type_id = types
        .integer
        .as_ref()
        .ok_or("timed model needs an integer literal type to write this clock path")?;
    Ok(serde_json::json!({
        "term": "literal",
        "type": type_id,
        "value_kind": "integer",
        "value": value,
    }))
}

fn project_root(
    root: &CheckedSemanticNodeV2,
    nodes: &BTreeMap<CheckedNodeId, &CheckedSemanticNodeV2>,
    literal_types: &TimedLiteralTypes,
) -> Result<CheckedTimedModel, &'static str> {
    let body = &root.body;
    let model = reference(binding(body, "model")?)?;
    let time_dimension = reference(binding(body, "time_dimension")?)?;
    let source_node = referenced_node(binding(body, "source")?, nodes)?;
    let source = project_source(source_node)?;
    let clock_places = reference_list(binding(body, "clock_places")?)?
        .into_iter()
        .map(|id| project_place(referenced_node_by_id(&id, nodes)?))
        .collect::<Result<Vec<_>, _>>()?;
    let resets = reference_list(binding(body, "resets")?)?
        .into_iter()
        .map(|id| project_reset(referenced_node_by_id(&id, nodes)?))
        .collect::<Result<Vec<_>, _>>()?;
    let guards = reference_list(binding(body, "guards")?)?
        .into_iter()
        .map(|id| project_predicate(referenced_node_by_id(&id, nodes)?, "clock_guard"))
        .collect::<Result<Vec<_>, _>>()?;
    let invariants = reference_list(binding(body, "invariants")?)?
        .into_iter()
        .map(|id| project_predicate(referenced_node_by_id(&id, nodes)?, "time_invariant"))
        .collect::<Result<Vec<_>, _>>()?;
    let urgencies = reference_list(binding(body, "urgencies")?)?
        .into_iter()
        .map(|id| project_urgency(referenced_node_by_id(&id, nodes)?))
        .collect::<Result<Vec<_>, _>>()?;
    let normalized_constraints = reference_list(binding(body, "constraints")?)?
        .into_iter()
        .map(|id| project_constraint(referenced_node_by_id(&id, nodes)?, source.unit.clone()))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(CheckedTimedModel {
        metadata: TimedNodeMetadata::from_node(root),
        model,
        time_dimension,
        source,
        clock_places,
        resets,
        guards,
        invariants,
        urgencies,
        normalized_constraints,
        literal_types: literal_types.clone(),
    })
}

fn project_source(node: &CheckedSemanticNodeV2) -> Result<CheckedTimeSource, &'static str> {
    if node.semantic_form.as_ref() != "time_source" {
        return Err("timed source reference has wrong form");
    }
    let body = &node.body;
    let kind = literal_text(binding(body, "kind")?)?.to_owned();
    let unit = reference(binding(body, "unit")?)?;
    let (model, operation, period) = if kind == "tick" {
        (
            Some(reference(binding(body, "model")?)?),
            Some(reference(binding(body, "operation")?)?),
            Some(normalized_rational(literal_string(binding(
                body, "period",
            )?)?)?),
        )
    } else if kind == "dense" {
        (None, None, None)
    } else {
        return Err("unknown timed source kind");
    };
    Ok(CheckedTimeSource {
        metadata: TimedNodeMetadata::from_node(node),
        kind,
        unit,
        model,
        operation,
        period,
        period_type: if node.semantic_form.as_ref() == "time_source"
            && binding(body, "period").is_ok()
        {
            Some(literal_type(binding(body, "period")?)?)
        } else {
            None
        },
    })
}

fn project_place(node: &CheckedSemanticNodeV2) -> Result<CheckedClockPlace, &'static str> {
    if node.semantic_form.as_ref() != "clock_place" {
        return Err("clock place reference has wrong form");
    }
    let body = &node.body;
    let path_value = binding(body, "path")?;
    let path = aggregate(path_value)?
        .iter()
        .map(project_path_step)
        .collect::<Result<Vec<_>, _>>()?;
    let resolved_path = aggregate(binding(body, "resolved_path")?)?
        .iter()
        .map(project_resolved_step)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(CheckedClockPlace {
        metadata: TimedNodeMetadata::from_node(node),
        path,
        resolved_path,
        model: reference(binding(body, "model")?)?,
        guards: reference_list(binding(body, "guards")?)?,
    })
}

fn project_path_step(value: &Value) -> Result<ClockPathStep, &'static str> {
    let kind = literal_text(binding(value, "kind")?)?;
    match kind {
        "field" => Ok(ClockPathStep::Field(
            literal_text(binding(value, "name")?)?.to_owned(),
        )),
        "tuple" => Ok(ClockPathStep::Tuple(literal_u64(binding(value, "index")?)?)),
        "union" => Ok(ClockPathStep::Union(
            literal_text(binding(value, "case")?)?.to_owned(),
        )),
        "slot" => {
            let selector = binding(value, "selector")?;
            let selector = if selector["term"] == "reference" {
                ClockSlotSelector::Key(reference(selector)?)
            } else {
                ClockSlotSelector::Index(literal_u64(selector)?)
            };
            let maximum_value = binding(value, "maximum")?;
            let maximum = if literal_text(maximum_value).ok() == Some("unbounded") {
                None
            } else {
                Some(literal_u64(maximum_value)?)
            };
            Ok(ClockPathStep::Slot {
                collection: literal_text(binding(value, "collection")?)?.to_owned(),
                selector,
                minimum: literal_u64(binding(value, "minimum")?)?,
                maximum,
            })
        }
        _ => Err("unknown clock path step"),
    }
}

fn project_resolved_step(value: &Value) -> Result<ResolvedClockPathStep, &'static str> {
    Ok(ResolvedClockPathStep {
        path_step: literal_u64(binding(value, "path_step")?)?,
        declaring_type: literal_text(binding(value, "declaring_type")?)?.to_owned(),
        member_identity: literal_text(binding(value, "member_identity")?)?.to_owned(),
        member_type: literal_text(binding(value, "member_type")?)?.to_owned(),
        collection_kind: literal_text(binding(value, "collection_kind")?)?.to_owned(),
        minimum: literal_text(binding(value, "minimum")?)?.to_owned(),
        maximum: literal_text(binding(value, "maximum")?)?.to_owned(),
    })
}

fn project_reset(node: &CheckedSemanticNodeV2) -> Result<CheckedClockReset, &'static str> {
    if node.semantic_form.as_ref() != "clock_reset" {
        return Err("clock reset reference has wrong form");
    }
    let body = &node.body;
    Ok(CheckedClockReset {
        metadata: TimedNodeMetadata::from_node(node),
        place: reference(binding(body, "place")?)?,
        value: normalized_rational(literal_string(binding(body, "value")?)?)?,
        value_type: literal_type(binding(body, "value")?)?,
    })
}

fn project_predicate(
    node: &CheckedSemanticNodeV2,
    expected_form: &str,
) -> Result<CheckedTimedPredicate, &'static str> {
    if node.semantic_form.as_ref() != expected_form {
        return Err("timed predicate reference has wrong form");
    }
    Ok(CheckedTimedPredicate {
        metadata: TimedNodeMetadata::from_node(node),
        predicate: reference(binding(&node.body, "predicate")?)?,
    })
}

fn project_urgency(node: &CheckedSemanticNodeV2) -> Result<CheckedUrgency, &'static str> {
    if node.semantic_form.as_ref() != "urgency" {
        return Err("urgency reference has wrong form");
    }
    let body = &node.body;
    match literal_text(binding(body, "kind")?)? {
        "when" => Ok(CheckedUrgency::When {
            metadata: TimedNodeMetadata::from_node(node),
            predicate: reference(binding(body, "predicate")?)?,
        }),
        "operation" => Ok(CheckedUrgency::Operation {
            metadata: TimedNodeMetadata::from_node(node),
            model: reference(binding(body, "model")?)?,
            operation_name: literal_text(binding(body, "operation_name")?)?.to_owned(),
        }),
        _ => Err("unknown urgency kind"),
    }
}

fn project_constraint(
    node: &CheckedSemanticNodeV2,
    unit: CheckedNodeId,
) -> Result<CheckedClockConstraint, &'static str> {
    if node.semantic_form.as_ref() != "clock_constraint" {
        return Err("clock constraint reference has wrong form");
    }
    let body = &node.body;
    Ok(CheckedClockConstraint {
        metadata: TimedNodeMetadata::from_node(node),
        place: reference(binding(body, "place")?)?,
        comparison: literal_text(binding(body, "comparison")?)?.to_owned(),
        bound: normalized_rational(literal_string(binding(body, "bound")?)?)?,
        unit,
        bound_type: literal_type(binding(body, "bound")?)?,
    })
}

fn referenced_node<'a>(
    value: &Value,
    nodes: &'a BTreeMap<CheckedNodeId, &'a CheckedSemanticNodeV2>,
) -> Result<&'a CheckedSemanticNodeV2, &'static str> {
    let id = reference(value)?;
    referenced_node_by_id(&id, nodes)
}

fn referenced_node_by_id<'a>(
    id: &CheckedNodeId,
    nodes: &'a BTreeMap<CheckedNodeId, &'a CheckedSemanticNodeV2>,
) -> Result<&'a CheckedSemanticNodeV2, &'static str> {
    nodes
        .get(id)
        .copied()
        .ok_or("admitted timed-model edge is absent")
}

fn binding<'a>(value: &'a Value, name: &str) -> Result<&'a Value, &'static str> {
    aggregate(value)?
        .iter()
        .find(|member| member["term"] == "binding" && member["name"] == name)
        .map(|member| &member["value"])
        .ok_or("admitted timed-model binding is absent")
}

fn aggregate(value: &Value) -> Result<&[Value], &'static str> {
    if value["term"] != "aggregate" {
        return Err("admitted timed-model term is not an aggregate");
    }
    value["members"]
        .as_array()
        .map(Vec::as_slice)
        .ok_or("aggregate has no members")
}

fn reference(value: &Value) -> Result<CheckedNodeId, &'static str> {
    if value["term"] != "reference" {
        return Err("admitted timed-model term is not a reference");
    }
    serde_json::from_value(value["target"].clone()).map_err(|_| "reference is malformed")
}

fn reference_list(value: &Value) -> Result<Vec<CheckedNodeId>, &'static str> {
    aggregate(value)?.iter().map(reference).collect()
}

fn literal_text(value: &Value) -> Result<&str, &'static str> {
    if value["term"] != "literal" || value["value_kind"] != "text" {
        return Err("admitted timed-model term is not text");
    }
    value["value"].as_str().ok_or("text literal is malformed")
}

fn literal_string(value: &Value) -> Result<&str, &'static str> {
    if value["term"] != "literal" {
        return Err("admitted timed-model term is not a literal");
    }
    value["value"].as_str().ok_or("literal string is malformed")
}

fn literal_type(value: &Value) -> Result<CheckedNodeId, &'static str> {
    if value["term"] != "literal" {
        return Err("admitted timed-model term is not a literal");
    }
    serde_json::from_value(value["type"].clone()).map_err(|_| "literal type is malformed")
}

fn literal_u64(value: &Value) -> Result<u64, &'static str> {
    if value["term"] != "literal" || value["value_kind"] != "integer" {
        return Err("admitted timed-model term is not an integer");
    }
    value["value"]
        .as_u64()
        .ok_or("integer literal is malformed")
}

fn normalized_rational(value: &str) -> Result<String, &'static str> {
    let (numerator, denominator) = value.split_once('/').unwrap_or((value, "1"));
    let numerator = Integer::from_str(numerator).map_err(|_| "rational numerator is malformed")?;
    let denominator =
        Integer::from_str(denominator).map_err(|_| "rational denominator is malformed")?;
    let rational =
        Rational::new(numerator, denominator).map_err(|_| "rational denominator is zero")?;
    Ok(format!(
        "{}/{}",
        rational.numerator(),
        rational.denominator()
    ))
}

fn find_literal_type(
    nodes: &BTreeMap<CheckedNodeId, &CheckedSemanticNodeV2>,
    kind: &str,
) -> Option<CheckedNodeId> {
    fn visit(value: &Value, kind: &str) -> Option<CheckedNodeId> {
        match value {
            Value::Object(object) => {
                if object.get("term").and_then(Value::as_str) == Some("literal")
                    && object.get("value_kind").and_then(Value::as_str) == Some(kind)
                {
                    return serde_json::from_value(object.get("type")?.clone()).ok();
                }
                object.values().find_map(|child| visit(child, kind))
            }
            Value::Array(values) => values.iter().find_map(|child| visit(child, kind)),
            _ => None,
        }
    }

    nodes.values().find_map(|node| visit(&node.body, kind))
}
