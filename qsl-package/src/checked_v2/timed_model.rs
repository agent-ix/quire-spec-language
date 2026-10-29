// SPDX-License-Identifier: AGPL-3.0-or-later
//! Typed projection of QSpec FR-322's admitted timed-model graph.

use std::collections::BTreeMap;
use std::str::FromStr;

use quire_contract_model::{
    CheckedDeclaration, CheckedNodeId, CheckedNodeOwner, CheckedNodeTag, CheckedOccurrence,
    CheckedOccurrenceRole, CheckedPackageV2, CheckedSemanticNodeV2, CheckedTerm, LiteralKind,
    NominalIdentityPreimage, TimedModelForm,
};
use quire_exact::{Integer, Rational};

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
    /// One guard for each guarded operation, in the root's declared order.
    pub guards: Vec<CheckedClockGuard>,
    /// Time invariants in the root's declared order.
    pub invariants: Vec<CheckedTimeInvariant>,
    /// Urgency declarations in the root's declared order.
    pub urgencies: Vec<CheckedUrgency>,
    /// Every constraint a guard part or an invariant names, in the root's
    /// declared order.
    pub constraints: Vec<CheckedClockConstraint>,
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

/// The comparison of a `clock_constraint` (QSpec FR-322 "Clause nodes"),
/// declared in the rank the aggregate order compares it by: `lt`, `le`,
/// `eq`, `ge`, `gt`.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ClockComparison {
    /// `lt`.
    Lt,
    /// `le`.
    Le,
    /// `eq`.
    Eq,
    /// `ge`.
    Ge,
    /// `gt`.
    Gt,
}

impl ClockComparison {
    /// The wire word.
    pub const fn as_wire(self) -> &'static str {
        match self {
            Self::Lt => "lt",
            Self::Le => "le",
            Self::Eq => "eq",
            Self::Ge => "ge",
            Self::Gt => "gt",
        }
    }

    /// The comparison a wire word names, `None` for any other word.
    // Intake proof: clause_intake_is_total_and_round_trips_the_clause_fixture.
    #[ix_trace_rs::string_edge]
    pub fn from_wire(word: &str) -> Option<Self> {
        Some(match word {
            "lt" => Self::Lt,
            "le" => Self::Le,
            "eq" => Self::Eq,
            "ge" => Self::Ge,
            "gt" => Self::Gt,
            _ => return None,
        })
    }
}

/// What a `clock_constraint` bounds (QSpec FR-322 "Clause nodes").
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CheckedConstraintTarget {
    /// Kind `place`: the clock of one instance of a template place.
    Place {
        /// The `clock_place` template.
        place: CheckedNodeId,
        /// The owner term, an expression of the place's owner type.
        owner: CheckedNodeId,
        /// One term for each `slot` step of the place's path.
        slots: Vec<CheckedNodeId>,
    },
    /// Kind `parameter`: the clock an operation's `Clock` parameter binds.
    Parameter {
        /// The no-point `state`/`operation_anchor` of the operation.
        operation: CheckedNodeId,
        /// The parameter's declared name.
        parameter: String,
    },
}

/// What a `clock_reset` writes (QSpec FR-322 "Clause nodes").
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CheckedResetTarget {
    /// Kind `place`: each instance of a template place the binders select
    /// where the condition holds.
    Place {
        /// The `clock_place` template.
        place: CheckedNodeId,
        /// The owner term.
        owner: CheckedNodeId,
        /// One term for each `slot` step of the place's path.
        slots: Vec<CheckedNodeId>,
        /// The `parameter` nodes the reset quantifies, each one of `slots`,
        /// in slot order.
        binders: Vec<CheckedNodeId>,
        /// The Boolean expression under which the reset happens.
        condition: CheckedNodeId,
    },
    /// Kind `parameter`: the clock an operation's `Clock` parameter binds.
    Parameter {
        /// The parameter's declared name.
        parameter: String,
    },
}

/// One part of a `clock_guard`: the constraints that hold in the operation
/// when the predicate does.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedGuardPart {
    /// The part's Boolean expression node.
    pub predicate: CheckedNodeId,
    /// The part's `clock_constraint` nodes, in aggregate order.
    pub constraints: Vec<CheckedNodeId>,
}

/// One operation's `clock_guard`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedClockGuard {
    /// The node's checked wire metadata.
    pub metadata: TimedNodeMetadata,
    /// The no-point `state`/`operation_anchor` of the guarded operation.
    pub operation: CheckedNodeId,
    /// The nonempty disjunction of parts.
    pub parts: Vec<CheckedGuardPart>,
}

/// One `time_invariant`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedTimeInvariant {
    /// The node's checked wire metadata.
    pub metadata: TimedNodeMetadata,
    /// The Boolean expression node where the bounds hold.
    pub predicate: CheckedNodeId,
    /// The nonempty upper bounds, `clock_constraint` nodes of kind `place`
    /// in aggregate order.
    pub bounds: Vec<CheckedNodeId>,
}

/// The closed kind of an admitted time source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckedTimeSourceKind {
    /// Continuous dense time.
    Dense,
    /// Discrete ticks from the selected model operation.
    Tick,
}

impl CheckedTimeSourceKind {
    /// The original checked-wire spelling.
    pub const fn as_wire(self) -> &'static str {
        match self {
            Self::Dense => "dense",
            Self::Tick => "tick",
        }
    }
}

/// A dense or tick time source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedTimeSource {
    /// The node's checked wire metadata.
    pub metadata: TimedNodeMetadata,
    /// The closed admitted source kind.
    pub kind: CheckedTimeSourceKind,
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
    /// A collection slot of a template place (QSpec FR-322 "Timed-model
    /// nodes"): every element's place is one instance of it.
    Slot {
        /// The collection kind.
        collection: String,
        /// The symbolic element selector: the type node of the selector's
        /// domain, `Integer` for a position and the key type for a `Map`.
        selector: CheckedNodeId,
        /// Declared lower bound.
        minimum: u64,
        /// Declared upper bound, or `None` when unbounded.
        maximum: Option<u64>,
    },
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
}

/// A checked clock reset with an exact normalized nonnegative value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckedClockReset {
    /// The node's checked wire metadata.
    pub metadata: TimedNodeMetadata,
    /// The no-point `state`/`operation_anchor` of the resetting operation.
    pub operation: CheckedNodeId,
    /// The reset clock.
    pub target: CheckedResetTarget,
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
    /// The constrained clock.
    pub target: CheckedConstraintTarget,
    /// The comparison.
    pub comparison: ClockComparison,
    /// The reduced rational bound.
    pub bound: String,
    /// Quantity type, which is the selected unit node.
    pub bound_type: CheckedNodeId,
}

impl CheckedClockPlace {
    /// The typed place of the emitted `clock_place` node `node`.
    pub(crate) fn project(node: &CheckedSemanticNodeV2) -> Result<Self, &'static str> {
        project_place(node)
    }
}

impl CheckedTimedModel {
    /// Construct the timed root and source represented by one S3 time
    /// declaration. The emitter rekeys both placeholders from their bodies.
    pub(crate) fn from_checked_time_source(
        source: &qsl_semantics::check::ModelTimeSource,
        source_id: CheckedNodeId,
        root_id: CheckedNodeId,
        owner: CheckedNodeOwner,
        text_type: CheckedNodeId,
        rational_type: CheckedNodeId,
        source_alias: &str,
    ) -> Option<Self> {
        let model = checked_id(source.model_node);
        let source_metadata = TimedNodeMetadata {
            node_id: source_id.clone(),
            schema_version: "quire.checked-semantic-graph/v2".into(),
            semantic_type: source_id,
            dependencies: Vec::new(),
            occurrences: vec![CheckedOccurrence {
                role: CheckedOccurrenceRole::Expression,
                ordinal: 0,
            }],
            recursion_group: None,
            nominal_identity_preimage: None,
            declaration: None,
            owner: None,
        };
        let (kind, model_ref, operation, period, period_type) = match &source.source {
            qsl_semantics::check::TimeSourceKind::Dense => {
                (CheckedTimeSourceKind::Dense, None, None, None, None)
            }
            qsl_semantics::check::TimeSourceKind::Tick { period, .. } => (
                CheckedTimeSourceKind::Tick,
                Some(model.clone()),
                Some(checked_id(source.tick_operation_anchor?)),
                Some(format!("{}/{}", period.numerator(), period.denominator())),
                Some(rational_type),
            ),
        };
        let timed_source = CheckedTimeSource {
            metadata: source_metadata,
            kind,
            unit: checked_id(source.unit_node),
            model: model_ref,
            operation,
            period,
            period_type,
        };
        let root_metadata = TimedNodeMetadata {
            node_id: root_id.clone(),
            schema_version: "quire.checked-semantic-graph/v2".into(),
            semantic_type: root_id,
            dependencies: Vec::new(),
            occurrences: vec![CheckedOccurrence {
                role: CheckedOccurrenceRole::Declaration,
                ordinal: 0,
            }],
            recursion_group: None,
            nominal_identity_preimage: None,
            declaration: Some(CheckedDeclaration {
                qualified_name: vec![source_alias.into(), "Timed".into()],
            }),
            owner: Some(owner),
        };
        Some(Self {
            metadata: root_metadata,
            model: model.clone(),
            time_dimension: checked_id(source.time_dimension),
            source: timed_source,
            clock_places: Vec::new(),
            resets: Vec::new(),
            guards: Vec::new(),
            invariants: Vec::new(),
            urgencies: Vec::new(),
            constraints: Vec::new(),
            literal_types: TimedLiteralTypes {
                text: text_type,
                integer: None,
            },
        })
    }

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
            .filter(|node| {
                CheckedNodeTag::from_wire(&node.node_tag) == Some(CheckedNodeTag::TimedModel)
            })
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
            .filter(|node| {
                CheckedNodeTag::from_wire(&node.node_tag) == Some(CheckedNodeTag::TimedModel)
            })
            .map(|node| (node.node_id.clone(), node))
            .collect::<BTreeMap<_, _>>();
        let mut roots = nodes
            .values()
            .filter(|node| {
                TimedModelForm::from_wire(&node.semantic_form) == Some(TimedModelForm::Model)
            })
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
        let types = &self.literal_types;
        let mut nodes = Vec::with_capacity(
            2 + self.clock_places.len()
                + self.resets.len()
                + self.guards.len()
                + self.invariants.len()
                + self.urgencies.len()
                + self.constraints.len(),
        );
        nodes.push(write_source(&self.source, types));
        for place in &self.clock_places {
            nodes.push(write_place(place, types)?);
        }
        nodes.extend(self.resets.iter().map(|reset| write_reset(reset, types)));
        nodes.extend(self.guards.iter().map(write_guard));
        nodes.extend(self.invariants.iter().map(write_invariant));
        nodes.extend(
            self.urgencies
                .iter()
                .map(|urgency| write_urgency(urgency, types)),
        );
        nodes.extend(
            self.constraints
                .iter()
                .map(|constraint| write_constraint(constraint, types)),
        );
        nodes.push(write_root(self));
        Ok(nodes)
    }
}

fn checked_id(key: quire_exact::NodeKey) -> CheckedNodeId {
    CheckedNodeId {
        domain: quire_exact::NODE_KEY_DOMAIN.into(),
        digest: key.to_string().into(),
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

    fn node(&self, form: &str, body: CheckedTerm) -> CheckedSemanticNodeV2 {
        CheckedSemanticNodeV2 {
            owner_model: None,
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
        binding_term("kind", text_literal(&types.text, source.kind.as_wire())),
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
                CheckedTerm::literal_rational(period_type.clone(), period.as_str()),
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
        .map(|step| -> Result<CheckedTerm, &'static str> {
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
                    let selector = reference_term(selector);
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
        .map(|step| -> Result<CheckedTerm, &'static str> {
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
        ]),
    ))
}

fn write_reset(reset: &CheckedClockReset, types: &TimedLiteralTypes) -> CheckedSemanticNodeV2 {
    let value = binding_term(
        "value",
        CheckedTerm::literal_quantity(reset.value_type.clone(), reset.value.as_str()),
    );
    let operation = binding_term("operation", reference_term(&reset.operation));
    let members = match &reset.target {
        CheckedResetTarget::Place {
            place,
            owner,
            slots,
            binders,
            condition,
        } => vec![
            binding_term("kind", text_literal(&types.text, "place")),
            operation,
            binding_term("place", reference_term(place)),
            binding_term("owner", reference_term(owner)),
            binding_term("slots", reference_terms(slots)),
            binding_term("binders", reference_terms(binders)),
            binding_term("condition", reference_term(condition)),
            value,
        ],
        CheckedResetTarget::Parameter { parameter } => vec![
            binding_term("kind", text_literal(&types.text, "parameter")),
            operation,
            binding_term("parameter", text_literal(&types.text, parameter)),
            value,
        ],
    };
    reset.metadata.node("clock_reset", aggregate_term(members))
}

fn write_guard(guard: &CheckedClockGuard) -> CheckedSemanticNodeV2 {
    let parts = guard
        .parts
        .iter()
        .map(|part| {
            aggregate_term(
                std::iter::once(binding_term("predicate", reference_term(&part.predicate)))
                    .chain(part.constraints.iter().map(reference_term))
                    .collect(),
            )
        })
        .collect();
    guard.metadata.node(
        "clock_guard",
        aggregate_term(vec![
            binding_term("operation", reference_term(&guard.operation)),
            binding_term("parts", aggregate_term(parts)),
        ]),
    )
}

fn write_invariant(invariant: &CheckedTimeInvariant) -> CheckedSemanticNodeV2 {
    invariant.metadata.node(
        "time_invariant",
        aggregate_term(vec![
            binding_term("predicate", reference_term(&invariant.predicate)),
            binding_term("bounds", reference_terms(&invariant.bounds)),
        ]),
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
    let tail = [
        binding_term(
            "comparison",
            text_literal(&types.text, constraint.comparison.as_wire()),
        ),
        binding_term(
            "bound",
            CheckedTerm::literal_quantity(constraint.bound_type.clone(), constraint.bound.as_str()),
        ),
    ];
    let head = match &constraint.target {
        CheckedConstraintTarget::Place {
            place,
            owner,
            slots,
        } => vec![
            binding_term("kind", text_literal(&types.text, "place")),
            binding_term("place", reference_term(place)),
            binding_term("owner", reference_term(owner)),
            binding_term("slots", reference_terms(slots)),
        ],
        CheckedConstraintTarget::Parameter {
            operation,
            parameter,
        } => vec![
            binding_term("kind", text_literal(&types.text, "parameter")),
            binding_term("operation", reference_term(operation)),
            binding_term("parameter", text_literal(&types.text, parameter)),
        ],
    };
    constraint.metadata.node(
        "clock_constraint",
        aggregate_term(head.into_iter().chain(tail).collect()),
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
                        .constraints
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

fn reference_term(id: &CheckedNodeId) -> CheckedTerm {
    CheckedTerm::reference(id.clone())
}

fn reference_terms(ids: &[CheckedNodeId]) -> CheckedTerm {
    aggregate_term(ids.iter().map(reference_term).collect())
}

fn binding_term(name: &str, value: CheckedTerm) -> CheckedTerm {
    CheckedTerm::binding(name, value)
}

fn aggregate_term(members: Vec<CheckedTerm>) -> CheckedTerm {
    CheckedTerm::aggregate(members)
}

fn text_literal(type_id: &CheckedNodeId, value: &str) -> CheckedTerm {
    CheckedTerm::literal_text(type_id.clone(), value)
}

fn integer_literal(types: &TimedLiteralTypes, value: u64) -> Result<CheckedTerm, &'static str> {
    let type_id = types
        .integer
        .as_ref()
        .ok_or("timed model needs an integer literal type to write this clock path")?;
    Ok(CheckedTerm::literal_integer(type_id.clone(), value))
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
        .map(|id| project_guard(referenced_node_by_id(&id, nodes)?))
        .collect::<Result<Vec<_>, _>>()?;
    let invariants = reference_list(binding(body, "invariants")?)?
        .into_iter()
        .map(|id| project_invariant(referenced_node_by_id(&id, nodes)?))
        .collect::<Result<Vec<_>, _>>()?;
    let urgencies = reference_list(binding(body, "urgencies")?)?
        .into_iter()
        .map(|id| project_urgency(referenced_node_by_id(&id, nodes)?))
        .collect::<Result<Vec<_>, _>>()?;
    let constraints = reference_list(binding(body, "constraints")?)?
        .into_iter()
        .map(|id| project_constraint(referenced_node_by_id(&id, nodes)?))
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
        constraints,
        literal_types: literal_types.clone(),
    })
}

// Intake proof: time_source_intake_is_total_and_preserves_dense_and_tick_wire.
// The wire literal is consumed once into CheckedTimeSourceKind; unknown or
// mistyped payloads refuse instead of escaping as a dispatch string.
#[ix_trace_rs::string_edge]
fn project_source(node: &CheckedSemanticNodeV2) -> Result<CheckedTimeSource, &'static str> {
    if TimedModelForm::from_wire(&node.semantic_form) != Some(TimedModelForm::Source) {
        return Err("timed source reference has wrong form");
    }
    let body = &node.body;
    let kind = literal_text(binding(body, "kind")?)?;
    let unit = reference(binding(body, "unit")?)?;
    let (kind, model, operation, period) = match kind {
        "tick" => (
            CheckedTimeSourceKind::Tick,
            Some(reference(binding(body, "model")?)?),
            Some(reference(binding(body, "operation")?)?),
            Some(normalized_rational(literal_string(binding(
                body, "period",
            )?)?)?),
        ),
        "dense" => (CheckedTimeSourceKind::Dense, None, None, None),
        _ => return Err("unknown timed source kind"),
    };
    Ok(CheckedTimeSource {
        metadata: TimedNodeMetadata::from_node(node),
        kind,
        unit,
        model,
        operation,
        period,
        period_type: if binding(body, "period").is_ok() {
            Some(literal_type(binding(body, "period")?)?)
        } else {
            None
        },
    })
}

fn project_place(node: &CheckedSemanticNodeV2) -> Result<CheckedClockPlace, &'static str> {
    if TimedModelForm::from_wire(&node.semantic_form) != Some(TimedModelForm::Place) {
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
    })
}

// Intake proof: clock_path_intake_is_total_for_each_closed_step_and_rejects_unknown_or_mistyped.
// Every accepted wire kind becomes a ClockPathStep variant; unknown kinds refuse.
#[ix_trace_rs::string_edge]
fn project_path_step(value: &CheckedTerm) -> Result<ClockPathStep, &'static str> {
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
            // The selector is symbolic; an index literal is no template
            // step (QSpec FR-322-AC-148).
            let selector = reference(binding(value, "selector")?)?;
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

fn project_resolved_step(value: &CheckedTerm) -> Result<ResolvedClockPathStep, &'static str> {
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

// Intake proof: clause_intake_is_total_and_round_trips_the_clause_fixture.
// The `kind` word is consumed once into CheckedResetTarget; an unknown kind
// refuses.
#[ix_trace_rs::string_edge]
fn project_reset(node: &CheckedSemanticNodeV2) -> Result<CheckedClockReset, &'static str> {
    if TimedModelForm::from_wire(&node.semantic_form) != Some(TimedModelForm::Reset) {
        return Err("clock reset reference has wrong form");
    }
    let body = &node.body;
    let target = match literal_text(binding(body, "kind")?)? {
        "place" => CheckedResetTarget::Place {
            place: reference(binding(body, "place")?)?,
            owner: reference(binding(body, "owner")?)?,
            slots: reference_list(binding(body, "slots")?)?,
            binders: reference_list(binding(body, "binders")?)?,
            condition: reference(binding(body, "condition")?)?,
        },
        "parameter" => CheckedResetTarget::Parameter {
            parameter: literal_text(binding(body, "parameter")?)?.to_owned(),
        },
        _ => return Err("unknown clock reset kind"),
    };
    Ok(CheckedClockReset {
        metadata: TimedNodeMetadata::from_node(node),
        operation: reference(binding(body, "operation")?)?,
        target,
        value: normalized_rational(literal_string(binding(body, "value")?)?)?,
        value_type: literal_type(binding(body, "value")?)?,
    })
}

fn project_guard(node: &CheckedSemanticNodeV2) -> Result<CheckedClockGuard, &'static str> {
    if TimedModelForm::from_wire(&node.semantic_form) != Some(TimedModelForm::Guard) {
        return Err("clock guard reference has wrong form");
    }
    let body = &node.body;
    let parts = aggregate(binding(body, "parts")?)?
        .iter()
        .map(|part| {
            let (predicate, constraints) = aggregate(part)?
                .split_first()
                .ok_or("a guard part is empty")?;
            let CheckedTerm::Binding { name, value } = predicate else {
                return Err("a guard part does not begin with its predicate");
            };
            if name.as_ref() != "predicate" {
                return Err("a guard part does not begin with its predicate");
            }
            Ok(CheckedGuardPart {
                predicate: reference(value)?,
                constraints: constraints
                    .iter()
                    .map(reference)
                    .collect::<Result<_, _>>()?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(CheckedClockGuard {
        metadata: TimedNodeMetadata::from_node(node),
        operation: reference(binding(body, "operation")?)?,
        parts,
    })
}

fn project_invariant(node: &CheckedSemanticNodeV2) -> Result<CheckedTimeInvariant, &'static str> {
    if TimedModelForm::from_wire(&node.semantic_form) != Some(TimedModelForm::Invariant) {
        return Err("time invariant reference has wrong form");
    }
    let body = &node.body;
    Ok(CheckedTimeInvariant {
        metadata: TimedNodeMetadata::from_node(node),
        predicate: reference(binding(body, "predicate")?)?,
        bounds: reference_list(binding(body, "bounds")?)?,
    })
}

// Intake proof: urgency_intake_is_total_and_preserves_both_variants_or_refuses_unknown.
// No wire kind escapes the CheckedUrgency variant conversion.
#[ix_trace_rs::string_edge]
fn project_urgency(node: &CheckedSemanticNodeV2) -> Result<CheckedUrgency, &'static str> {
    if TimedModelForm::from_wire(&node.semantic_form) != Some(TimedModelForm::Urgency) {
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

// Intake proof: clause_intake_is_total_and_round_trips_the_clause_fixture.
// The `kind` word is consumed once into CheckedConstraintTarget and the
// comparison once into ClockComparison; unknown words refuse.
#[ix_trace_rs::string_edge]
fn project_constraint(
    node: &CheckedSemanticNodeV2,
) -> Result<CheckedClockConstraint, &'static str> {
    if TimedModelForm::from_wire(&node.semantic_form) != Some(TimedModelForm::Constraint) {
        return Err("clock constraint reference has wrong form");
    }
    let body = &node.body;
    let target = match literal_text(binding(body, "kind")?)? {
        "place" => CheckedConstraintTarget::Place {
            place: reference(binding(body, "place")?)?,
            owner: reference(binding(body, "owner")?)?,
            slots: reference_list(binding(body, "slots")?)?,
        },
        "parameter" => CheckedConstraintTarget::Parameter {
            operation: reference(binding(body, "operation")?)?,
            parameter: literal_text(binding(body, "parameter")?)?.to_owned(),
        },
        _ => return Err("unknown clock constraint kind"),
    };
    Ok(CheckedClockConstraint {
        metadata: TimedNodeMetadata::from_node(node),
        target,
        comparison: ClockComparison::from_wire(literal_text(binding(body, "comparison")?)?)
            .ok_or("unknown clock comparison")?,
        bound: normalized_rational(literal_string(binding(body, "bound")?)?)?,
        bound_type: literal_type(binding(body, "bound")?)?,
    })
}

fn referenced_node<'a>(
    value: &CheckedTerm,
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

fn binding<'a>(value: &'a CheckedTerm, name: &str) -> Result<&'a CheckedTerm, &'static str> {
    aggregate(value)?
        .iter()
        .find_map(|member| match member {
            CheckedTerm::Binding {
                name: member_name,
                value,
            } if member_name.as_ref() == name => Some(value.as_ref()),
            _ => None,
        })
        .ok_or("admitted timed-model binding is absent")
}

fn aggregate(value: &CheckedTerm) -> Result<&[CheckedTerm], &'static str> {
    match value {
        CheckedTerm::Aggregate { members } => Ok(members),
        _ => Err("admitted timed-model term is not an aggregate"),
    }
}

fn reference(value: &CheckedTerm) -> Result<CheckedNodeId, &'static str> {
    match value {
        CheckedTerm::Reference { target } => Ok(target.clone()),
        _ => Err("admitted timed-model term is not a reference"),
    }
}

fn reference_list(value: &CheckedTerm) -> Result<Vec<CheckedNodeId>, &'static str> {
    aggregate(value)?.iter().map(reference).collect()
}

fn literal_text(value: &CheckedTerm) -> Result<&str, &'static str> {
    match value {
        CheckedTerm::Literal {
            value_kind: LiteralKind::Text,
            value,
            ..
        } => value.as_str().ok_or("text literal is malformed"),
        _ => Err("admitted timed-model term is not text"),
    }
}

fn literal_string(value: &CheckedTerm) -> Result<&str, &'static str> {
    match value {
        CheckedTerm::Literal { value, .. } => value.as_str().ok_or("literal string is malformed"),
        _ => Err("admitted timed-model term is not a literal"),
    }
}

fn literal_type(value: &CheckedTerm) -> Result<CheckedNodeId, &'static str> {
    match value {
        CheckedTerm::Literal { type_id, .. } => Ok(type_id.clone()),
        _ => Err("admitted timed-model term is not a literal"),
    }
}

fn literal_u64(value: &CheckedTerm) -> Result<u64, &'static str> {
    match value {
        CheckedTerm::Literal {
            value_kind: LiteralKind::Integer,
            value,
            ..
        } => value.as_u64().ok_or("integer literal is malformed"),
        _ => Err("admitted timed-model term is not an integer"),
    }
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
    fn visit(value: &CheckedTerm, kind: &str) -> Option<CheckedNodeId> {
        match value {
            CheckedTerm::Literal {
                type_id,
                value_kind,
                ..
            } if value_kind.as_wire() == kind => Some(type_id.clone()),
            CheckedTerm::Aggregate { members } => {
                members.iter().find_map(|child| visit(child, kind))
            }
            CheckedTerm::Binding { value, .. } => visit(value, kind),
            CheckedTerm::Application { arguments, .. } => {
                arguments.iter().find_map(|child| visit(child, kind))
            }
            CheckedTerm::ExtensionForm { clauses, .. } => {
                clauses.iter().find_map(|child| visit(child, kind))
            }
            _ => None,
        }
    }
    nodes.values().find_map(|node| visit(&node.body, kind))
}

#[cfg(test)]
mod intake_proofs {
    use super::*;

    fn fixture_node(form: &str) -> CheckedSemanticNodeV2 {
        let directory = std::env::var_os("QSPEC_DIR").expect("QSPEC_DIR supplies owner fixtures");
        let fixture = if form == "time_source" {
            "positive-tick-timed-model-nodes.json"
        } else {
            "positive-timed-model-nodes.json"
        };
        let path = std::path::Path::new(&directory)
            .join("proposals/checked-package-v2/fixtures")
            .join(fixture);
        let bytes = std::fs::read(path).expect("the published timed-model fixture is readable");
        let package: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let node = package["semantic_graph"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["semantic_form"] == form)
            .unwrap()
            .clone();
        serde_json::from_value(node).unwrap()
    }

    fn replace_binding(body: &mut CheckedTerm, name: &str, replacement: CheckedTerm) {
        let CheckedTerm::Aggregate { members } = body else {
            panic!("fixture body is aggregate")
        };
        let value = members
            .iter_mut()
            .find_map(|member| match member {
                CheckedTerm::Binding {
                    name: actual,
                    value,
                } if actual.as_ref() == name => Some(value),
                _ => None,
            })
            .expect("fixture binding exists");
        **value = replacement;
    }

    /// The graph nodes of the published fixture `name`.
    fn fixture_nodes(name: &str) -> Vec<CheckedSemanticNodeV2> {
        let directory = std::env::var_os("QSPEC_DIR").expect("QSPEC_DIR supplies owner fixtures");
        let path = std::path::Path::new(&directory)
            .join("proposals/checked-package-v2/fixtures")
            .join(name);
        let bytes = std::fs::read(path).expect("the published fixture is readable");
        let package: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        serde_json::from_value(package["semantic_graph"]["nodes"].clone()).unwrap()
    }

    /// QSpec FR-322-AC-261, FR-322-AC-266 (TC-367 CK-32): the clause
    /// fixture's closed forms, every clause in both kinds, reconstruct as
    /// one typed model, and the writer gives back every timed node of the
    /// fixture exactly; an unknown kind or comparison word refuses.
    #[ix_trace_rs::trace("QSpec-FR-322-AC-261", "QSpec-FR-322-AC-266")]
    #[test]
    fn clause_intake_is_total_and_round_trips_the_clause_fixture() {
        let nodes = fixture_nodes("positive-timed-clause-nodes.json");
        let models = CheckedTimedModel::from_nodes(&nodes).expect("the clause fixture projects");
        let [model] = models.as_slice() else {
            panic!("one timed model");
        };
        assert_eq!(model.clock_places.len(), 9);
        assert_eq!(model.resets.len(), 5);
        assert_eq!(model.guards.len(), 2);
        assert_eq!(model.invariants.len(), 2);
        assert_eq!(model.urgencies.len(), 2);
        assert_eq!(model.constraints.len(), 7);
        let parameter_resets = model
            .resets
            .iter()
            .filter(|reset| matches!(reset.target, CheckedResetTarget::Parameter { .. }))
            .count();
        assert_eq!(parameter_resets, 1);
        let parameter_constraints = model
            .constraints
            .iter()
            .filter(|constraint| {
                matches!(constraint.target, CheckedConstraintTarget::Parameter { .. })
            })
            .count();
        assert_eq!(parameter_constraints, 1);
        assert_eq!(
            model.guards[0]
                .parts
                .iter()
                .map(|part| part.constraints.len())
                .collect::<Vec<_>>(),
            [2, 1]
        );
        let written = model.write_nodes().expect("the clause model writes");
        let timed: Vec<&CheckedSemanticNodeV2> = nodes
            .iter()
            .filter(|node| node.node_tag.as_ref() == "timed_model")
            .collect();
        assert_eq!(written.len(), timed.len());
        for node in &written {
            let published = timed
                .iter()
                .find(|published| published.node_id == node.node_id)
                .unwrap_or_else(|| panic!("{} is a published node", node.semantic_form));
            assert_eq!(
                serde_json::to_value(node).unwrap(),
                serde_json::to_value(published).unwrap(),
                "{} round trips",
                node.semantic_form
            );
        }
        let text = |node: &CheckedSemanticNodeV2| {
            literal_type(binding(&node.body, "kind").unwrap()).unwrap()
        };
        let mut reset = timed
            .iter()
            .find(|node| node.semantic_form.as_ref() == "clock_reset")
            .map(|node| (*node).clone())
            .unwrap();
        let text_type = text(&reset);
        replace_binding(
            &mut reset.body,
            "kind",
            CheckedTerm::literal_text(text_type.clone(), "other"),
        );
        assert_eq!(project_reset(&reset), Err("unknown clock reset kind"));
        let mut constraint = timed
            .iter()
            .find(|node| node.semantic_form.as_ref() == "clock_constraint")
            .map(|node| (*node).clone())
            .unwrap();
        replace_binding(
            &mut constraint.body,
            "comparison",
            CheckedTerm::literal_text(text_type.clone(), "ne"),
        );
        assert_eq!(
            project_constraint(&constraint),
            Err("unknown clock comparison")
        );
        replace_binding(
            &mut constraint.body,
            "kind",
            CheckedTerm::literal_text(text_type, "other"),
        );
        assert_eq!(
            project_constraint(&constraint),
            Err("unknown clock constraint kind")
        );
    }

    #[test]
    fn time_source_intake_is_total_and_preserves_dense_and_tick_wire() {
        let tick = fixture_node("time_source");
        let projected = project_source(&tick).unwrap();
        assert_eq!(projected.kind, CheckedTimeSourceKind::Tick);
        let text_type = literal_type(binding(&tick.body, "kind").unwrap()).unwrap();
        let types = TimedLiteralTypes {
            text: text_type.clone(),
            integer: None,
        };
        assert_eq!(write_source(&projected, &types).body, tick.body);
        let mut dense = tick.clone();
        replace_binding(
            &mut dense.body,
            "kind",
            CheckedTerm::literal_text(text_type.clone(), "dense"),
        );
        let projected = project_source(&dense).unwrap();
        assert_eq!(projected.kind, CheckedTimeSourceKind::Dense);
        assert!(
            projected.model.is_none()
                && projected.operation.is_none()
                && projected.period.is_none()
        );
        let mut unknown = tick.clone();
        replace_binding(
            &mut unknown.body,
            "kind",
            CheckedTerm::literal_text(text_type.clone(), "other"),
        );
        assert_eq!(project_source(&unknown), Err("unknown timed source kind"));
        replace_binding(
            &mut unknown.body,
            "kind",
            CheckedTerm::literal_integer(text_type, 0),
        );
        assert_eq!(
            project_source(&unknown),
            Err("admitted timed-model term is not text")
        );
        let CheckedTerm::Aggregate { members } = &mut unknown.body else {
            panic!("fixture body is aggregate")
        };
        members.retain(|member| !matches!(member, CheckedTerm::Binding { name, .. } if name.as_ref() == "unit"));
        assert_eq!(
            project_source(&unknown),
            Err("admitted timed-model term is not text"),
            "kind decoding still precedes the unit lookup"
        );
    }

    #[test]
    fn clock_path_intake_is_total_for_each_closed_step_and_rejects_unknown_or_mistyped() {
        let source = fixture_node("time_source");
        let text_type = literal_type(binding(&source.body, "kind").unwrap()).unwrap();
        let text = |value: &str| CheckedTerm::literal_text(text_type.clone(), value);
        let integer = |value| CheckedTerm::literal_integer(text_type.clone(), value);
        let integer_type = || checked_id(qsl_semantics::check::builtin_integer_type_key());
        let step = |kind: &str, name: &str, value| {
            CheckedTerm::aggregate(vec![
                CheckedTerm::binding("kind", text(kind)),
                CheckedTerm::binding(name, value),
            ])
        };
        assert_eq!(
            project_path_step(&step("field", "name", text("clock"))),
            Ok(ClockPathStep::Field("clock".into()))
        );
        assert_eq!(
            project_path_step(&step("tuple", "index", integer(2))),
            Ok(ClockPathStep::Tuple(2))
        );
        assert_eq!(
            project_path_step(&step("union", "case", text("open"))),
            Ok(ClockPathStep::Union("open".into()))
        );
        let slot = |maximum| {
            CheckedTerm::aggregate(vec![
                CheckedTerm::binding("kind", text("slot")),
                CheckedTerm::binding("collection", text("sequence")),
                CheckedTerm::binding("selector", CheckedTerm::reference(integer_type())),
                CheckedTerm::binding("minimum", integer(0)),
                CheckedTerm::binding("maximum", maximum),
            ])
        };
        // An index literal is no symbolic selector (QSpec FR-322-AC-148).
        let indexed = CheckedTerm::aggregate(vec![
            CheckedTerm::binding("kind", text("slot")),
            CheckedTerm::binding("collection", text("sequence")),
            CheckedTerm::binding("selector", integer(1)),
            CheckedTerm::binding("minimum", integer(0)),
            CheckedTerm::binding("maximum", integer(3)),
        ]);
        assert!(project_path_step(&indexed).is_err());
        for (maximum, expected) in [(integer(3), Some(3)), (text("unbounded"), None)] {
            assert_eq!(
                project_path_step(&slot(maximum)),
                Ok(ClockPathStep::Slot {
                    collection: "sequence".into(),
                    selector: integer_type(),
                    minimum: 0,
                    maximum: expected,
                })
            );
        }
        assert_eq!(
            project_path_step(&step("other", "name", text("clock"))),
            Err("unknown clock path step")
        );
        assert_eq!(
            project_path_step(&step("tuple", "index", text("2"))),
            Err("admitted timed-model term is not an integer")
        );
    }

    #[test]
    fn urgency_intake_is_total_and_preserves_both_variants_or_refuses_unknown() {
        let source = fixture_node("time_source");
        let mut node = fixture_node("urgency");
        let type_id = literal_type(binding(&source.body, "kind").unwrap()).unwrap();
        let text = |value: &str| CheckedTerm::literal_text(type_id.clone(), value);
        let target = reference(binding(&source.body, "model").unwrap()).unwrap();
        node.body = CheckedTerm::aggregate(vec![
            CheckedTerm::binding("kind", text("when")),
            CheckedTerm::binding("predicate", CheckedTerm::reference(target.clone())),
        ]);
        assert!(
            matches!(project_urgency(&node), Ok(CheckedUrgency::When { predicate, .. }) if predicate == target)
        );
        node.body = CheckedTerm::aggregate(vec![
            CheckedTerm::binding("kind", text("operation")),
            CheckedTerm::binding("model", CheckedTerm::reference(target.clone())),
            CheckedTerm::binding("operation_name", text("advance")),
        ]);
        assert!(
            matches!(project_urgency(&node), Ok(CheckedUrgency::Operation { model, operation_name, .. }) if model == target && operation_name == "advance")
        );
        replace_binding(&mut node.body, "kind", text("other"));
        assert_eq!(project_urgency(&node), Err("unknown urgency kind"));
        replace_binding(
            &mut node.body,
            "kind",
            CheckedTerm::literal_integer(type_id, 0),
        );
        assert_eq!(
            project_urgency(&node),
            Err("admitted timed-model term is not text")
        );
    }
}
