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
use crate::check::node_key::{
    Binding, BodyTerm, GroupMember, GroupTerm, LawRole, MemberTerm, NodeTag, Operation,
    OperationLaw, Operator,
};
use crate::check::refusal::{CheckRefusal, KeyFault};
use crate::check::temporal_clause::condition::TypedCondition;
use crate::check::temporal_clause::{
    BinaryTemporal, CheckedTemporalClause, FormulaNode, FormulaNodeId, UnaryTemporal,
};
use crate::value::member::Member;

/// The stage name this module's faults report.
const STAGE: &str = "lowering.temporal";

/// One place of a temporal clause an occurrence or a refusal is located at.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TemporalSite {
    /// The clause's declared name: its `claim` occurrence.
    Claim,
    /// The `over` parameter's name.
    Over,
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
    /// Each `holds` atom's typed condition, by its formula node's index.
    pub(crate) conditions: &'a BTreeMap<usize, TypedCondition>,
}

/// A lowered temporal clause's keys.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LoweredTemporalClause {
    /// The `temporal`/`temporal_clause` node: the clause's identity.
    pub(crate) key: NodeKey,
    /// The `over` parameter's `value`/`parameter` node.
    pub(crate) over: NodeKey,
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
        let over = self.parameter(over_name, 0, over_type, &over_at)?;
        self.record(over_type, OccurrenceRole::Type, over_at);
        let scope = [Binder {
            slot: 0,
            parameter: over,
            semantic_type: over_type,
        }];

        let formula = self.temporal_formula(input, boolean, &scope)?;

        let mut fairness = Vec::with_capacity(clause.fairness().len());
        for (constraint, source) in clause.fairness().iter().zip(clause.fairness_sources()) {
            let location = at(TemporalSite::Fairness(*source));
            let operation = constraint.operation();
            let declaration = self.object_node(operation.declaring_type(), &location)?;
            let name = Identifier::new(operation.name().to_owned())
                .map_err(|_| stage_fault(&location, "fairness-operation-named"))?;
            let key = self.insert(
                &location,
                NodeTag::Temporal,
                "fairness",
                Some(boolean),
                None,
                BodyTerm::application(
                    Operator::TemporalFairness,
                    Operation {
                        member: Some(Member::Fairness {
                            kind: constraint.kind(),
                            granularity: constraint.granularity(),
                            declaration,
                            name,
                        }),
                        ..Operation::plain("quire.op.temporal.fair")
                    },
                    boolean,
                    Vec::new(),
                ),
            )?;
            self.record(key, OccurrenceRole::Expression, location);
            fairness.push(key);
        }

        let law = OperationLaw {
            role: LawRole::TemporalProfile,
            definition: clause.profile().definition(),
        };
        let kind = self.text_literal("origin", &claim)?;
        let clock = self.text_literal(clause.clock(), &claim)?;
        // QSpec FR-370: over, clock, activation, captures, fairness and
        // formula, in that order. An `on origin` activation binds `kind`
        // only, and an emitted clause declares no capture (`is_emitted`).
        let key = self.insert(
            &claim,
            NodeTag::Temporal,
            "temporal_clause",
            Some(boolean),
            None,
            BodyTerm::application(
                Operator::Temporal,
                Operation {
                    laws: vec![law],
                    ..Operation::plain("quire.op.temporal.clause")
                },
                boolean,
                vec![
                    MemberTerm::reference(over),
                    MemberTerm::Leaf(clock),
                    MemberTerm::Group(GroupTerm::new(vec![GroupMember::Binding(Binding::new(
                        "kind", kind,
                    ))])),
                    MemberTerm::Group(GroupTerm::new(Vec::new())),
                    MemberTerm::Group(GroupTerm::references(fairness)),
                    MemberTerm::reference(formula),
                ],
            ),
        )?;
        self.record(key, OccurrenceRole::Claim, claim);
        Ok(LoweredTemporalClause { key, over, boolean })
    }

    /// Key every node of `input`'s formula, each operand before its
    /// operator, and return the root's key. The arena puts an operator
    /// before its operands, so one reverse pass keys every operand first.
    fn temporal_formula(
        &mut self,
        input: &TemporalClauseInput<'_>,
        boolean: NodeKey,
        scope: &[Binder],
    ) -> Result<NodeKey, CheckRefusal> {
        let formula = input.clause.formula();
        let mut keys: Vec<Option<NodeKey>> = vec![None; formula.nodes().len()];
        for (index, node) in formula.nodes().iter().enumerate().rev() {
            let location = temporal_location(input.index, TemporalSite::Node(index));
            let operand = |id: FormulaNodeId| -> Result<MemberTerm, CheckRefusal> {
                keys.get(id.index())
                    .copied()
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
                    let mut binders = Binders::new(&condition.slot_names, scope);
                    let operand = self.expression(condition.body.root(), &mut binders)?;
                    (
                        "quire.op.temporal.holds",
                        None,
                        vec![MemberTerm::Leaf(operand)],
                    )
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
                    Some(Member::TemporalInterval {
                        interval: *interval,
                    }),
                    vec![operand(*only)?],
                ),
                FormulaNode::Binary {
                    operator,
                    interval,
                    left,
                    right,
                } => (
                    binary_identity(*operator),
                    Some(Member::TemporalInterval {
                        interval: *interval,
                    }),
                    vec![operand(*left)?, operand(*right)?],
                ),
            };
            let key = self.insert(
                &location,
                NodeTag::Temporal,
                "formula",
                Some(boolean),
                None,
                BodyTerm::application(
                    Operator::TemporalFormula,
                    Operation {
                        member,
                        ..Operation::plain(identity)
                    },
                    boolean,
                    arguments,
                ),
            )?;
            self.record(key, OccurrenceRole::Expression, location);
            if let Some(slot) = keys.get_mut(index) {
                *slot = Some(key);
            }
        }
        keys.get(formula.root().index())
            .copied()
            .flatten()
            .ok_or_else(|| {
                stage_fault(
                    &temporal_location(input.index, TemporalSite::Claim),
                    "formula-root-keyed",
                )
            })
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
fn stage_fault(location: &Location, invariant: &'static str) -> CheckRefusal {
    fault(
        location,
        KeyFault::StageFault(qsl_foundation::InternalFault::new(STAGE, invariant)),
    )
}
