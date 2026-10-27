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
    TimeInvariantForm, TypeFormHead, UrgentForm, UrgentTarget,
};
use qsl_foundation::Span;
use quire_checked::key::DeclarationKey;
use quire_exact::{EffectiveId, Rational, ValueType};
use quire_semantic_value::declaration::TypeEnvironment;

use super::assemble::TimedDeclaration;
use super::ir::{CheckedBody, CheckedNode};
use super::lowering::{strictly_below, AdmittedModel, CollectionShape, PlaceStep};
use super::refusal::TimedConstruct;
use super::refusal::{CheckCause, CheckRefusal, TemporalCause};
use super::state_clause::{
    check_clause, population_of, ClauseDeclarations, ClauseOperation, ClauseView, PopulationDomain,
    StateClauseDeclaration,
};
use super::state_graph_claim::StatePredicate;
use super::temporal_clause::{ModelTimeSource, TimeSourceKind};
use super::time_units::{TimeScale, TimeUnits};
use crate::family::CheckContext;

mod quantified;
mod reading;

pub(crate) use reading::site as clock_reading_site;

/// The index of a clock field in [`TimedModel::clocks`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ClockId(usize);

impl ClockId {
    /// The field's position in [`TimedModel::clocks`].
    pub const fn index(self) -> usize {
        self.0
    }
}

/// One clock place of a timed model's object type (FR-230 `ClockField`,
/// "Places"): an object's clock field, or a clock below one of its fields
/// along a path of record fields. Every object of the type
/// holds its own clock of the place.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockField {
    owner: EffectiveId,
    field: DeclarationKey,
    path: Vec<PlaceStep>,
    name: String,
}

impl ClockField {
    /// The object type that declares the place's field.
    pub fn owner(&self) -> EffectiveId {
        self.owner
    }

    /// The declaration key of the object's field the place is under, the
    /// member an operation's frame names to reset it.
    pub fn field(&self) -> &DeclarationKey {
        &self.field
    }

    /// The path from the object to the clock, the object's field first.
    pub fn path(&self) -> &[PlaceStep] {
        &self.path
    }

    /// The place's path written with `.` between its steps: the field's
    /// member name for a clock field, `[slot]` for a collection slot.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The most clocks of this place one object holds (FR-230 "Clock
    /// collections", the static clock dimension): one outside collections,
    /// the product of the declared maxima of the collections on its path,
    /// and `None` when one of them declares no maximum.
    pub fn bound(&self) -> Option<u64> {
        self.path.iter().try_fold(1_u64, |count, step| match step {
            PlaceStep::Field(_) => Some(count),
            PlaceStep::Slot(shape) => count.checked_mul(shape.max?),
        })
    }

    /// The collection shapes on the place's path, outermost first.
    pub fn slots(&self) -> impl Iterator<Item = &CollectionShape> {
        self.path.iter().filter_map(|step| match step {
            PlaceStep::Slot(shape) => Some(shape),
            PlaceStep::Field(_) => None,
        })
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
            BinaryOperator::Add
            | BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide
            | BinaryOperator::IntegerDiv
            | BinaryOperator::IntegerRem
            | BinaryOperator::IntegerMod
            | BinaryOperator::NotEqual
            | BinaryOperator::And
            | BinaryOperator::Or
            | BinaryOperator::Implies => return None,
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

/// An atomic clock constraint `x ~ c` over the clock `x` of `self`, or of
/// the object `self` reaches through the reference fields `via`, its bound
/// exact in the model's unit (FR-230 `ClockConstraint`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockConstraint {
    clock: ClockId,
    via: Vec<PlaceStep>,
    slots: Vec<u64>,
    op: ClockOp,
    bound: Rational,
    span: Span,
}

impl ClockConstraint {
    /// The constrained clock of the object holding it.
    pub fn clock(&self) -> ClockId {
        self.clock
    }

    /// The reference fields walked from the receiver to the object holding
    /// the clock, in order: none for a clock of the receiver; `[peer]` for
    /// `self.peer`'s clock, which each state's value of `self.peer` selects
    /// (FR-230-AC-10).
    pub fn via(&self) -> &[PlaceStep] {
        &self.via
    }

    /// The slot of each collection on the clock's path, outermost first:
    /// none outside collections; `[j]` for the clock of slot `j` of a
    /// quantified collection (FR-230 "Quantified reads").
    pub fn slots(&self) -> &[u64] {
        &self.slots
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

/// An element of a clock collection a guard part's data predicate reads
/// (FR-230 "Quantified reads"): the quantifier's binder for one slot,
/// bound to the element the receiver's collection field holds at the slot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElementBinding {
    name: String,
    field: String,
    slot: u64,
}

impl ElementBinding {
    /// The element the receiver's collection field `field` holds at
    /// `slot`, read by the parameter `name`.
    pub fn new(name: String, field: String, slot: u64) -> Self {
        Self { name, field, slot }
    }

    /// The parameter name the data predicate reads the element by.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The receiver's collection field.
    pub fn field(&self) -> &str {
        &self.field
    }

    /// The slot.
    pub fn slot(&self) -> u64 {
        self.slot
    }
}

/// One convex part of an operation's guard: its data predicate, `None` for
/// `true`, and a conjunction of atomic clock constraints.
#[derive(Debug)]
pub struct ConvexPart {
    data: Option<CheckedTimedPredicate>,
    elements: Vec<ElementBinding>,
    clocks: Vec<ClockConstraint>,
}

impl ConvexPart {
    /// The part's data predicate; `None` when the part reads no data.
    pub fn data(&self) -> Option<&CheckedTimedPredicate> {
        self.data.as_ref()
    }

    /// The collection elements the data predicate reads, each a parameter
    /// after `self`, in parameter order. A part whose element's slot the
    /// collection does not hold is not enabled.
    pub fn elements(&self) -> &[ElementBinding] {
        &self.elements
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

/// A reset `self.x = c` of one clock of an operation's receiver, or of the
/// clock of each slot a `forall` selects (FR-230 "Resets by selection").
#[derive(Debug)]
pub struct ClockReset {
    clock: ClockId,
    value: Rational,
    span: Span,
    selection: Option<SlotSelection>,
}

/// The slots a reset `forall(e in self.f: P(e) implies e.x = c)` writes:
/// each slot the collection holds at the pre-state whose element satisfies
/// `P`, every held slot when `P` is absent. A slot not selected keeps its
/// value.
#[derive(Debug)]
pub struct SlotSelection {
    field: String,
    name: String,
    predicate: Option<CheckedTimedPredicate>,
}

impl SlotSelection {
    /// The receiver's collection field.
    pub fn field(&self) -> &str {
        &self.field
    }

    /// The parameter name the predicate reads the element by, after
    /// `self`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The data predicate over the element; `None` selects every held
    /// slot.
    pub fn predicate(&self) -> Option<&CheckedTimedPredicate> {
        self.predicate.as_ref()
    }
}

impl ClockReset {
    /// The slots the reset writes; `None` for a reset of one clock.
    pub fn selection(&self) -> Option<&SlotSelection> {
        self.selection.as_ref()
    }

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
#[derive(Debug)]
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
    /// Each object type of the model with the static clock dimension of
    /// one of its objects ([`Self::dimension`]).
    dimensions: Vec<(EffectiveId, Option<u64>)>,
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

    /// Each clock field with its id, in order.
    pub fn clock_fields(&self) -> impl Iterator<Item = (ClockId, &ClockField)> {
        self.clocks
            .iter()
            .enumerate()
            .map(|(index, field)| (ClockId(index), field))
    }

    /// The static clock dimension of one object of type `owner` (FR-230
    /// "Clock collections"): the [`ClockField::bound`] of each place the
    /// type declares, summed; a place in a collection with no declared
    /// maximum adds no static bound.
    ///
    /// The places counted are those of `owner` and of every type it
    /// conforms to. `None` when `owner` is no object type of the model or
    /// the sum overflows.
    pub fn dimension(&self, owner: EffectiveId) -> Option<u64> {
        self.dimensions
            .iter()
            .find(|(object, _)| *object == owner)
            .and_then(|(_, dimension)| *dimension)
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
    elements: Vec<ElementBinding>,
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
    dimensions: Vec<(EffectiveId, Option<u64>)>,
    /// Each guarded precondition, by its index among the unit's state
    /// clauses.
    guards: Vec<(usize, Vec<PartDraft>)>,
    resets: Vec<ResetsDraft>,
    invariants: Vec<InvariantDraft>,
    urgency: Vec<UrgencyDraft>,
}

/// One reset before the data predicate of its slot selection is typed.
struct ResetDraft {
    clock: ClockId,
    value: Rational,
    span: Span,
    /// The collection field, the element's parameter name and the data
    /// predicate over it, of a reset by selection.
    selection: Option<(String, String, Option<ExpressionForm>)>,
}

impl ResetDraft {
    /// A reset of one clock.
    fn single(clock: ClockId, value: Rational, span: Span) -> Self {
        Self {
            clock,
            value,
            span,
            selection: None,
        }
    }
}

/// The resets of one postcondition before they are typed.
struct ResetsDraft {
    clause: String,
    operation: ClauseOperation,
    context: EffectiveId,
    resets: Vec<ResetDraft>,
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
    /// The unit's clock-reading declarations (FR-230 "Clock-reading
    /// declarations").
    pub(crate) clock_reading: &'a [super::assemble::ClockReadingDeclaration],
    /// The unit's operation bodies, which S3 does not type: a call of a
    /// clock-reading declaration in one is refused here.
    pub(crate) operation_bodies: &'a [super::OperationBodyDeclaration],
    pub(crate) units: &'a TimeUnits,
    pub(crate) rational_bits: u64,
    /// The package's expression-node limit, which bounds each guard's
    /// disjunctive form.
    pub(crate) nodes: u64,
    /// Each selected model alias's declaration span, which locates a clock
    /// field of an untimed model that no clause reads.
    pub(crate) model_spans: &'a std::collections::BTreeMap<String, Span>,
}

/// `cause`, located by the span it carries.
fn refusal(cause: CheckCause) -> CheckRefusal {
    CheckRefusal {
        location: quire_semantic_value::location::Location::root(
            quire_semantic_value::location::Origin::Expression,
        ),
        cause,
    }
}

/// The clocks of one model and how its quantities convert.
struct Clocks<'a> {
    model: &'a AdmittedModel,
    fields: Vec<ClockField>,
    units: &'a TimeUnits,
    target: Option<TimeScale>,
    rational_bits: u64,
    /// The unit's clock-reading declarations, whose calls read clocks.
    declarations: &'a reading::Declarations<'a>,
}

impl Clocks<'_> {
    /// The clock place the path `self.f.g…` reads at `id` of `form`, for a
    /// clause about `context`.
    fn read(&self, form: &ExpressionForm, id: ExprId, context: EffectiveId) -> Option<ClockId> {
        let path = self_path(form, id)?;
        self.fields
            .iter()
            .position(|clock| clock.path == path && self.model.conforms(context, clock.owner))
            .map(ClockId)
    }

    /// Whether the path `self.f…` at `id` of `form` is a value holding clock
    /// places below it: a clock-bearing record or union value.
    fn bears(&self, form: &ExpressionForm, id: ExprId, context: EffectiveId) -> bool {
        self_path(form, id).is_some_and(|path| self.bears_path(&path, context))
    }

    /// Whether `path` (its slots open) is a clock place of an object of type
    /// `context`, or a clock-bearing value: a clock place lies at or below
    /// it.
    fn reaches(&self, path: &[PlaceStep], context: EffectiveId) -> bool {
        self.fields.iter().any(|clock| {
            clock.path.len() >= path.len()
                && same_place(&clock.path[..path.len()], path)
                && self.model.conforms(context, clock.owner)
        })
    }

    /// Whether the value an object of type `context` holds at `path` is
    /// clock-bearing: a clock place lies strictly below it.
    fn bears_path(&self, path: &[PlaceStep], context: EffectiveId) -> bool {
        self.fields.iter().any(|clock| {
            strictly_below(&clock.path, path) && self.model.conforms(context, clock.owner)
        })
    }

    /// Whether node `id` of `form` reads a clock through a path other than
    /// `self`, `o.x`: a read no clock constraint admits (FR-230). The field
    /// is looked up on the operand's object type; when that type is not
    /// known, `.x` reads a clock when every package field named `x` is one.
    fn foreign(
        &self,
        form: &ExpressionForm,
        id: ExprId,
        context: EffectiveId,
        parents: &[Option<ExprId>],
    ) -> bool {
        let Some(ExprNode::Field { operand, field }) = form.expression.get(id).map(|n| n.node())
        else {
            return false;
        };
        if matches!(
            form.expression.get(*operand).map(|n| n.node()),
            Some(ExprNode::SelfRef)
        ) {
            return false;
        }
        match self.object_of(form, *operand, context, parents) {
            Some(object) => self.model.clock_field(object, field).is_some(),
            None => self.model.names_only_clocks(field),
        }
    }

    /// The object type node `id` of `form` denotes an object of, when it is
    /// `self` or the binder of an enclosing query over `allInstances<T>`.
    fn object_of(
        &self,
        form: &ExpressionForm,
        id: ExprId,
        context: EffectiveId,
        parents: &[Option<ExprId>],
    ) -> Option<EffectiveId> {
        let name = match form.expression.get(id)?.node() {
            ExprNode::SelfRef => return Some(context),
            ExprNode::Name(name) if name.segments.len() == 1 => name.name.as_str(),
            _ => return None,
        };
        // The nearest enclosing binding of `name` whose scope holds `id`.
        let mut child = id;
        while let Some(parent) = parents.get(child.index()).copied().flatten() {
            let (binder, source) = match form.expression.get(parent)?.node() {
                ExprNode::Query { binder, source, .. }
                | ExprNode::Count { binder, source, .. }
                | ExprNode::Sum { binder, source, .. } => (binder.as_str(), Some(*source)),
                ExprNode::Accumulate {
                    binder,
                    accumulator,
                    source,
                    ..
                } => {
                    if accumulator == name && *source != child {
                        return None;
                    }
                    (binder.as_str(), Some(*source))
                }
                ExprNode::Let { name: bound, .. } | ExprNode::Seq { binder: bound, .. } => {
                    (bound.as_str(), None)
                }
                ExprNode::Population { binder, .. } => (binder.as_str(), None),
                _ => ("", None),
            };
            if binder == name && source != Some(child) {
                let ExprNode::AllInstances { target, .. } = form.expression.get(source?)?.node()
                else {
                    return None;
                };
                let TypeFormHead::Name(written) = &target.head else {
                    return None;
                };
                return self.model.object_type_named(written);
            }
            child = parent;
        }
        None
    }

    /// The exact value of the time quantity `quantity`, in the model's unit.
    fn quantity(&self, quantity: &QuantityForm) -> Result<Rational, TemporalCause> {
        let magnitude = super::probabilistic::exact_value(&quantity.magnitude, self.rational_bits)
            .map_err(|_| TemporalCause::ClockBoundNotConstant {
                span: quantity.span,
            })?;
        let Some(target) = &self.target else {
            return Err(TemporalCause::ClockBoundNotConstant {
                span: quantity.span,
            });
        };
        let value = self
            .units
            .convert(&magnitude, &quantity.unit.name, target)
            .map_err(|cause| {
                // A unit of another time dimension refuses at the constant;
                // a unit that is no time unit, at the unit.
                let span = if matches!(
                    cause,
                    super::refusal::TimeUnitCause::OtherTimeDimension { .. }
                ) {
                    quantity.span
                } else {
                    quantity.unit.span
                };
                TemporalCause::TimeQuantity {
                    cause: Box::new(cause),
                    span,
                }
            })?;
        if value.numerator().is_negative() {
            return Err(TemporalCause::NegativeClockConstant {
                span: quantity.span,
            });
        }
        Ok(value)
    }
}

/// The record-field path `f.g…` of `self.f.g…` at `id` of `form`, the
/// object's field first; `None` when `id` is no such path.
fn self_path(form: &ExpressionForm, id: ExprId) -> Option<Vec<PlaceStep>> {
    let mut path = Vec::new();
    let mut at = id;
    loop {
        match form.expression.get(at)?.node() {
            ExprNode::Field { operand, field } => {
                path.push(PlaceStep::Field(field.clone()));
                at = *operand;
            }
            ExprNode::SelfRef if !path.is_empty() => {
                path.reverse();
                return Some(path);
            }
            _ => return None,
        }
    }
}

/// The binder a traversal node binds and the collection it ranges over.
fn traversal(node: &ExprNode) -> Option<(&str, ExprId)> {
    match node {
        ExprNode::Query { binder, source, .. }
        | ExprNode::Count { binder, source, .. }
        | ExprNode::Sum { binder, source, .. }
        | ExprNode::Accumulate { binder, source, .. } => Some((binder.as_str(), *source)),
        _ => None,
    }
}

/// Whether node `id` is the operand of `size` or the collection a traversal
/// ranges over: a position that reads no element whole.
fn traversed(form: &ExpressionForm, id: ExprId, parents: &[Option<ExprId>]) -> bool {
    let Some(parent) = parents.get(id.index()).copied().flatten() else {
        return false;
    };
    match form.expression.get(parent).map(|node| node.node()) {
        Some(ExprNode::Size(operand)) => *operand == id,
        Some(node) => traversal(node).is_some_and(|(_, source)| source == id),
        None => false,
    }
}

/// The place path node `id` reads when it is a part of an element of a
/// collection `self.f…`, through the binder of a traversal over it:
/// `e.g…` is the collection's path, a slot, then `g…`. The slot's shape is
/// left open ([`same_place`] matches any).
fn element_path(
    form: &ExpressionForm,
    id: ExprId,
    parents: &[Option<ExprId>],
) -> Option<Vec<PlaceStep>> {
    let mut steps = Vec::new();
    let mut at = id;
    let name = loop {
        match form.expression.get(at)?.node() {
            ExprNode::Field { operand, field } => {
                steps.push(PlaceStep::Field(field.clone()));
                at = *operand;
            }
            ExprNode::Name(name) if name.segments.len() == 1 => break name.name.as_str(),
            _ => return None,
        }
    };
    steps.reverse();
    // The nearest enclosing traversal binding `name` whose body holds `at`.
    let mut child = at;
    while let Some(parent) = parents.get(child.index()).copied().flatten() {
        if let Some((binder, source)) = form
            .expression
            .get(parent)
            .and_then(|node| traversal(node.node()))
        {
            if binder == name && source != child {
                let mut path = self_path(form, source)?;
                path.push(PlaceStep::Slot(OPEN_SLOT));
                path.extend(steps);
                return Some(path);
            }
        }
        child = parent;
    }
    None
}

/// The slot shape [`element_path`] writes, which [`same_place`] does not
/// compare.
const OPEN_SLOT: CollectionShape = CollectionShape {
    ordered: false,
    unique: false,
    min: 0,
    max: None,
};

/// Whether two place paths name one place: equal steps, any slot matching
/// any slot.
fn same_place(left: &[PlaceStep], right: &[PlaceStep]) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|steps| match steps {
            (PlaceStep::Field(left), PlaceStep::Field(right)) => left == right,
            (PlaceStep::Slot(_), PlaceStep::Slot(_)) => true,
            _ => false,
        })
}

/// One expression form with the span of each node and what each node reads
/// of the clocks.
struct Located<'f> {
    form: &'f ExpressionForm,
    /// The span of each node, by id.
    spans: Vec<Span>,
    /// The clock a node reads directly, `self.x`.
    clock: Vec<Option<ClockId>>,
    /// The span of the first clock read under each node.
    first: Vec<Option<Span>>,
}

impl<'f> Located<'f> {
    /// `form`'s clock reads, for a clause about `context`; `None` when its
    /// spans do not fit it.
    fn of(form: &'f ExpressionForm, clocks: &Clocks<'_>, context: EffectiveId) -> Option<Self> {
        let spans = form.spans.of_nodes(&form.expression)?;
        let mut clock = vec![None; form.expression.len()];
        let mut first: Vec<Option<Span>> = vec![None; form.expression.len()];
        let mut parents: Vec<Option<ExprId>> = vec![None; form.expression.len()];
        for node in form.expression.iter() {
            for child in node.node().children() {
                if let Some(slot) = parents.get_mut(child.index()) {
                    *slot = Some(node.id());
                }
            }
        }
        // Children precede their parents, so each child is final when its
        // parent is reached.
        for node in form.expression.iter() {
            let id = node.id();
            // A call of a clock-reading declaration reads its clocks, at the
            // call.
            if clocks.declarations.is_call(&form.expression, id) {
                first[id.index()] = spans.get(id.index()).copied();
                continue;
            }
            if let Some(read) = clocks.read(form, id, context) {
                clock[id.index()] = Some(read);
                first[id.index()] = spans.get(id.index()).copied();
                continue;
            }
            // A part of an element of a clock collection, read through a
            // traversal's binder (`e.n`, `e.x`): a clock or a clock-bearing
            // value is a read no constraint takes; a data part reads none.
            if let Some(path) = element_path(form, id, &parents) {
                if clocks.reaches(&path, context) {
                    first[id.index()] = spans.get(id.index()).copied();
                }
                continue;
            }
            // A `filter` over a clock collection is a collection of its
            // clock-bearing elements: read whole, outside `size` or another
            // traversal's source, it compares or passes them as values
            // (FR-230 "Element identity"; SR-1655001 FND-001).
            if let ExprNode::Query {
                query: qsl_forms::BinderQuery::Filter,
                source,
                ..
            } = node.node()
            {
                if clocks.bears(form, *source, context) && !traversed(form, id, &parents) {
                    first[id.index()] = spans.get(id.index()).copied();
                    continue;
                }
            }
            // A clock-bearing value read whole is a read no constraint takes,
            // except a clock collection whose size is read or which a
            // traversal ranges over: its elements are read through the
            // binder. A data field of one (`self.t.n`) reads no clock.
            if clocks.bears(form, id, context) {
                if !traversed(form, id, &parents) {
                    first[id.index()] = spans.get(id.index()).copied();
                }
                continue;
            }
            // A data field of a clock-bearing record (`self.t.n`) reads no
            // clock; any other path is decided below.
            if self_path(form, id).is_some_and(|path| {
                path.split_last().is_some_and(|(_, record)| {
                    !record.is_empty() && clocks.bears_path(record, context)
                })
            }) {
                continue;
            }
            // A clock read through another path is a read, though no
            // constraint can take it.
            if clocks.foreign(form, id, context, &parents) {
                first[id.index()] = spans.get(id.index()).copied();
                continue;
            }
            first[id.index()] = node
                .node()
                .children()
                .into_iter()
                .find_map(|child| first.get(child.index()).copied().flatten());
        }
        Some(Self {
            form,
            spans,
            clock,
            first,
        })
    }

    fn node(&self, id: ExprId) -> Option<&'f ExprNode> {
        self.form.expression.get(id).map(|node| node.node())
    }

    fn root(&self) -> ExprId {
        self.form.expression.root_id()
    }

    fn span(&self, id: ExprId) -> Span {
        self.spans
            .get(id.index())
            .copied()
            .unwrap_or(Span { start: 0, end: 0 })
    }

    fn reads_clock(&self, id: ExprId) -> bool {
        self.first(id).is_some()
    }

    fn first(&self, id: ExprId) -> Option<Span> {
        self.first.get(id.index()).copied().flatten()
    }

    fn clock(&self, id: ExprId) -> Option<ClockId> {
        self.clock.get(id.index()).copied().flatten()
    }

    /// A clock read under `id` where no constraint may read it, at the read.
    fn outside(&self, id: ExprId) -> TemporalCause {
        TemporalCause::ClockReadOutsideConstraint {
            span: self.first(id).unwrap_or_else(|| self.span(id)),
        }
    }

    /// The non-negative constant `id` denotes: a time quantity literal. A
    /// negated literal is negative.
    fn constant(&self, id: ExprId, clocks: &Clocks<'_>) -> Result<Rational, TemporalCause> {
        let span = self.span(id);
        match self.node(id) {
            Some(ExprNode::Quantity(quantity)) => clocks.quantity(quantity),
            Some(ExprNode::Negate(operand))
                if matches!(self.node(*operand), Some(ExprNode::Quantity(_))) =>
            {
                Err(TemporalCause::NegativeClockConstant { span })
            }
            _ => Err(TemporalCause::ClockBoundNotConstant { span }),
        }
    }
}

/// A data literal of a guard part (FR-230 "Clock constraints and guards").
#[derive(Clone, Debug)]
enum DataLiteral {
    /// A clock-free node of the precondition itself.
    Site(ExprId),
    /// A data predicate or data argument of a clock-reading declaration's
    /// body, read at the call: its parameters replaced by the call's
    /// arguments, every node located at the call.
    Formed(std::rc::Rc<ExpressionForm>),
    /// A data predicate of a quantifier's body read at one slot: the binder
    /// renamed to the element binding's name.
    Element(std::rc::Rc<ExpressionForm>, ElementBinding),
}

impl DataLiteral {
    /// The literal's own form, `None` for a node of the clause.
    fn form(&self) -> Option<&ExpressionForm> {
        match self {
            Self::Site(_) => None,
            Self::Formed(form) | Self::Element(form, _) => Some(form),
        }
    }
}

impl PartialEq for DataLiteral {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Site(left), Self::Site(right)) => left == right,
            (Self::Formed(left), Self::Formed(right))
            | (Self::Element(left, _), Self::Element(right, _)) => std::rc::Rc::ptr_eq(left, right),
            _ => false,
        }
    }
}

/// A disjunctive form: each part a list of data literals (literal,
/// positive) and a conjunction of clock constraints.
type Dnf = Vec<(Vec<(DataLiteral, bool)>, Vec<ClockConstraint>)>;

/// The parts of a guard's disjunctive forms, counted against the package's
/// node limit: a guard whose disjunctive form grows past it is refused, as
/// a body with that many nodes would be.
struct PartBudget {
    limit: u64,
    used: u64,
}

impl PartBudget {
    fn charge(&mut self, parts: usize) -> Result<(), CheckCause> {
        let actual = self
            .used
            .saturating_add(u64::try_from(parts).unwrap_or(u64::MAX));
        if actual > self.limit {
            return Err(CheckCause::ResourceExhausted(Box::new(
                super::refusal::StageLimitCause {
                    stage: super::refusal::CheckingStage::Typing,
                    kind: super::refusal::CheckingLimitKind::Nodes,
                    limit: self.limit,
                    actual: u128::from(actual),
                    region: None,
                },
            )));
        }
        self.used = actual;
        Ok(())
    }

    fn product(&mut self, left: &Dnf, right: &Dnf) -> Result<Dnf, CheckCause> {
        self.charge(left.len().saturating_mul(right.len()))?;
        Ok(left
            .iter()
            .flat_map(|(data, clocks)| {
                right.iter().map(move |(more_data, more_clocks)| {
                    (
                        data.iter().chain(more_data).cloned().collect(),
                        clocks.iter().chain(more_clocks).cloned().collect(),
                    )
                })
            })
            .collect())
    }
}

fn timed(cause: TemporalCause) -> CheckCause {
    CheckCause::Temporal(Box::new(cause))
}

/// The disjunctive forms, read positively and negated, of the atomic clock
/// reading at node `id`: a comparison of a place expression with a
/// constant, or a call of a clock-reading predicate, expanded. `None` when
/// the node is neither.
fn clock_atom(
    located: &Located<'_>,
    id: ExprId,
    clocks: &Clocks<'_>,
    context: EffectiveId,
    budget: &mut PartBudget,
) -> Result<Option<(Dnf, Dnf)>, CheckCause> {
    if let Some(read) = quantified::read(located, id, clocks, context, budget)? {
        return Ok(Some(read));
    }
    let expansion = reading::Expansion {
        located,
        clocks,
        declarations: clocks.declarations,
        context,
    };
    if let Some(read) = expansion.predicate(id, budget)? {
        return Ok(Some(read));
    }
    let Some(ExprNode::Binary {
        operator,
        left,
        right,
    }) = located.node(id)
    else {
        return Ok(None);
    };
    let Some(op) = ClockOp::of(*operator) else {
        return Ok(None);
    };
    let span = located.span(id);
    let (places, op, constant) = match (
        expansion.site_places(*left, budget)?,
        expansion.site_places(*right, budget)?,
    ) {
        (Some(_), Some(_)) => return Err(timed(TemporalCause::ClockComparison { span })),
        (Some(places), None) => (places, op, *right),
        (None, Some(places)) => (places, op.mirrored(), *left),
        (None, None) => return Ok(None),
    };
    if located.reads_clock(constant) {
        return Err(timed(TemporalCause::ClockComparison { span }));
    }
    let bound = located.constant(constant, clocks).map_err(timed)?;
    Ok(Some(reading::constraints(places, op, &bound, span)))
}

/// A precondition body's guard: its disjunctive form over clock constraints
/// and maximal clock-free data subterms, and the polarity of each clock
/// constraint node (true where read positively).
fn guard(
    located: &Located<'_>,
    clocks: &Clocks<'_>,
    context: EffectiveId,
    budget: &mut PartBudget,
) -> Result<(Dnf, BTreeMap<ExprId, bool>), CheckCause> {
    let len = located.form.expression.len();
    let mut positive: Vec<Option<Dnf>> = vec![None; len];
    let mut negative: Vec<Option<Dnf>> = vec![None; len];
    let mut atoms = BTreeSet::new();
    let take = |dnf: &[Option<Dnf>], id: ExprId| dnf.get(id.index()).cloned().flatten();
    // A quantifier over a clock collection reads its own body.
    let quantified = quantified::bodies(located, clocks, context);
    for node in located.form.expression.iter() {
        let id = node.id();
        if quantified.contains(&id) {
            continue;
        }
        let (pos, neg) = if !located.reads_clock(id) {
            (
                vec![(vec![(DataLiteral::Site(id), true)], Vec::new())],
                vec![(vec![(DataLiteral::Site(id), false)], Vec::new())],
            )
        } else if let Some(read) = clock_atom(located, id, clocks, context, budget)? {
            atoms.insert(id);
            read
        } else {
            match node.node() {
                ExprNode::Binary {
                    operator:
                        operator @ (BinaryOperator::And | BinaryOperator::Or | BinaryOperator::Implies),
                    left,
                    right,
                } => {
                    let (Some(lp), Some(ln), Some(rp), Some(rn)) = (
                        take(&positive, *left),
                        take(&negative, *left),
                        take(&positive, *right),
                        take(&negative, *right),
                    ) else {
                        return Err(timed(located.outside(id)));
                    };
                    match operator {
                        BinaryOperator::And => (budget.product(&lp, &rp)?, [ln, rn].concat()),
                        BinaryOperator::Or => ([lp, rp].concat(), budget.product(&ln, &rn)?),
                        _ => ([ln, rp].concat(), budget.product(&lp, &rn)?),
                    }
                }
                ExprNode::Not(operand) => {
                    let (Some(pos), Some(neg)) =
                        (take(&positive, *operand), take(&negative, *operand))
                    else {
                        return Err(timed(located.outside(id)));
                    };
                    (neg, pos)
                }
                // Not a Boolean of the guard's structure: an error only where
                // the structure reads it.
                _ => continue,
            }
        };
        positive[id.index()] = Some(pos);
        negative[id.index()] = Some(neg);
    }
    let root = located.root();
    let dnf = take(&positive, root).ok_or_else(|| timed(located.outside(root)))?;
    Ok((dnf, polarity(located, &atoms)))
}

/// Each of `atoms` with whether it is read positively: under an even number
/// of `not`s and `implies` premises.
fn polarity(located: &Located<'_>, atoms: &BTreeSet<ExprId>) -> BTreeMap<ExprId, bool> {
    let mut found = BTreeMap::new();
    let mut pending = vec![(located.root(), true)];
    while let Some((id, positive)) = pending.pop() {
        if atoms.contains(&id) {
            found.insert(id, positive);
            continue;
        }
        match located.node(id) {
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

/// The conjuncts of the top-level `and` chain, in source order.
fn conjuncts(located: &Located<'_>) -> Vec<ExprId> {
    let mut found = Vec::new();
    let mut pending = vec![located.root()];
    while let Some(id) = pending.pop() {
        match located.node(id) {
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
/// clause body or read at its call, in source order.
fn part_predicate(
    located: &Located<'_>,
    literals: &[(DataLiteral, bool)],
) -> Option<ExpressionForm> {
    let mut literals: Vec<(DataLiteral, bool)> = literals.to_vec();
    let start = |literal: &DataLiteral| match literal.form() {
        None => match literal {
            DataLiteral::Site(id) => located.span(*id).start,
            _ => 0,
        },
        Some(form) => form
            .spans
            .span(form.spans.root())
            .map_or(0, |span| span.start),
    };
    literals.sort_by_key(|(literal, _)| start(literal));
    literals.dedup();
    let operands = literals
        .into_iter()
        .map(|(literal, positive)| {
            let cut = match &literal {
                DataLiteral::Site(id) => located.form.cut(*id, &BTreeMap::new())?,
                DataLiteral::Formed(form) | DataLiteral::Element(form, _) => (**form).clone(),
            };
            if positive {
                Some(cut)
            } else {
                cut.negated()
            }
        })
        .collect::<Option<Vec<_>>>()?;
    ExpressionForm::conjunction(operands)
}

/// What one state clause contributes to its operation's timing.
enum ClauseClocks {
    /// The clause reads no clock.
    None,
    /// A precondition's guard parts.
    Guard(Vec<PartDraft>),
    /// A postcondition's resets.
    Resets(Vec<ResetDraft>),
}

/// What a postcondition's resets cut from its body: each reset conjunct,
/// replaced by `true`, and each clock component of a record literal written
/// whole, removed from the literal.
#[derive(Default)]
struct ResetCut {
    leaves: BTreeMap<ExprId, ExprNode>,
    removed: BTreeSet<ExprId>,
    /// The clock-bearing record literals the resets consumed.
    literals: BTreeSet<ExprId>,
    resets: Vec<ResetDraft>,
}

/// The resets of a postcondition: each top-level conjunct `self.x = c` that
/// writes a clock of the operation's frame, and each `self.f = R { … }`
/// writing a clock-bearing record whole, whose clock components are resets
/// and whose data components stay a data write (FR-230 "Records, tuples
/// and unions").
fn resets(
    located: &Located<'_>,
    clause: &StateClauseDeclaration,
    clocks: &Clocks<'_>,
) -> Result<ResetCut, TemporalCause> {
    let written: BTreeSet<&DeclarationKey> = clause
        .operation
        .iter()
        .flat_map(|operation| operation.declaration.effect().written_fields())
        .collect();
    let mut cut = ResetCut::default();
    for conjunct in conjuncts(located) {
        if !located.reads_clock(conjunct) {
            continue;
        }
        if let Some(reset) = quantified::reset(located, conjunct, clocks, clause.context, &written)?
        {
            cut.leaves.insert(conjunct, ExprNode::Boolean(true));
            cut.resets.push(reset);
            continue;
        }
        let span = located.span(conjunct);
        let Some(ExprNode::Binary {
            operator: BinaryOperator::Equal,
            left,
            right,
        }) = located.node(conjunct)
        else {
            return Err(nested_write(located, conjunct));
        };
        if clocks.bears(located.form, *left, clause.context) {
            let Some(ExprNode::Record { .. }) = located.node(*right) else {
                return Err(located.outside(*right));
            };
            let Some(path) = self_path(located.form, *left) else {
                return Err(located.outside(*left));
            };
            RecordWrite {
                located,
                context: clause.context,
                clocks,
                written: &written,
                span,
            }
            .resets(path, *right, &mut cut)?;
            continue;
        }
        let Some(clock) = located.clock(*left) else {
            if !located.reads_clock(*left) && located.reads_clock(*right) {
                return Err(located.outside(*right));
            }
            return Err(nested_write(located, conjunct));
        };
        if located.reads_clock(*right) {
            return Err(TemporalCause::ClockWrite { span });
        }
        let value = match located.constant(*right, clocks) {
            Ok(value) => value,
            Err(TemporalCause::ClockBoundNotConstant { .. }) => {
                return Err(TemporalCause::ClockWrite { span })
            }
            Err(cause) => return Err(cause),
        };
        let in_frame = clocks
            .fields
            .get(clock.0)
            .is_some_and(|field| written.contains(&field.field));
        if !in_frame {
            return Err(TemporalCause::ClockWrite { span });
        }
        cut.leaves.insert(conjunct, ExprNode::Boolean(true));
        cut.resets.push(ResetDraft::single(clock, value, span));
    }
    Ok(cut)
}

/// One postcondition conjunct `self.f = literal` writing a clock-bearing
/// record: the clause, the receiver's type, the model's clocks, the fields
/// the operation's frame writes and the conjunct's span.
struct RecordWrite<'w, 'f, 'c> {
    located: &'w Located<'f>,
    context: EffectiveId,
    clocks: &'w Clocks<'c>,
    written: &'w BTreeSet<&'w DeclarationKey>,
    span: Span,
}

impl RecordWrite<'_, '_, '_> {
    /// The clock places of the receiver at `path` or strictly below it.
    fn places_below<'p>(
        &'p self,
        path: &'p [PlaceStep],
    ) -> impl Iterator<Item = (usize, &'p ClockField)> + 'p {
        self.clocks
            .fields
            .iter()
            .enumerate()
            .filter(move |(_, clock)| {
                strictly_below(&clock.path, path)
                    && self.clocks.model.conforms(self.context, clock.owner)
            })
    }

    /// The resets of `self.<path> = literal`: each clock component of the
    /// record literal a reset of its place to its constant, each
    /// clock-bearing component a literal read the same way, each data
    /// component kept. A literal that leaves out a clock component writes
    /// that clock no constant, which no reset admits (`clock-write`).
    fn resets(
        &self,
        path: Vec<PlaceStep>,
        literal: ExprId,
        cut: &mut ResetCut,
    ) -> Result<(), TemporalCause> {
        let located = self.located;
        let Some(ExprNode::Record { fields, .. }) = located.node(literal) else {
            return Err(TemporalCause::ClockWrite {
                span: located.span(literal),
            });
        };
        cut.literals.insert(literal);
        let named: BTreeSet<&str> = fields.iter().map(|(name, _)| name.as_str()).collect();
        let omitted = self.places_below(&path).any(|(_, clock)| {
            clock.path.get(path.len()).is_some_and(|step| match step {
                PlaceStep::Field(component) => !named.contains(component.as_str()),
                PlaceStep::Slot(_) => false,
            })
        });
        if omitted {
            return Err(TemporalCause::ClockWrite {
                span: located.span(literal),
            });
        }
        for (name, initializer) in fields {
            let mut below = path.clone();
            below.push(PlaceStep::Field(name.clone()));
            let place = self
                .places_below(&path)
                .find(|(_, clock)| clock.path == below)
                .map(|(index, _)| index);
            let bears = self.places_below(&below).next().is_some();
            let qsl_forms::FieldInitializer::Value(value) = initializer else {
                if place.is_some() || bears {
                    return Err(TemporalCause::ClockWrite { span: self.span });
                }
                continue;
            };
            if let Some(index) = place {
                if located.reads_clock(*value) {
                    return Err(TemporalCause::ClockWrite { span: self.span });
                }
                let value_span = located.span(*value);
                let constant = match located.constant(*value, self.clocks) {
                    Ok(constant) => constant,
                    Err(TemporalCause::ClockBoundNotConstant { .. }) => {
                        return Err(TemporalCause::ClockWrite { span: value_span })
                    }
                    Err(cause) => return Err(cause),
                };
                let in_frame = self
                    .clocks
                    .fields
                    .get(index)
                    .is_some_and(|field| self.written.contains(&field.field));
                if !in_frame {
                    return Err(TemporalCause::ClockWrite { span: self.span });
                }
                cut.removed.insert(*value);
                cut.resets
                    .push(ResetDraft::single(ClockId(index), constant, value_span));
            } else if bears {
                self.resets(below, *value, cut)?;
            }
        }
        Ok(())
    }
}

/// A postcondition's clock read under `conjunct` that is no top-level reset:
/// `clock-write` at an equation `self.x = e` that sets a clock, such as one nested under
/// `implies`, and otherwise `clock-read-outside-constraint` at the read.
fn nested_write(located: &Located<'_>, conjunct: ExprId) -> TemporalCause {
    let mut pending = vec![conjunct];
    while let Some(id) = pending.pop() {
        if let Some(ExprNode::Binary {
            operator: BinaryOperator::Equal,
            left,
            ..
        }) = located.node(id)
        {
            if located.clock(*left).is_some() {
                return TemporalCause::ClockWrite {
                    span: located.span(id),
                };
            }
        }
        if let Some(node) = located.node(id) {
            pending.extend(node.children());
        }
    }
    located.outside(conjunct)
}

/// The span of the first record literal of a clock-bearing type that is not
/// one of `consumed`, the literals a reset writes: a literal with a clock
/// component is checked only as a reset (FR-230 "Records, tuples and
/// unions").
fn stray_literal(
    located: &Located<'_>,
    clocks: &Clocks<'_>,
    consumed: &BTreeSet<ExprId>,
) -> Option<Span> {
    located
        .form
        .expression
        .iter()
        .filter(|node| !consumed.contains(&node.id()))
        .filter(|node| {
            matches!(node.node(), ExprNode::Record { name, .. } if clocks.model.literal_bears_clock(name))
        })
        .map(|node| located.span(node.id()))
        .min_by_key(|span| span.start)
}

/// One state clause, its clocks read out: a precondition's guard parts or a
/// postcondition's resets, and the clause body with each clock constraint
/// and reset replaced by the Boolean it contributes to the data reading.
fn read_clause(
    clause: &mut StateClauseDeclaration,
    clocks: &Clocks<'_>,
    budget: &mut PartBudget,
) -> Result<ClauseClocks, CheckCause> {
    let form = ExpressionForm {
        expression: clause.body.clone(),
        spans: clause.spans.body.clone(),
    };
    let Some(located) = Located::of(&form, clocks, clause.context) else {
        return Ok(ClauseClocks::None);
    };
    let root = located.root();
    if !located.reads_clock(root) {
        if let Some(span) = stray_literal(&located, clocks, &BTreeSet::new()) {
            return Err(timed(TemporalCause::ClockWrite { span }));
        }
        return Ok(ClauseClocks::None);
    }
    let mut removed = BTreeSet::new();
    let (leaves, read) = match clause.kind {
        StateClauseKind::Invariant => return Err(timed(located.outside(root))),
        StateClauseKind::Precondition => {
            if let Some(span) = stray_literal(&located, clocks, &BTreeSet::new()) {
                return Err(timed(TemporalCause::ClockWrite { span }));
            }
            let (dnf, polarity) = guard(&located, clocks, clause.context, budget)?;
            let parts = dnf
                .into_iter()
                .map(|(literals, clocks)| {
                    let data = if literals.is_empty() {
                        None
                    } else {
                        Some(
                            part_predicate(&located, &literals)
                                .ok_or_else(|| timed(located.outside(root)))?,
                        )
                    };
                    let mut elements: Vec<ElementBinding> = Vec::new();
                    for (literal, _) in &literals {
                        if let DataLiteral::Element(_, binding) = literal {
                            if !elements.contains(binding) {
                                elements.push(binding.clone());
                            }
                        }
                    }
                    elements.sort_by_key(|binding| binding.slot);
                    Ok(PartDraft {
                        data,
                        elements,
                        clocks,
                    })
                })
                .collect::<Result<Vec<_>, CheckCause>>()?;
            let leaves = polarity
                .into_iter()
                .map(|(id, positive)| (id, ExprNode::Boolean(positive)))
                .collect();
            (leaves, ClauseClocks::Guard(parts))
        }
        StateClauseKind::Postcondition => {
            let cut = resets(&located, clause, clocks).map_err(timed)?;
            if let Some(span) = stray_literal(&located, clocks, &cut.literals) {
                return Err(timed(TemporalCause::ClockWrite { span }));
            }
            removed = cut.removed;
            (cut.leaves, ClauseClocks::Resets(cut.resets))
        }
    };
    let rewritten = form
        .cut_removing(root, &leaves, &removed)
        .ok_or_else(|| timed(located.outside(root)))?;
    clause.body = rewritten.expression;
    clause.spans.body = rewritten.spans;
    Ok(read)
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
    let population =
        population_of(models, *context, types).map_err(|_| TemporalCause::TimedStateObject {
            ambiguous: true,
            span,
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
    if let Some(located) = Located::of(predicate, clocks, context) {
        if located.reads_clock(located.root()) {
            return Err(located.outside(located.root()));
        }
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
    let bounds =
        form.bounds
            .iter()
            .map(|bound| {
                let root = bound.clock.expression.root_id();
                let clock = clocks.read(&bound.clock, root, context).ok_or(
                    TemporalCause::UnknownClock {
                        span: bound
                            .clock
                            .spans
                            .span(bound.clock.spans.root())
                            .unwrap_or(bound.span),
                    },
                )?;
                Ok(ClockConstraint {
                    clock,
                    via: Vec::new(),
                    slots: Vec::new(),
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
/// The located refusal of the first ill-formed timed member: a clock of a
/// model that is not dense, then, clause by clause, a clock read outside an
/// atomic clock constraint or a clock write other than a reset of the
/// frame, then a time invariant or `urgent` declaration with no timed
/// model or an ill-formed bound, operation or predicate. A guard whose
/// disjunctive form outgrows the node limit is refused as resource
/// exhausted.
pub(crate) fn prepare(
    inputs: &Inputs<'_>,
    state_clauses: &mut [StateClauseDeclaration],
) -> Result<TimedDrafts, CheckRefusal> {
    // FR-146 over the clock-reading declarations, which have no function
    // node for the value checker's measure check to reach (FR-091-AC-30).
    if let Some((cycle, name)) = reading::unmeasured_cycle(inputs.clock_reading) {
        return Err(CheckRefusal {
            location: quire_semantic_value::location::Location::root(
                quire_semantic_value::location::Origin::TypeDeclaration {
                    name: clock_reading_site(name),
                },
            ),
            cause: CheckCause::UnprovedDecrease {
                cycle,
                obligation: super::refusal::MeasureObligation::MissingMeasure,
            },
        });
    }
    prepare_models(inputs, state_clauses).map_err(refusal)
}

fn prepare_models(
    inputs: &Inputs<'_>,
    state_clauses: &mut [StateClauseDeclaration],
) -> Result<TimedDrafts, CheckCause> {
    let mut budget = PartBudget {
        limit: inputs.nodes,
        used: 0,
    };
    // FR-230 "Clock-reading declarations": each body is checked once. A call
    // this check does not expand reaches the value checker, which refuses it
    // (`CheckCause::ClockReadingCall`).
    let declarations = reading::Declarations::new(inputs.clock_reading, inputs.models);
    for declaration in inputs.clock_reading {
        reading::check_declaration(declaration, &declarations).map_err(timed)?;
    }
    for body in inputs.operation_bodies {
        if let Some(span) = reading::first_call(&declarations, &body.body, &body.spans.body) {
            return Err(timed(TemporalCause::ClockReadOutsideConstraint { span }));
        }
    }
    let source_of = |model: &AdmittedModel| {
        model.alias().and_then(|alias| {
            inputs
                .time_sources
                .iter()
                .find(|source| source.model == alias)
        })
    };
    let clocks_of = |model| {
        let fields = AdmittedModel::clock_places(model)
            .iter()
            .map(|place| ClockField {
                owner: place.owner,
                field: place.field.clone(),
                name: place.name(),
                path: place.path.clone(),
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
            declarations: &declarations,
        }
    };
    // A clock of a tick model refuses at its time declaration; one of an
    // untimed model at its first read, or at the model's declaration when no
    // clause reads it.
    for model in inputs.models {
        let clocks = clocks_of(model);
        let Some(field) = clocks.fields.first() else {
            continue;
        };
        let span = match source_of(model) {
            Some(source) if matches!(source.source, TimeSourceKind::Dense) => {
                // A place under an optional member is guarded by a presence
                // FR-230 does not yet give S3 a reading of (STD-493 item
                // 17).
                if AdmittedModel::clock_places(model)
                    .iter()
                    .any(|place| place.optional)
                {
                    return Err(CheckCause::TimedNotChecked {
                        construct: TimedConstruct::Clock,
                        span: source.span,
                    });
                }
                continue;
            }
            Some(source) => Some(source.span),
            None => state_clauses
                .iter()
                .find_map(|clause| {
                    let form = ExpressionForm {
                        expression: clause.body.clone(),
                        spans: clause.spans.body.clone(),
                    };
                    Located::of(&form, &clocks, clause.context)
                        .and_then(|located| located.first(located.root()))
                })
                .or_else(|| {
                    model
                        .alias()
                        .and_then(|alias| inputs.model_spans.get(alias))
                        .copied()
                }),
        };
        if let Some(span) = span {
            return Err(timed(TemporalCause::ClockWithoutDenseTime {
                field: field.name.clone(),
                span,
            }));
        }
    }
    let declarations: Vec<&TimedDeclaration> = inputs
        .timed_forms
        .iter()
        .filter(|form| {
            matches!(
                form,
                TimedDeclaration::TimeInvariant(_) | TimedDeclaration::Urgent(_)
            )
        })
        .collect();
    if let Some(form) = declarations.first() {
        match (inputs.time_sources, form) {
            ([], TimedDeclaration::TimeInvariant(form)) => {
                return Err(timed(TemporalCause::TimeInvariantWithoutTimeSource {
                    span: form.span,
                }))
            }
            ([], TimedDeclaration::Urgent(form)) => {
                return Err(timed(TemporalCause::UrgencyWithoutTimeSource {
                    span: form.span,
                }))
            }
            ([_], _) | ([], _) => {}
            (_, form) => {
                let span = match form {
                    TimedDeclaration::TimeInvariant(form) => form.span,
                    TimedDeclaration::Urgent(form) => form.span,
                    TimedDeclaration::TimeSource(_) | TimedDeclaration::ClockType(_) => form.span(),
                };
                return Err(timed(TemporalCause::AmbiguousTimedDeclaration { span }));
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
            match read_clause(clause, &clocks, &mut budget)? {
                ClauseClocks::None => {}
                ClauseClocks::Guard(parts) => {
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
                ClauseClocks::Resets(written) => {
                    if let (false, Some(operation)) = (written.is_empty(), &clause.operation) {
                        resets.push(ResetsDraft {
                            clause: clause.name.clone(),
                            operation: operation.clone(),
                            context: clause.context,
                            resets: written,
                        });
                    }
                }
            }
        }
        let mut invariants = Vec::new();
        let mut urgency_drafts = Vec::new();
        for form in &declarations {
            let span = match form {
                TimedDeclaration::TimeInvariant(form) => form.span,
                TimedDeclaration::Urgent(form) => form.span,
                TimedDeclaration::TimeSource(_) | TimedDeclaration::ClockType(_) => continue,
            };
            let state = || state_object(model, inputs.models, inputs.types, span);
            match form {
                TimedDeclaration::TimeInvariant(form) => {
                    let (context, population) = state().map_err(timed)?;
                    invariants
                        .push(time_invariant(form, &clocks, context, population).map_err(timed)?);
                }
                TimedDeclaration::Urgent(form) => {
                    urgency_drafts
                        .push(urgency(form, model, &clocks, state, &guarded).map_err(timed)?);
                }
                TimedDeclaration::TimeSource(_) | TimedDeclaration::ClockType(_) => {}
            }
        }
        let dimensions = model
            .object_types()
            .map(|(_, object)| {
                let dimension = clocks
                    .fields
                    .iter()
                    .filter(|clock| model.conforms(object, clock.owner))
                    .filter_map(ClockField::bound)
                    .try_fold(0_u64, u64::checked_add);
                (object, dimension)
            })
            .collect();
        models.push(ModelDraft {
            model: source.model.clone(),
            span: source.span,
            clocks: clocks.fields,
            dimensions,
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

/// The element type of the collection field `field` of `context`, as the
/// clause checker types it.
fn element_type(
    checking: &Checking<'_, '_>,
    context: EffectiveId,
    field: &str,
    span: Span,
) -> Result<ValueType, CheckRefusal> {
    checking
        .declarations
        .scope
        .types()
        .attribute(context, field)
        .and_then(|attribute| match attribute.field().value_type() {
            ValueType::Collection(collection) => Some(collection.element().clone()),
            _ => None,
        })
        .ok_or_else(|| {
            refusal(CheckCause::TimedNotChecked {
                construct: TimedConstruct::Clock,
                span,
            })
        })
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
        refusal(timed(TemporalCause::TimedPredicate {
            cause: Box::new(refused.cause),
            span,
        }))
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
        binders: &[],
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
                // Each element a part reads, typed as an element of the
                // receiver's collection field.
                let binders = part
                    .elements
                    .iter()
                    .map(|binding| {
                        let span = clause
                            .spans
                            .body
                            .span(clause.spans.body.root())
                            .unwrap_or(Span { start: 0, end: 0 });
                        let element = element_type(checking, clause.context, &binding.field, span)?;
                        Ok((binding.name.clone(), element))
                    })
                    .collect::<Result<Vec<_>, CheckRefusal>>()?;
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
                            binders: &binders,
                            body: &form.expression,
                        };
                        let (predicate, used) = typed(&view, form, checking, cx)?;
                        nodes_used = nodes_used.max(used);
                        Some(predicate)
                    }
                };
                checked.push(ConvexPart {
                    data,
                    elements: part.elements,
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
        let mut resets = Vec::with_capacity(draft.resets.len());
        for written in draft.resets {
            let mut checked = Vec::with_capacity(written.resets.len());
            for reset in written.resets {
                let selection = match reset.selection {
                    None => None,
                    Some((field, name, predicate)) => {
                        let predicate = match predicate {
                            None => None,
                            Some(form) => {
                                let element =
                                    element_type(checking, written.context, &field, reset.span)?;
                                let binders = [(name.clone(), element)];
                                let view = ClauseView {
                                    kind: StateClauseKind::Precondition,
                                    complete_value: checking.complete_value,
                                    context: Some(written.context),
                                    receiver: Some(("self", ValueType::Reference(written.context))),
                                    operation: None,
                                    context_population: None,
                                    frame_population: None,
                                    binders: &binders,
                                    body: &form.expression,
                                };
                                let (predicate, used) = typed(&view, &form, checking, cx)?;
                                nodes_used = nodes_used.max(used);
                                Some(predicate)
                            }
                        };
                        Some(SlotSelection {
                            field,
                            name,
                            predicate,
                        })
                    }
                };
                checked.push(ClockReset {
                    clock: reset.clock,
                    value: reset.value,
                    span: reset.span,
                    selection,
                });
            }
            resets.push(ClockResets {
                clause: written.clause,
                operation: written.operation,
                resets: checked,
            });
        }
        models.push(TimedModel {
            model: draft.model,
            span: draft.span,
            clocks: draft.clocks,
            dimensions: draft.dimensions,
            guards,
            resets,
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
    refusal(timed(TemporalCause::TimedModelNotEmitted {
        model: model.model.clone(),
        span: model.span,
    }))
}
