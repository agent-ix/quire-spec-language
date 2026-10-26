// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-230: a timed state model's clocks, guards, resets, time invariants
//! and urgency, checked at S3 (ADR-026 CK-2 to CK-9; QSpec FR-415).
//!
//! A clock is a field of a model object type that the admitted domain
//! package types `ix://quire/native/Clock`. It is a place in the timed
//! model, never a value of the value kernel: S3 confines every read of a
//! clock to the clock operand of an atomic clock constraint `self.x ~ c`,
//! and every write to a reset `self.x = c` in a postcondition, `c` an exact
//! non-negative time quantity converted exactly to the model's unit.
//!
//! The check runs in two steps. [`prepare`] reads every clock out of the
//! unit's state clauses before they are typed: a precondition's clock
//! constraints become the operation's guard, a disjunction of convex parts,
//! each a conjunction of atomic clock constraints with the data predicate
//! of its part; a postcondition's resets become the operation's resets.
//! Each clock constraint and reset in a clause is replaced in the clause by
//! the Boolean it contributes to the clause's data reading (`true` where it
//! is read positively, `false` under a negation), so the clause the value
//! kernel types and evaluates reads data alone. [`check`] then types each
//! guard part's data predicate, and each time invariant's and `urgent
//! when`'s predicate, with the one clause checker (FR-104), and records one
//! [`TimedModel`] for each dense time source.
//!
//! The predicate of a time invariant and of `urgent when` reads the model's
//! state object as `self`, as a `terminal when` predicate does (FR-124): the
//! one object type the model's populations range over.

use std::collections::{BTreeMap, BTreeSet};

use qsl_forms::{
    BinaryOperator, ExprId, ExprNode, ExpressionForm, QuantityForm, StateClauseKind,
    TimeInvariantForm, UrgentForm, UrgentTarget,
};
use qsl_foundation::Span;
use quire_checked::key::DeclarationKey;
use quire_exact::{EffectiveId, Rational, ValueType};
use quire_semantic_value::declaration::TypeEnvironment;

use super::assemble::TimedDeclaration;
use super::ir::{CheckedBody, CheckedNode};
use super::lowering::AdmittedModel;
use super::refusal::{CheckCause, CheckRefusal, TemporalCause};
use super::state_clause::{
    check_clause, population_of, ClauseDeclarations, ClauseOperation, ClauseView,
    PopulationDomain, StateClauseDeclaration,
};
use super::state_graph_claim::StatePredicate;
use super::temporal_clause::{ModelTimeSource, TimeSourceKind};
use super::time_units::{TimeScale, TimeUnits};
use crate::family::CheckContext;

/// The index of a clock field in [`TimedModel::clocks`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ClockId(usize);

impl ClockId {
    /// The field's position in [`TimedModel::clocks`].
    pub const fn index(self) -> usize {
        self.0
    }
}

/// One clock field of a timed model's object type (FR-230 `ClockField`).
/// Every object of the type holds its own clock of the field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockField {
    owner: EffectiveId,
    field: DeclarationKey,
    name: String,
}

impl ClockField {
    /// The object type that declares the field.
    pub fn owner(&self) -> EffectiveId {
        self.owner
    }

    /// The field's declaration key in its domain package.
    pub fn field(&self) -> &DeclarationKey {
        &self.field
    }

    /// The field's member name.
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// The comparison of an atomic clock constraint `x ~ c` (FR-230
/// `ClockOp`).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ClockOp {
    /// `x < c`.
    Less,
    /// `x <= c`.
    LessOrEqual,
    /// `x = c`.
    Equal,
    /// `x >= c`.
    GreaterOrEqual,
    /// `x > c`.
    Greater,
}

impl ClockOp {
    /// The comparison a written operator denotes, `None` for an operator
    /// that is no clock comparison.
    fn of(operator: BinaryOperator) -> Option<Self> {
        Some(match operator {
            BinaryOperator::Less => Self::Less,
            BinaryOperator::LessOrEqual => Self::LessOrEqual,
            BinaryOperator::Equal => Self::Equal,
            BinaryOperator::GreaterOrEqual => Self::GreaterOrEqual,
            BinaryOperator::Greater => Self::Greater,
            _ => return None,
        })
    }

    /// `c ~ x` read as `x ~' c`.
    const fn mirrored(self) -> Self {
        match self {
            Self::Less => Self::Greater,
            Self::LessOrEqual => Self::GreaterOrEqual,
            Self::Equal => Self::Equal,
            Self::GreaterOrEqual => Self::LessOrEqual,
            Self::Greater => Self::Less,
        }
    }

    /// The convex constraints whose union is `not (x ~ c)`: one, except for
    /// `=`, whose negation is `x < c` or `x > c`.
    fn negated(self) -> Vec<Self> {
        match self {
            Self::Less => vec![Self::GreaterOrEqual],
            Self::LessOrEqual => vec![Self::Greater],
            Self::Equal => vec![Self::Less, Self::Greater],
            Self::GreaterOrEqual => vec![Self::Less],
            Self::Greater => vec![Self::LessOrEqual],
        }
    }

    /// Whether `value ~ bound`.
    pub fn holds(self, value: &Rational, bound: &Rational) -> bool {
        let order = value.cmp(bound);
        match self {
            Self::Less => order.is_lt(),
            Self::LessOrEqual => order.is_le(),
            Self::Equal => order.is_eq(),
            Self::GreaterOrEqual => order.is_ge(),
            Self::Greater => order.is_gt(),
        }
    }
}

/// An atomic clock constraint `x ~ c` over the clock `x` of `self`, its
/// bound exact in the model's unit (FR-230 `ClockConstraint`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockConstraint {
    clock: ClockId,
    op: ClockOp,
    bound: Rational,
    span: Span,
}

impl ClockConstraint {
    /// The constrained clock of the receiver.
    pub fn clock(&self) -> ClockId {
        self.clock
    }

    /// The comparison.
    pub fn op(&self) -> ClockOp {
        self.op
    }

    /// The bound, in the model's unit.
    pub fn bound(&self) -> &Rational {
        &self.bound
    }

    /// The span of the constraint as written.
    pub fn span(&self) -> Span {
        self.span
    }

    fn with_op(&self, op: ClockOp) -> Self {
        Self { op, ..self.clone() }
    }
}

/// A checked Boolean predicate of a timed model: a guard part's data
/// predicate, a time invariant's or an `urgent when`'s predicate.
#[derive(Debug)]
pub struct CheckedTimedPredicate {
    parameters: Vec<(String, ValueType)>,
    body: CheckedBody,
    slots: usize,
}

impl CheckedTimedPredicate {
    /// The slots the body is checked over, in order: `self`, then the
    /// operation's parameters for a guard part.
    pub fn parameters(&self) -> &[(String, ValueType)] {
        &self.parameters
    }
}

impl StatePredicate for CheckedTimedPredicate {
    fn body(&self) -> CheckedNode<'_> {
        self.body.root()
    }

    fn slots(&self) -> usize {
        self.slots
    }
}

/// One convex part of an operation's guard: its data predicate, `None` for
/// `true`, and a conjunction of atomic clock constraints.
#[derive(Debug)]
pub struct ConvexPart {
    data: Option<CheckedTimedPredicate>,
    clocks: Vec<ClockConstraint>,
}

impl ConvexPart {
    /// The part's data predicate; `None` when the part reads no data.
    pub fn data(&self) -> Option<&CheckedTimedPredicate> {
        self.data.as_ref()
    }

    /// The part's clock constraints, in source order.
    pub fn clocks(&self) -> &[ClockConstraint] {
        &self.clocks
    }
}

/// The guard one precondition places on its operation: a disjunction of
/// convex parts. An operation's guard is the conjunction of the guards of
/// its preconditions.
#[derive(Debug)]
pub struct ClockGuard {
    clause: String,
    operation: ClauseOperation,
    parts: Vec<ConvexPart>,
}

impl ClockGuard {
    /// The precondition's declared name.
    pub fn clause(&self) -> &str {
        &self.clause
    }

    /// The guarded operation.
    pub fn operation(&self) -> &ClauseOperation {
        &self.operation
    }

    /// The convex parts, in the order the precondition's disjunctive form
    /// lists them.
    pub fn parts(&self) -> &[ConvexPart] {
        &self.parts
    }
}

/// A reset `self.x = c` of one clock of an operation's receiver.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockReset {
    clock: ClockId,
    value: Rational,
    span: Span,
}

impl ClockReset {
    /// The reset clock of the receiver.
    pub fn clock(&self) -> ClockId {
        self.clock
    }

    /// The value it is reset to, in the model's unit.
    pub fn value(&self) -> &Rational {
        &self.value
    }

    /// The span of the reset conjunct.
    pub fn span(&self) -> Span {
        self.span
    }
}

/// The resets one postcondition writes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockResets {
    clause: String,
    operation: ClauseOperation,
    resets: Vec<ClockReset>,
}

impl ClockResets {
    /// The postcondition's declared name.
    pub fn clause(&self) -> &str {
        &self.clause
    }

    /// The operation whose postcondition it is.
    pub fn operation(&self) -> &ClauseOperation {
        &self.operation
    }

    /// The resets, in source order.
    pub fn resets(&self) -> &[ClockReset] {
        &self.resets
    }
}

/// `time invariant N when { P } { x <= c and ... }`: at a state where `P`
/// holds of an object, time may pass only while the bounds hold of its
/// clocks.
#[derive(Debug)]
pub struct TimeInvariant {
    name: String,
    context: EffectiveId,
    predicate: CheckedTimedPredicate,
    bounds: Vec<ClockConstraint>,
}

impl TimeInvariant {
    /// The declared name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The object type `self` ranges over.
    pub fn context(&self) -> EffectiveId {
        self.context
    }

    /// The `when` predicate.
    pub fn predicate(&self) -> &CheckedTimedPredicate {
        &self.predicate
    }

    /// The upper bounds, each `<` or `<=`, in declared order.
    pub fn bounds(&self) -> &[ClockConstraint] {
        &self.bounds
    }
}

/// An `urgent` declaration: no positive delay where it applies.
#[derive(Debug)]
pub enum Urgency {
    /// `urgent N when { P }`: urgent at a state where `P` holds of an
    /// object of the context type.
    When {
        /// The declared name.
        name: String,
        /// The object type `self` ranges over.
        context: EffectiveId,
        /// The predicate.
        predicate: CheckedTimedPredicate,
    },
    /// `urgent N O`: urgent while operation `O` is enabled.
    Operation {
        /// The declared name.
        name: String,
        /// The operation's declaration key.
        operation: DeclarationKey,
    },
}

/// FR-230's checked `TimedModel`, for one dense time source: the model's
/// clocks and the guards, resets, time invariants and urgency that read
/// them. The time source itself is the graph's
/// [`ModelTimeSource`](super::ModelTimeSource) of the same model.
#[derive(Debug)]
pub struct TimedModel {
    model: String,
    span: Span,
    clocks: Vec<ClockField>,
    guards: Vec<ClockGuard>,
    resets: Vec<ClockResets>,
    invariants: Vec<TimeInvariant>,
    urgency: Vec<Urgency>,
}

impl TimedModel {
    /// The model alias.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// The clock fields, in ascending declaration-key order.
    pub fn clocks(&self) -> &[ClockField] {
        &self.clocks
    }

    /// The clock field `id` names.
    pub fn clock(&self, id: ClockId) -> Option<&ClockField> {
        self.clocks.get(id.0)
    }

    /// Each precondition's guard, in clause order.
    pub fn guards(&self) -> &[ClockGuard] {
        &self.guards
    }

    /// Each postcondition's resets, in clause order.
    pub fn resets(&self) -> &[ClockResets] {
        &self.resets
    }

    /// The time invariants, in declaration order.
    pub fn invariants(&self) -> &[TimeInvariant] {
        &self.invariants
    }

    /// The urgency declarations, in declaration order.
    pub fn urgency(&self) -> &[Urgency] {
        &self.urgency
    }

    /// Whether the model declares anything past its time source: a clock,
    /// a time invariant or an urgency declaration.
    pub fn has_members(&self) -> bool {
        !self.clocks.is_empty() || !self.invariants.is_empty() || !self.urgency.is_empty()
    }
}

/// One guard part before its data predicate is typed.
struct PartDraft {
    data: Option<ExpressionForm>,
    clocks: Vec<ClockConstraint>,
}

/// A predicate over the model's state object, before it is typed.
struct PredicateDraft {
    context: EffectiveId,
    population: Option<PopulationDomain>,
    predicate: ExpressionForm,
}

enum UrgencyDraft {
    When {
        name: String,
        predicate: PredicateDraft,
    },
    Operation {
        name: String,
        operation: DeclarationKey,
    },
}

struct InvariantDraft {
    name: String,
    predicate: PredicateDraft,
    bounds: Vec<ClockConstraint>,
}

struct ModelDraft {
    model: String,
    span: Span,
    clocks: Vec<ClockField>,
    /// Each guarded precondition, by its index among the unit's state
    /// clauses.
    guards: Vec<(usize, Vec<PartDraft>)>,
    resets: Vec<ClockResets>,
    invariants: Vec<InvariantDraft>,
    urgency: Vec<UrgencyDraft>,
}

/// What [`prepare`] read out of a unit: one draft per dense time source.
pub(crate) struct TimedDrafts {
    models: Vec<ModelDraft>,
}

/// What [`prepare`] reads.
pub(crate) struct Inputs<'a> {
    pub(crate) models: &'a [AdmittedModel],
    pub(crate) types: &'a TypeEnvironment,
    pub(crate) time_sources: &'a [ModelTimeSource],
    pub(crate) timed_forms: &'a [TimedDeclaration],
    pub(crate) units: &'a TimeUnits,
    pub(crate) rational_bits: u64,
}

fn refusal(cause: TemporalCause) -> CheckRefusal {
    CheckRefusal {
        location: quire_semantic_value::location::Location::root(
            quire_semantic_value::location::Origin::Expression,
        ),
        cause: CheckCause::Temporal(Box::new(cause)),
    }
}

/// The clocks of one model and how its quantities convert.
struct Clocks<'a> {
    model: &'a AdmittedModel,
    fields: Vec<ClockField>,
    units: &'a TimeUnits,
    target: Option<TimeScale>,
    rational_bits: u64,
}

impl Clocks<'_> {
    /// The clock `self.f` reads at `id` of `form`, for a clause about
    /// `context`.
    fn read(&self, form: &ExpressionForm, id: ExprId, context: EffectiveId) -> Option<ClockId> {
        let ExprNode::Field { operand, field } = form.expression.get(id)?.node() else {
            return None;
        };
        if !matches!(form.expression.get(*operand)?.node(), ExprNode::SelfRef) {
            return None;
        }
        let key = self.model.clock_field(context, field)?;
        self.fields
            .iter()
            .position(|clock| &clock.field == key)
            .map(ClockId)
    }

    /// The exact value of the time quantity `quantity`, in the model's unit.
    fn quantity(&self, quantity: &QuantityForm) -> Result<Rational, TemporalCause> {
        let magnitude =
            super::probabilistic::exact_value(&quantity.magnitude, self.rational_bits).map_err(
                |_| TemporalCause::ClockBoundNotConstant {
                    span: quantity.span,
                },
            )?;
        let Some(target) = &self.target else {
            return Err(TemporalCause::ClockBoundNotConstant {
                span: quantity.span,
            });
        };
        let value = self
            .units
            .convert(&magnitude, &quantity.unit.name, target)
            .map_err(|cause| TemporalCause::TimeQuantity {
                cause: Box::new(cause),
                span: quantity.unit.span,
            })?;
        if value.numerator().is_negative() {
            return Err(TemporalCause::NegativeClockConstant {
                span: quantity.span,
            });
        }
        Ok(value)
    }

    /// The non-negative constant `id` of `form` denotes: a time quantity
    /// literal. A negated literal is negative.
    fn constant(&self, form: &ExpressionForm, id: ExprId) -> Result<Rational, TemporalCause> {
        let span = span_of(form, id);
        match form.expression.get(id).map(|node| node.node()) {
            Some(ExprNode::Quantity(quantity)) => self.quantity(quantity),
            Some(ExprNode::Negate(operand))
                if matches!(
                    form.expression.get(*operand).map(|node| node.node()),
                    Some(ExprNode::Quantity(_))
                ) =>
            {
                Err(TemporalCause::NegativeClockConstant { span })
            }
            _ => Err(TemporalCause::ClockBoundNotConstant { span }),
        }
    }
}

/// The span of node `id` of `form`, or of its root when the spans do not
/// reach it.
fn span_of(form: &ExpressionForm, id: ExprId) -> Span {
    let path = path_to(form, id);
    path.and_then(|path| form.spans.at(&path))
        .or_else(|| form.spans.span(form.spans.root()))
        .unwrap_or(Span { start: 0, end: 0 })
}

/// The child-index path from the root of `form` to `id`.
fn path_to(form: &ExpressionForm, id: ExprId) -> Option<Vec<usize>> {
    let parents = parents(form);
    let mut path = Vec::new();
    let mut at = id;
    while let Some((parent, index)) = parents.get(at.index()).copied().flatten() {
        path.push(index);
        at = parent;
    }
    (at == form.expression.root_id()).then(|| {
        path.reverse();
        path
    })
}

/// Each node's parent and its index among the parent's children.
fn parents(form: &ExpressionForm) -> Vec<Option<(ExprId, usize)>> {
    let mut parents = vec![None; form.expression.len()];
    for node in form.expression.iter() {
        for (index, child) in node.node().children().into_iter().enumerate() {
            if let Some(slot) = parents.get_mut(child.index()) {
                *slot = Some((node.id(), index));
            }
        }
    }
    parents
}

/// What each node of a clause body reads of the clocks.
struct Reads {
    /// The clock a node reads directly, `self.x`.
    clock: Vec<Option<ClockId>>,
    /// The span of the first clock read under each node, in node order.
    first: Vec<Option<Span>>,
}

impl Reads {
    fn of(form: &ExpressionForm, clocks: &Clocks<'_>, context: EffectiveId) -> Self {
        let mut clock = vec![None; form.expression.len()];
        let mut first: Vec<Option<Span>> = vec![None; form.expression.len()];
        // Children precede their parents, so each child is final when its
        // parent is reached.
        for node in form.expression.iter() {
            let id = node.id();
            if let Some(read) = clocks.read(form, id, context) {
                clock[id.index()] = Some(read);
                first[id.index()] = Some(span_of(form, id));
                continue;
            }
            first[id.index()] = node
                .node()
                .children()
                .into_iter()
                .find_map(|child| first.get(child.index()).copied().flatten());
        }
        Self { clock, first }
    }

    fn reads_clock(&self, id: ExprId) -> bool {
        self.first.get(id.index()).copied().flatten().is_some()
    }

    fn first(&self, id: ExprId) -> Option<Span> {
        self.first.get(id.index()).copied().flatten()
    }

    fn clock(&self, id: ExprId) -> Option<ClockId> {
        self.clock.get(id.index()).copied().flatten()
    }
}

/// A disjunctive form: each part a set of data literals (node, positive)
/// and clock constraints.
type Dnf = Vec<(Vec<(ExprId, bool)>, Vec<ClockConstraint>)>;

fn product(left: &Dnf, right: &Dnf) -> Dnf {
    left.iter()
        .flat_map(|(data, clocks)| {
            right.iter().map(move |(more_data, more_clocks)| {
                (
                    data.iter().chain(more_data).copied().collect(),
                    clocks.iter().chain(more_clocks).cloned().collect(),
                )
            })
        })
        .collect()
}

/// The atomic clock constraint node `id` of `form` is, when it compares a
/// clock with a constant.
fn atom(
    form: &ExpressionForm,
    id: ExprId,
    reads: &Reads,
    clocks: &Clocks<'_>,
) -> Result<Option<ClockConstraint>, TemporalCause> {
    let Some(ExprNode::Binary {
        operator,
        left,
        right,
    }) = form.expression.get(id).map(|node| node.node())
    else {
        return Ok(None);
    };
    let Some(op) = ClockOp::of(*operator) else {
        return Ok(None);
    };
    let span = span_of(form, id);
    let (clock, op, constant) = match (reads.clock(*left), reads.clock(*right)) {
        (Some(_), Some(_)) => return Err(TemporalCause::ClockComparison { span }),
        (Some(clock), None) => (clock, op, *right),
        (None, Some(clock)) => (clock, op.mirrored(), *left),
        (None, None) => return Ok(None),
    };
    if reads.reads_clock(constant) {
        return Err(TemporalCause::ClockComparison { span });
    }
    let bound = clocks.constant(form, constant)?;
    Ok(Some(ClockConstraint {
        clock,
        op,
        bound,
        span,
    }))
}

/// A precondition body's guard: its disjunctive form over clock constraints
/// and maximal clock-free data subterms, and the polarity of each clock
/// constraint node (true where read positively).
fn guard(
    form: &ExpressionForm,
    reads: &Reads,
    clocks: &Clocks<'_>,
) -> Result<(Dnf, BTreeMap<ExprId, bool>), TemporalCause> {
    let len = form.expression.len();
    let mut positive: Vec<Option<Dnf>> = vec![None; len];
    let mut negative: Vec<Option<Dnf>> = vec![None; len];
    let mut atoms = BTreeSet::new();
    let take = |dnf: &[Option<Dnf>], id: ExprId| dnf.get(id.index()).cloned().flatten();
    for node in form.expression.iter() {
        let id = node.id();
        let (pos, neg) = if !reads.reads_clock(id) {
            (
                vec![(vec![(id, true)], Vec::new())],
                vec![(vec![(id, false)], Vec::new())],
            )
        } else if let Some(constraint) = atom(form, id, reads, clocks)? {
            atoms.insert(id);
            let negated = constraint
                .op
                .negated()
                .into_iter()
                .map(|op| (Vec::new(), vec![constraint.with_op(op)]))
                .collect();
            (vec![(Vec::new(), vec![constraint])], negated)
        } else {
            let structure = match node.node() {
                ExprNode::Binary {
                    operator: operator @ (BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Implies),
                    left,
                    right,
                } => Some((*operator, *left, *right)),
                _ => None,
            };
            match (structure, node.node()) {
                (Some((operator, left, right)), _) => {
                    let (Some(lp), Some(ln), Some(rp), Some(rn)) = (
                        take(&positive, left),
                        take(&negative, left),
                        take(&positive, right),
                        take(&negative, right),
                    ) else {
                        return Err(outside(reads, id, form));
                    };
                    match operator {
                        BinaryOperator::And => (product(&lp, &rp), [ln, rn].concat()),
                        BinaryOperator::Or => ([lp, rp].concat(), product(&ln, &rn)),
                        _ => ([ln, rp].concat(), product(&lp, &rn)),
                    }
                }
                (None, ExprNode::Not(operand)) => {
                    let (Some(pos), Some(neg)) = (take(&positive, *operand), take(&negative, *operand))
                    else {
                        return Err(outside(reads, id, form));
                    };
                    (neg, pos)
                }
                _ => return Err(outside(reads, id, form)),
            }
        };
        positive[id.index()] = Some(pos);
        negative[id.index()] = Some(neg);
    }
    let root = form.expression.root_id();
    let dnf = take(&positive, root).ok_or_else(|| outside(reads, root, form))?;
    Ok((dnf, polarity(form, &atoms)))
}

/// A clock read where no constraint may read it, at the read.
fn outside(reads: &Reads, id: ExprId, form: &ExpressionForm) -> TemporalCause {
    TemporalCause::ClockReadOutsideConstraint {
        span: reads.first(id).unwrap_or_else(|| span_of(form, id)),
    }
}

/// Each of `atoms` with whether it is read positively: under an even number
/// of `not`s and `implies` premises.
fn polarity(form: &ExpressionForm, atoms: &BTreeSet<ExprId>) -> BTreeMap<ExprId, bool> {
    let mut found = BTreeMap::new();
    let mut pending = vec![(form.expression.root_id(), true)];
    while let Some((id, positive)) = pending.pop() {
        if atoms.contains(&id) {
            found.insert(id, positive);
            continue;
        }
        match form.expression.get(id).map(|node| node.node()) {
            Some(ExprNode::Not(operand)) => pending.push((*operand, !positive)),
            Some(ExprNode::Binary {
                operator: BinaryOperator::Implies,
                left,
                right,
            }) => {
                pending.push((*left, !positive));
                pending.push((*right, positive));
            }
            Some(ExprNode::Binary {
                operator: BinaryOperator::And | BinaryOperator::Or,
                left,
                right,
            }) => {
                pending.push((*left, positive));
                pending.push((*right, positive));
            }
            _ => {}
        }
    }
    found
}

/// The conjuncts of `form`'s top-level `and` chain, in source order.
fn conjuncts(form: &ExpressionForm) -> Vec<ExprId> {
    let mut found = Vec::new();
    let mut pending = vec![form.expression.root_id()];
    while let Some(id) = pending.pop() {
        match form.expression.get(id).map(|node| node.node()) {
            Some(ExprNode::Binary {
                operator: BinaryOperator::And,
                left,
                right,
            }) => {
                pending.push(*right);
                pending.push(*left);
            }
            _ => found.push(id),
        }
    }
    found
}

/// The data predicate of one guard part: its literals, each cut from the
/// clause body, in source order.
fn part_predicate(form: &ExpressionForm, literals: &[(ExprId, bool)]) -> Option<ExpressionForm> {
    let mut literals: Vec<(ExprId, bool)> = literals.to_vec();
    literals.sort_by_key(|(id, _)| span_of(form, *id).start);
    literals.dedup();
    let operands = literals
        .into_iter()
        .map(|(id, positive)| {
            let cut = form.cut(id, &BTreeMap::new())?;
            if positive {
                Some(cut)
            } else {
                cut.negated()
            }
        })
        .collect::<Option<Vec<_>>>()?;
    ExpressionForm::conjunction(operands)
}

/// One state clause, its clocks read out: a precondition's guard parts, a
/// postcondition's resets, and the clause body with each clock constraint
/// and reset replaced by the Boolean it contributes to the data reading.
fn read_clause(
    clause: &mut StateClauseDeclaration,
    clocks: &Clocks<'_>,
) -> Result<(Option<Vec<PartDraft>>, Option<Vec<ClockReset>>), TemporalCause> {
    let form = ExpressionForm {
        expression: clause.body.clone(),
        spans: clause.spans.body.clone(),
    };
    let reads = Reads::of(&form, clocks, clause.context);
    let root = form.expression.root_id();
    if !reads.reads_clock(root) {
        return Ok((None, None));
    }
    let (leaves, guard_parts, resets) = match clause.kind {
        StateClauseKind::Invariant => return Err(outside(&reads, root, &form)),
        StateClauseKind::Precondition => {
            let (dnf, polarity) = guard(&form, &reads, clocks)?;
            let parts = dnf
                .into_iter()
                .map(|(literals, clocks)| {
                    let data = if literals.is_empty() {
                        None
                    } else {
                        Some(part_predicate(&form, &literals).ok_or(
                            TemporalCause::ClockReadOutsideConstraint {
                                span: span_of(&form, root),
                            },
                        )?)
                    };
                    Ok(PartDraft { data, clocks })
                })
                .collect::<Result<Vec<_>, TemporalCause>>()?;
            let leaves = polarity
                .into_iter()
                .map(|(id, positive)| (id, ExprNode::Boolean(positive)))
                .collect();
            (leaves, Some(parts), None)
        }
        StateClauseKind::Postcondition => {
            let written = clause
                .operation
                .as_ref()
                .map(|operation| {
                    operation
                        .declaration
                        .effect()
                        .written_fields()
                        .cloned()
                        .collect::<BTreeSet<_>>()
                })
                .unwrap_or_default();
            let mut leaves = BTreeMap::new();
            let mut resets = Vec::new();
            for conjunct in conjuncts(&form) {
                if !reads.reads_clock(conjunct) {
                    continue;
                }
                let span = span_of(&form, conjunct);
                let Some(ExprNode::Binary {
                    operator: BinaryOperator::Equal,
                    left,
                    right,
                }) = form.expression.get(conjunct).map(|node| node.node())
                else {
                    return Err(outside(&reads, conjunct, &form));
                };
                let Some(clock) = reads.clock(*left) else {
                    return Err(outside(&reads, conjunct, &form));
                };
                if reads.reads_clock(*right) {
                    return Err(TemporalCause::ClockWrite { span });
                }
                let in_frame = clocks
                    .fields
                    .get(clock.0)
                    .is_some_and(|field| written.contains(&field.field));
                let value = match clocks.constant(&form, *right) {
                    Ok(value) => value,
                    Err(TemporalCause::ClockBoundNotConstant { .. }) => {
                        return Err(TemporalCause::ClockWrite { span })
                    }
                    Err(cause) => return Err(cause),
                };
                if !in_frame {
                    return Err(TemporalCause::ClockWrite { span });
                }
                leaves.insert(conjunct, ExprNode::Boolean(true));
                resets.push(ClockReset { clock, value, span });
            }
            (leaves, None, Some(resets))
        }
    };
    let rewritten = form
        .cut(root, &leaves)
        .ok_or(TemporalCause::ClockReadOutsideConstraint {
            span: span_of(&form, root),
        })?;
    clause.body = rewritten.expression;
    clause.spans.body = rewritten.spans;
    Ok((guard_parts, resets))
}

/// The state object of `model` and its population, which a time invariant
/// or `urgent when` predicate reads as `self`.
fn state_object(
    model: &AdmittedModel,
    models: &[AdmittedModel],
    types: &TypeEnvironment,
    span: Span,
) -> Result<(EffectiveId, Option<PopulationDomain>), TemporalCause> {
    let objects: Vec<EffectiveId> = model.population_objects().into_iter().collect();
    let [context] = objects.as_slice() else {
        return Err(TemporalCause::TimedStateObject {
            ambiguous: !objects.is_empty(),
            span,
        });
    };
    let population = population_of(models, *context, types).map_err(|_| {
        TemporalCause::TimedStateObject {
            ambiguous: true,
            span,
        }
    })?;
    Ok((*context, population))
}

/// A predicate over the state object that reads no clock.
fn state_predicate(
    predicate: &ExpressionForm,
    clocks: &Clocks<'_>,
    context: EffectiveId,
    population: Option<PopulationDomain>,
) -> Result<PredicateDraft, TemporalCause> {
    let reads = Reads::of(predicate, clocks, context);
    if let Some(span) = reads.first(predicate.expression.root_id()) {
        return Err(TemporalCause::ClockReadOutsideConstraint { span });
    }
    Ok(PredicateDraft {
        context,
        population,
        predicate: predicate.clone(),
    })
}

fn time_invariant(
    form: &TimeInvariantForm,
    clocks: &Clocks<'_>,
    context: EffectiveId,
    population: Option<PopulationDomain>,
) -> Result<InvariantDraft, TemporalCause> {
    let predicate = state_predicate(&form.guard, clocks, context, population)?;
    let bounds = form
        .bounds
        .iter()
        .map(|bound| {
            let root = bound.clock.expression.root_id();
            let clock = clocks
                .read(&bound.clock, root, context)
                .ok_or(TemporalCause::UnknownClock {
                    span: span_of(&bound.clock, root),
                })?;
            Ok(ClockConstraint {
                clock,
                op: if bound.strict {
                    ClockOp::Less
                } else {
                    ClockOp::LessOrEqual
                },
                bound: clocks.quantity(&bound.bound)?,
                span: bound.span,
            })
        })
        .collect::<Result<_, TemporalCause>>()?;
    Ok(InvariantDraft {
        name: form.name.name.clone(),
        predicate,
        bounds,
    })
}

fn urgency(
    form: &UrgentForm,
    model: &AdmittedModel,
    clocks: &Clocks<'_>,
    state: impl FnOnce() -> Result<(EffectiveId, Option<PopulationDomain>), TemporalCause>,
    guarded: &BTreeMap<DeclarationKey, String>,
) -> Result<UrgencyDraft, TemporalCause> {
    match &form.target {
        UrgentTarget::When(predicate) => {
            let (context, population) = state()?;
            Ok(UrgencyDraft::When {
                name: form.name.name.clone(),
                predicate: state_predicate(predicate, clocks, context, population)?,
            })
        }
        UrgentTarget::Operation(operation) => {
            let keys = model.tick_operation_keys(operation);
            let [key] = keys.as_slice() else {
                return Err(TemporalCause::UrgentOperation {
                    name: operation.name.clone(),
                    ambiguous: !keys.is_empty(),
                    span: operation.span,
                });
            };
            if let Some(clock) = guarded.get(*key) {
                return Err(TemporalCause::UrgentGuardReadsClock {
                    clock: clock.clone(),
                    span: form.span,
                });
            }
            Ok(UrgencyDraft::Operation {
                name: form.name.name.clone(),
                operation: (*key).clone(),
            })
        }
    }
}

/// FR-230's first step: read every clock out of the unit's state clauses,
/// and resolve every time invariant and `urgent` declaration, before the
/// clauses are typed.
///
/// # Errors
///
/// The located refusal of the first ill-formed timed member, in source
/// order: clocks of a model that is not dense, clock reads outside an
/// atomic clock constraint, clock writes other than a reset in the frame,
/// and timed declarations with no timed model.
pub(crate) fn prepare(
    inputs: &Inputs<'_>,
    state_clauses: &mut [StateClauseDeclaration],
) -> Result<TimedDrafts, CheckRefusal> {
    prepare_models(inputs, state_clauses).map_err(refusal)
}

fn prepare_models(
    inputs: &Inputs<'_>,
    state_clauses: &mut [StateClauseDeclaration],
) -> Result<TimedDrafts, TemporalCause> {
    let source_of = |model: &AdmittedModel| {
        model
            .alias()
            .and_then(|alias| inputs.time_sources.iter().find(|source| source.model == alias))
    };
    let clocks_of = |model| {
        let fields = AdmittedModel::clock_fields(model)
            .into_iter()
            .map(|(owner, field, name)| ClockField {
                owner,
                field: field.clone(),
                name: name.to_owned(),
            })
            .collect();
        let target = source_of(model)
            .and_then(|source| inputs.units.target(&source.unit).ok())
            .flatten();
        Clocks {
            model,
            fields,
            units: inputs.units,
            target,
            rational_bits: inputs.rational_bits,
        }
    };
    // A clock of a tick model refuses at its time declaration; one of an
    // untimed model at its first read.
    for model in inputs.models {
        let clocks = clocks_of(model);
        let Some(field) = clocks.fields.first() else {
            continue;
        };
        let span = match source_of(model) {
            Some(source) if matches!(source.source, TimeSourceKind::Dense) => continue,
            Some(source) => Some(source.span),
            None => state_clauses.iter().find_map(|clause| {
                let form = ExpressionForm {
                    expression: clause.body.clone(),
                    spans: clause.spans.body.clone(),
                };
                Reads::of(&form, &clocks, clause.context).first(form.expression.root_id())
            }),
        };
        if let Some(span) = span {
            return Err(TemporalCause::ClockWithoutDenseTime {
                field: field.name.clone(),
                span,
            });
        }
    }
    let declarations: Vec<&TimedDeclaration> = inputs
        .timed_forms
        .iter()
        .filter(|form| !matches!(form, TimedDeclaration::TimeSource(_)))
        .collect();
    if let Some(form) = declarations.first() {
        match (inputs.time_sources, form) {
            ([], TimedDeclaration::TimeInvariant(form)) => {
                return Err(TemporalCause::TimeInvariantWithoutTimeSource { span: form.span })
            }
            ([], TimedDeclaration::Urgent(form)) => {
                return Err(TemporalCause::UrgencyWithoutTimeSource { span: form.span })
            }
            ([_], _) | ([], _) => {}
            (_, form) => {
                let span = match form {
                    TimedDeclaration::TimeInvariant(form) => form.span,
                    TimedDeclaration::Urgent(form) => form.span,
                    TimedDeclaration::TimeSource(form) => form.span,
                };
                return Err(TemporalCause::AmbiguousTimedDeclaration { span });
            }
        }
    }
    let mut models = Vec::new();
    for source in inputs.time_sources {
        let Some(model) = inputs
            .models
            .iter()
            .find(|model| model.alias() == Some(source.model.as_str()))
        else {
            continue;
        };
        let clocks = clocks_of(model);
        let mut guards = Vec::new();
        let mut resets = Vec::new();
        // The clock each guarded operation's guard reads first, for
        // `urgent O`.
        let mut guarded = BTreeMap::new();
        for (index, clause) in state_clauses.iter_mut().enumerate() {
            if model.type_key(clause.context).is_none() {
                continue;
            }
            let (parts, written) = read_clause(clause, &clocks)?;
            if let Some(parts) = parts {
                let first = parts
                    .iter()
                    .flat_map(|part| &part.clocks)
                    .next()
                    .and_then(|constraint| clocks.fields.get(constraint.clock.0));
                if let (Some(key), Some(clock)) = (&clause.operation_key, first) {
                    guarded
                        .entry(key.clone())
                        .or_insert_with(|| clock.name.clone());
                }
                guards.push((index, parts));
            }
            if let (Some(written), Some(operation)) = (written, &clause.operation) {
                if !written.is_empty() {
                    resets.push(ClockResets {
                        clause: clause.name.clone(),
                        operation: operation.clone(),
                        resets: written,
                    });
                }
            }
        }
        let mut invariants = Vec::new();
        let mut urgency_drafts = Vec::new();
        for form in &declarations {
            let span = match form {
                TimedDeclaration::TimeInvariant(form) => form.span,
                TimedDeclaration::Urgent(form) => form.span,
                TimedDeclaration::TimeSource(_) => continue,
            };
            let state = || state_object(model, inputs.models, inputs.types, span);
            match form {
                TimedDeclaration::TimeInvariant(form) => {
                    let (context, population) = state()?;
                    invariants.push(time_invariant(form, &clocks, context, population)?);
                }
                TimedDeclaration::Urgent(form) => {
                    urgency_drafts.push(urgency(form, model, &clocks, state, &guarded)?);
                }
                TimedDeclaration::TimeSource(_) => {}
            }
        }
        models.push(ModelDraft {
            model: source.model.clone(),
            span: source.span,
            clocks: clocks.fields,
            guards,
            resets,
            invariants,
            urgency: urgency_drafts,
        });
    }
    Ok(TimedDrafts { models })
}

/// What [`check`] reads of the unit beside the drafts.
pub(crate) struct Checking<'d, 'a> {
    pub(crate) declarations: &'d ClauseDeclarations<'a>,
    pub(crate) state_clauses: &'d [StateClauseDeclaration],
    pub(crate) complete_value: bool,
}

/// One predicate typed by the clause checker, its refusal located in its
/// own spans.
fn typed(
    view: &ClauseView<'_>,
    form: &ExpressionForm,
    checking: &Checking<'_, '_>,
    cx: &mut CheckContext<'_, ClauseDeclarations<'_>>,
) -> Result<(CheckedTimedPredicate, u64), CheckRefusal> {
    let typed = check_clause(
        view,
        checking.declarations,
        &mut cx.reborrow(checking.declarations.scope),
    )
    .map_err(|refused| {
        let span = form
            .spans
            .at(&refused.location.path())
            .or_else(|| form.spans.span(form.spans.root()))
            .unwrap_or(Span { start: 0, end: 0 });
        refusal(TemporalCause::TimedPredicate {
            cause: Box::new(refused.cause),
            span,
        })
    })?;
    Ok((
        CheckedTimedPredicate {
            parameters: typed.parameters,
            body: typed.body,
            slots: typed.slots,
        },
        typed.nodes_used,
    ))
}

fn state_view<'v>(draft: &'v PredicateDraft, complete_value: bool) -> ClauseView<'v> {
    ClauseView {
        kind: StateClauseKind::Invariant,
        complete_value,
        context: Some(draft.context),
        receiver: Some(("self", ValueType::Reference(draft.context))),
        operation: None,
        context_population: draft.population,
        frame_population: None,
        body: &draft.predicate.expression,
    }
}

/// FR-230's second step: type each guard part's data predicate and each
/// time invariant's and `urgent when`'s predicate, and record one
/// [`TimedModel`] per dense time source. Returns the models with the
/// package's running node total after them.
///
/// # Errors
///
/// The clause checker's refusal of a predicate, located at the node it
/// names.
pub(crate) fn check(
    drafts: TimedDrafts,
    checking: &Checking<'_, '_>,
    cx: &mut CheckContext<'_, ClauseDeclarations<'_>>,
) -> Result<(Vec<TimedModel>, u64), CheckRefusal> {
    let mut nodes_used = checking.declarations.nodes_used;
    let mut models = Vec::with_capacity(drafts.models.len());
    for draft in drafts.models {
        let mut guards = Vec::with_capacity(draft.guards.len());
        for (index, parts) in draft.guards {
            let Some(clause) = checking.state_clauses.get(index) else {
                continue;
            };
            let Some(operation) = &clause.operation else {
                continue;
            };
            let mut checked = Vec::with_capacity(parts.len());
            for part in parts {
                let data = match &part.data {
                    None => None,
                    Some(form) => {
                        let view = ClauseView {
                            kind: StateClauseKind::Precondition,
                            complete_value: super::state_clause::is_complete_value(
                                &clause.selection,
                            ),
                            context: Some(clause.context),
                            receiver: Some(("self", ValueType::Reference(clause.context))),
                            operation: Some(operation),
                            context_population: clause.context_population,
                            frame_population: clause.frame_population,
                            body: &form.expression,
                        };
                        let (predicate, used) = typed(&view, form, checking, cx)?;
                        nodes_used = nodes_used.max(used);
                        Some(predicate)
                    }
                };
                checked.push(ConvexPart {
                    data,
                    clocks: part.clocks,
                });
            }
            guards.push(ClockGuard {
                clause: clause.name.clone(),
                operation: operation.clone(),
                parts: checked,
            });
        }
        let mut invariants = Vec::with_capacity(draft.invariants.len());
        for invariant in draft.invariants {
            let (predicate, used) = typed(
                &state_view(&invariant.predicate, checking.complete_value),
                &invariant.predicate.predicate,
                checking,
                cx,
            )?;
            nodes_used = nodes_used.max(used);
            invariants.push(TimeInvariant {
                name: invariant.name,
                context: invariant.predicate.context,
                predicate,
                bounds: invariant.bounds,
            });
        }
        let mut urgency = Vec::with_capacity(draft.urgency.len());
        for declared in draft.urgency {
            urgency.push(match declared {
                UrgencyDraft::When { name, predicate } => {
                    let (checked, used) = typed(
                        &state_view(&predicate, checking.complete_value),
                        &predicate.predicate,
                        checking,
                        cx,
                    )?;
                    nodes_used = nodes_used.max(used);
                    Urgency::When {
                        name,
                        context: predicate.context,
                        predicate: checked,
                    }
                }
                UrgencyDraft::Operation { name, operation } => {
                    Urgency::Operation { name, operation }
                }
            });
        }
        models.push(TimedModel {
            model: draft.model,
            span: draft.span,
            clocks: draft.clocks,
            guards,
            resets: draft.resets,
            invariants,
            urgency,
        });
    }
    Ok((models, nodes_used))
}

/// The refusal for a timed model that checks but whose clocks, guards,
/// resets, time invariants or urgency no S4 emission writes yet (FR-230):
/// refused rather than compiled and silently missing from the package's
/// identity.
pub(crate) fn not_emitted(model: &TimedModel) -> CheckRefusal {
    refusal(TemporalCause::TimedModelNotEmitted {
        model: model.model.clone(),
        span: model.span,
    })
}
