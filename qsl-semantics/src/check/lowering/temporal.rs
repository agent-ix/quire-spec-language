// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-337: the `temporal`/`temporal_clause` node each emitted temporal
//! clause lowers to, with the `temporal`/`formula` node of each formula
//! node and the `temporal`/`fairness` node of each fairness set member
//! (QSpec FR-370).
//!
//! The clause node's body is one `quire.op.temporal.clause` application
//! carrying the clause's `temporal_profile` law and the six arguments over,
//! clock, activation, captures, fairness and formula. Each temporal
//! operator is an application of its own `quire.op.temporal.*` identity,
//! its operands references to their own formula nodes, so every body stays
//! in FR-322's flat body grammar whatever the formula's depth; a `holds`
//! atom's operand is its condition, lowered as FR-093 lowers a Boolean
//! expression. Every node is keyed by FR-322's application-node rule over
//! its content alone: the clause's name enters no key, so two structurally
//! equal clauses share one node and differ only in their `claim`
//! occurrence (FR-337 "Outputs").
//!
//! A clause names no position of its own beyond its declaration, so each
//! node is located at an `Origin::TypeDeclaration` site of the clause
//! ([`temporal_site`]): the clause's name for its `claim`, the `over`
//! parameter's name, each formula node's span, each fairness constraint's
//! span, and each `holds` condition's own expression tree.

use std::collections::BTreeMap;

use quire_exact::{Identifier, NodeKey, ValueType};
use quire_semantic_value::location::{Location, Origin};

use super::{fault, Binder, Binders, Lowering};
use crate::check::family::OccurrenceRole;
use crate::check::ir::CheckedNode;
use crate::check::node_key::{
    Binding, BodyTerm, GroupMember, GroupTerm, LawRole, LeafTerm, MemberTerm, NodeTag, Operation,
    OperationLaw, Operator,
};
use crate::check::refusal::{CheckRefusal, KeyFault};
use crate::check::temporal_clause::condition::TypedConditions;
use crate::check::temporal_clause::{
    BinaryTemporal, CheckedFairnessMember, CheckedFormula, CheckedInterval, CheckedTemporalClause,
    FormulaNode, FormulaNodeId, UnaryTemporal,
};
use crate::value::member::{FairnessTarget, Member, TimedMember};

/// The stage name this module's faults report.
const STAGE: &str = "lowering.temporal";

/// One place of a temporal clause an occurrence or a refusal is located at.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TemporalSite {
    /// The clause's declared name: its `claim` occurrence.
    Claim,
    /// The `over` parameter's name.
    Over,
    /// The `on each` trigger binder's name.
    Trigger,
    /// The `on each` activation's `when` guard: its expression tree.
    Guard,
    /// The capture at this index: its declared name.
    Capture(usize),
    /// The initializer of the capture at this index: its expression tree.
    CaptureValue(usize),
    /// The formula node at this arena index: its span.
    Node(usize),
    /// The condition of the `holds` atom at this formula arena index: its
    /// expression tree.
    Condition(usize),
    /// The written fairness constraint at this index: its span.
    Fairness(usize),
}

/// The `Origin::TypeDeclaration` name of `site` of the temporal clause at
/// `clause`, its index among the unit's temporal clauses. Spaced, so it
/// names no declared type.
pub(crate) fn temporal_site(clause: usize, site: TemporalSite) -> String {
    match site {
        TemporalSite::Claim => format!("temporal {clause}"),
        TemporalSite::Over => format!("temporal {clause} over"),
        TemporalSite::Trigger => format!("temporal {clause} trigger"),
        TemporalSite::Guard => format!("temporal {clause} guard"),
        TemporalSite::Capture(index) => format!("temporal {clause} capture {index}"),
        TemporalSite::CaptureValue(index) => format!("temporal {clause} capture {index} value"),
        TemporalSite::Node(node) => format!("temporal {clause} node {node}"),
        TemporalSite::Condition(node) => format!("temporal {clause} holds {node}"),
        TemporalSite::Fairness(index) => format!("temporal {clause} fair {index}"),
    }
}

/// The root location of `site` of the temporal clause at `clause`.
pub(crate) fn temporal_location(clause: usize, site: TemporalSite) -> Location {
    Location::root(Origin::TypeDeclaration {
        name: temporal_site(clause, site),
    })
}

/// One emitted temporal clause, as lowering reads it.
pub(crate) struct TemporalClauseInput<'a> {
    /// The clause's index among the unit's temporal clauses.
    pub(crate) index: usize,
    /// The checked clause.
    pub(crate) clause: &'a CheckedTemporalClause,
    /// Each `holds` atom's typed condition, by its formula node's index, and
    /// the activation's typed guard.
    pub(crate) conditions: &'a TypedConditions,
}

/// A `holds` atom's checked condition and the name each of its slots was
/// bound under.
pub(super) struct FormulaCondition<'c> {
    /// The checked Boolean condition.
    pub(super) root: CheckedNode<'c>,
    /// The name each slot of the condition was bound under.
    pub(super) slot_names: &'c [String],
}

/// A temporal formula to key (FR-337, FR-488 "The body"): the formula, the
/// site each node is located at, and the typed condition of each `holds`
/// atom by its arena index.
pub(super) struct FormulaInput<'a, 'c> {
    /// The checked formula.
    pub(super) formula: &'a CheckedFormula,
    /// The site of the formula node at an arena index.
    pub(super) node_site: &'a dyn Fn(usize) -> Location,
    /// Each `holds` atom's condition, by arena index.
    pub(super) conditions: &'a BTreeMap<usize, FormulaCondition<'c>>,
    /// Where a fault of the formula as a whole is located.
    pub(super) root_site: &'a Location,
}

/// A lowered temporal clause's keys.
#[derive(Clone, Debug)]
pub(crate) struct LoweredTemporalClause {
    /// The `temporal`/`temporal_clause` node: the clause's identity.
    pub(crate) key: NodeKey,
    /// The `over` parameter's `value`/`parameter` node.
    pub(crate) over: NodeKey,
    /// Every parameter node of the clause in the order of its parameters:
    /// `over`, the trigger binder of an `on each` activation, then each
    /// capture.
    pub(crate) parameters: Vec<NodeKey>,
    /// The `Boolean` scalar type node: the clause's semantic type.
    pub(crate) boolean: NodeKey,
}

impl Lowering<'_> {
    /// Lower and key the temporal clause `input`, recording its one `claim`
    /// occurrence at its declared name, an `expression` occurrence of each
    /// formula and fairness node, and the `over` parameter's occurrences.
    pub(crate) fn temporal_clause(
        &mut self,
        input: &TemporalClauseInput<'_>,
    ) -> Result<LoweredTemporalClause, CheckRefusal> {
        let clause = input.clause;
        let at = |site| temporal_location(input.index, site);
        let claim = at(TemporalSite::Claim);
        let boolean = self.type_node(&ValueType::Boolean, &claim)?;

        let over_at = at(TemporalSite::Over);
        let (over_name, over_type) = clause.over();
        let over_type = self.binder_type(over_type, None, &over_at)?;
        let over = self.parameter(over_name, 0, over_type.clone(), &over_at)?;
        self.record(over_type.clone(), OccurrenceRole::Type, over_at);
        let scope = [Binder {
            slot: 0,
            parameter: over.clone().into(),
            semantic_type: over_type.clone().into(),
        }];

        // FR-337 "`on each` activation at S4" and "Captures at S4": the
        // trigger binder is the level-1 parameter node typed as `over` is,
        // and each capture the next level, typed by its declared type.
        let mut parameters = vec![over.clone()];
        let mut scope = scope.to_vec();
        let trigger = match clause.trigger() {
            None => None,
            Some((trigger_name, _)) => {
                let trigger_at = at(TemporalSite::Trigger);
                let trigger = self.parameter(trigger_name, 1, over_type.clone(), &trigger_at)?;
                self.record(over_type.clone(), OccurrenceRole::Type, trigger_at);
                parameters.push(trigger.clone());
                scope.push(Binder {
                    slot: 1,
                    parameter: trigger.clone().into(),
                    semantic_type: over_type.into(),
                });
                Some(trigger)
            }
        };
        let guard_scope = scope.clone();
        let first_capture = scope.len();
        let mut capture_terms = Vec::with_capacity(clause.captures().len());
        for (index, (name, declared)) in clause.captures().enumerate() {
            let at_name = at(TemporalSite::Capture(index));
            let initializer = input
                .conditions
                .captures
                .get(index)
                .ok_or_else(|| stage_fault(&at_name, "capture-initializer-typed"))?;
            let mut binders = Binders::new(&initializer.slot_names, &scope);
            let term = self.expression(initializer.body.root(), &mut binders)?;
            let declared = self.binder_type(declared, None, &at_name)?;
            let level = first_capture + index;
            let capture = self.parameter(name, level, declared.clone(), &at_name)?;
            self.record(declared.clone(), OccurrenceRole::Type, at_name);
            capture_terms.push(GroupMember::Binding(Binding::new(name, term)));
            parameters.push(capture.clone());
            scope.push(Binder {
                slot: level,
                parameter: capture.into(),
                semantic_type: declared.into(),
            });
        }

        let conditions: BTreeMap<usize, FormulaCondition<'_>> = input
            .conditions
            .holds
            .iter()
            .map(|(index, condition)| {
                (
                    *index,
                    FormulaCondition {
                        root: condition.body.root(),
                        slot_names: &condition.slot_names,
                    },
                )
            })
            .collect();
        let formula = self.temporal_formula(
            &FormulaInput {
                formula: clause.formula(),
                node_site: &|node| temporal_location(input.index, TemporalSite::Node(node)),
                conditions: &conditions,
                root_site: &claim,
            },
            boolean.clone(),
            &scope,
        )?;

        if clause.fairness().len() != clause.fairness_sources().len()
            || clause.fairness_order().len()
                != clause.fairness().len() + clause.protocol_fairness().count()
        {
            return Err(stage_fault(&claim, "fairness-member-has-a-source"));
        }
        let mut fairness = Vec::with_capacity(clause.fairness().len());
        let protocol_fairness: Vec<_> = clause.protocol_fairness().collect();
        for member in clause.fairness_order() {
            let (kind, granularity, target, source) = match *member {
                CheckedFairnessMember::Operation(index) => {
                    let constraint = clause
                        .fairness()
                        .get(index)
                        .ok_or_else(|| stage_fault(&claim, "operation-fairness-index-valid"))?;
                    let source = *clause
                        .fairness_sources()
                        .get(index)
                        .ok_or_else(|| stage_fault(&claim, "operation-fairness-source-valid"))?;
                    let location = at(TemporalSite::Fairness(source));
                    let operation = constraint.operation();
                    let declaration = self.object_node(operation.declaring_type(), &location)?;
                    let name = Identifier::new(operation.name().to_owned())
                        .map_err(|_| stage_fault(&location, "fairness-operation-named"))?;
                    (
                        constraint.kind(),
                        constraint.granularity(),
                        FairnessTarget::Operation { declaration, name },
                        source,
                    )
                }
                CheckedFairnessMember::Protocol { member, source } => {
                    let (constraint, _span) = protocol_fairness
                        .get(member)
                        .ok_or_else(|| stage_fault(&claim, "protocol-fairness-index-valid"))?;
                    let location = at(TemporalSite::Fairness(source));
                    let members =
                        self.protocol_members
                            .get(&constraint.protocol)
                            .ok_or_else(|| {
                                stage_fault(&location, "protocol-fairness-target-lowered")
                            })?;
                    let target = match &constraint.target {
                crate::check::temporal_clause::CheckedProtocolTarget::Root => {
                    FairnessTarget::ProtocolRoot {
                        node: members.clause.clone(),
                    }
                }
                crate::check::temporal_clause::CheckedProtocolTarget::AttemptNode { node } => {
                    FairnessTarget::AttemptNode {
                        node: members.nodes.get(node).cloned().ok_or_else(|| {
                            stage_fault(&location, "attempt-fairness-node-emitted")
                        })?,
                    }
                }
                crate::check::temporal_clause::CheckedProtocolTarget::Branch {
                    parallel,
                    branch,
                } => {
                    FairnessTarget::ParallelBranch {
                        parallel: members.nodes.get(parallel).cloned().ok_or_else(|| {
                            stage_fault(&location, "branch-fairness-node-emitted")
                        })?,
                        branch: Identifier::new(branch.clone())
                            .map_err(|_| stage_fault(&location, "branch-fairness-label-valid"))?,
                    }
                }
                crate::check::temporal_clause::CheckedProtocolTarget::CompensationTemplate {
                    index,
                } => FairnessTarget::CompensationTemplate {
                    node: members.compensations.get(*index).cloned().ok_or_else(|| {
                        stage_fault(&location, "compensation-fairness-node-emitted")
                    })?,
                },
                    };
                    (constraint.kind, constraint.granularity, target, source)
                }
            };
            let location = at(TemporalSite::Fairness(source));
            let key = self.insert(
                &location,
                NodeTag::Temporal,
                "fairness",
                Some(boolean.clone()),
                None,
                BodyTerm::application(
                    Operator::TemporalFairness,
                    Operation {
                        member: Some(Member::Fairness {
                            kind,
                            granularity,
                            target,
                        }),
                        ..Operation::plain("quire.op.temporal.fair")
                    },
                    boolean.clone(),
                    Vec::new(),
                ),
            )?;
            self.record(key.clone(), OccurrenceRole::Expression, location);
            fairness.push(key);
        }

        let law = OperationLaw {
            role: LawRole::TemporalProfile,
            definition: clause.profile().definition(),
        };
        let mut activation = Vec::new();
        match trigger {
            None => {
                let kind = self.text_literal("origin", &claim)?;
                activation.push(GroupMember::Binding(Binding::new("kind", kind)));
            }
            Some(trigger) => {
                let kind = self.text_literal("each", &claim)?;
                activation.push(GroupMember::Binding(Binding::new("kind", kind)));
                activation.push(GroupMember::Binding(Binding::new(
                    "trigger",
                    LeafTerm::reference(trigger),
                )));
                if let Some(guard) = &input.conditions.guard {
                    let mut binders = Binders::new(&guard.slot_names, &guard_scope);
                    let term = self.expression(guard.body.root(), &mut binders)?;
                    activation.push(GroupMember::Binding(Binding::new("guard", term)));
                }
            }
        }
        let clock = self.text_literal(clause.clock(), &claim)?;
        // QSpec FR-370: over, clock, activation, captures, fairness and
        // formula, in that order. An `on origin` activation binds `kind`
        // only, and a clause with no capture binds the empty aggregate.
        let key = self.insert(
            &claim,
            NodeTag::Temporal,
            "temporal_clause",
            Some(boolean.clone()),
            None,
            BodyTerm::application(
                Operator::Temporal,
                Operation {
                    laws: vec![law],
                    ..Operation::plain("quire.op.temporal.clause")
                },
                boolean.clone(),
                vec![
                    MemberTerm::reference(over.clone()),
                    MemberTerm::Leaf(clock),
                    MemberTerm::Group(GroupTerm::new(activation)),
                    MemberTerm::Group(GroupTerm::new(capture_terms)),
                    MemberTerm::Group(GroupTerm::references(fairness)),
                    MemberTerm::reference(formula),
                ],
            ),
        )?;
        self.record(key.clone(), OccurrenceRole::Claim, claim);
        Ok(LoweredTemporalClause {
            key,
            over,
            parameters,
            boolean,
        })
    }

    /// Key every node of `input`'s formula, each operand before its
    /// operator, and return the root's key. The arena puts an operator
    /// before its operands, so one reverse pass keys every operand first.
    pub(super) fn temporal_formula(
        &mut self,
        input: &FormulaInput<'_, '_>,
        boolean: NodeKey,
        scope: &[Binder],
    ) -> Result<NodeKey, CheckRefusal> {
        let formula = input.formula;
        let mut keys: Vec<Option<NodeKey>> = vec![None; formula.nodes().len()];
        for (index, node) in formula.nodes().iter().enumerate().rev() {
            let location = (input.node_site)(index);
            let operand = |id: FormulaNodeId| -> Result<MemberTerm, CheckRefusal> {
                keys.get(id.index())
                    .cloned()
                    .flatten()
                    .map(MemberTerm::reference)
                    .ok_or_else(|| stage_fault(&location, "operand-keyed-first"))
            };
            let (identity, member, arguments) = match node {
                FormulaNode::Constant(true) => ("quire.op.temporal.true", None, Vec::new()),
                FormulaNode::Constant(false) => ("quire.op.temporal.false", None, Vec::new()),
                FormulaNode::Holds { .. } => {
                    let condition = input
                        .conditions
                        .get(&index)
                        .ok_or_else(|| stage_fault(&location, "holds-condition-typed"))?;
                    let mut binders = Binders::new(condition.slot_names, scope);
                    let operand = self.expression(condition.root, &mut binders)?;
                    (
                        "quire.op.temporal.holds",
                        None,
                        vec![MemberTerm::Leaf(operand)],
                    )
                }
                FormulaNode::Aggregate { .. } => {
                    return Err(stage_fault(&location, "aggregate-not-emitted"));
                }
                FormulaNode::Not(only) => ("quire.op.temporal.not", None, vec![operand(*only)?]),
                FormulaNode::And(left, right) => (
                    "quire.op.temporal.and",
                    None,
                    vec![operand(*left)?, operand(*right)?],
                ),
                FormulaNode::Or(left, right) => (
                    "quire.op.temporal.or",
                    None,
                    vec![operand(*left)?, operand(*right)?],
                ),
                FormulaNode::Implies(left, right) => (
                    "quire.op.temporal.implies",
                    None,
                    vec![operand(*left)?, operand(*right)?],
                ),
                FormulaNode::Unary {
                    operator,
                    interval,
                    operand: only,
                } => (
                    unary_identity(*operator),
                    Some(temporal_interval(interval.as_ref())),
                    vec![operand(*only)?],
                ),
                FormulaNode::Binary {
                    operator,
                    interval,
                    left,
                    right,
                } => (
                    binary_identity(*operator),
                    Some(temporal_interval(interval.as_ref())),
                    vec![operand(*left)?, operand(*right)?],
                ),
            };
            let key = self.insert(
                &location,
                NodeTag::Temporal,
                "formula",
                Some(boolean.clone()),
                None,
                BodyTerm::application(
                    Operator::TemporalFormula,
                    Operation {
                        member,
                        ..Operation::plain(identity)
                    },
                    boolean.clone(),
                    arguments,
                ),
            )?;
            self.record(key.clone(), OccurrenceRole::Expression, location);
            if let Some(slot) = keys.get_mut(index) {
                *slot = Some(key);
            }
        }
        keys.get(formula.root().index())
            .cloned()
            .flatten()
            .ok_or_else(|| stage_fault(input.root_site, "formula-root-keyed"))
    }
}

/// A unary temporal operator's own `quire.op.temporal.*` identity.
const fn unary_identity(operator: UnaryTemporal) -> &'static str {
    match operator {
        UnaryTemporal::Eventually => "quire.op.temporal.eventually",
        UnaryTemporal::Always => "quire.op.temporal.always",
        UnaryTemporal::Once => "quire.op.temporal.once",
        UnaryTemporal::Historically => "quire.op.temporal.historically",
    }
}

/// A binary temporal operator's own `quire.op.temporal.*` identity.
const fn binary_identity(operator: BinaryTemporal) -> &'static str {
    match operator {
        BinaryTemporal::Until => "quire.op.temporal.until",
        BinaryTemporal::Release => "quire.op.temporal.release",
        BinaryTemporal::Since => "quire.op.temporal.since",
        BinaryTemporal::Triggered => "quire.op.temporal.triggered",
    }
}

/// A broken invariant of this lowering, never a refusal of the input.
pub(super) fn stage_fault(location: &Location, invariant: &'static str) -> CheckRefusal {
    fault(
        location,
        KeyFault::StageFault(quire_semantic_value::catalog::InternalFault::new(
            STAGE, invariant,
        )),
    )
}

/// A checked operator's interval in the FR-370 wire form.
fn temporal_interval(interval: Option<&CheckedInterval>) -> Member {
    match interval {
        None => Member::TemporalInterval { interval: None },
        Some(CheckedInterval::Steps(interval)) => Member::TemporalInterval {
            interval: Some(*interval),
        },
        Some(CheckedInterval::Timed(interval)) => Member::TimedInterval(Box::new(TimedMember {
            lower: interval.lower().clone(),
            lower_closed: interval.lower_closed(),
            upper: Some(interval.upper().clone()),
            upper_closed: interval.upper_closed(),
        })),
    }
}
