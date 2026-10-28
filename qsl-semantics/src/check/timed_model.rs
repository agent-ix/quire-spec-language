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
use quire_semantic_value::declaration::{CompositeShape, TypeEnvironment};

pub use self::collection_write::{CollectionWrite, Written};
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

mod collection_write;
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
    /// The clock aliases the clock is typed by, each alias of an alias on
    /// the chain.
    aliases: Vec<DeclarationKey>,
    /// Whether the place is an `Option<Clock>`, whose presence is a bit of
    /// the state (FR-230 "Presence bit").
    bit: bool,
    /// Whether reading the place needs its presence: an `Option<Clock>`, or
    /// a union case's payload, present while the value is that case.
    guarded: bool,
    /// The place's `resolved_path` rows, one for each path step.
    rows: Vec<super::lowering::PlaceRow>,
}

impl ClockField {
    /// Whether the place is an `Option<Clock>` field: its presence is a bit
    /// of each object's state, which `present(self.f)` reads, a write of a
    /// constant sets and `absent` clears (FR-230 "Presence bit").
    pub fn presence_bit(&self) -> bool {
        self.bit
    }

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

    /// The `resolved_path` rows of the place's `clock_place` template node,
    /// one for each path step (QSpec FR-322 "Timed-model nodes").
    pub fn resolved_path(&self) -> &[super::lowering::PlaceRow] {
        &self.rows
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
            PlaceStep::Field(_) | PlaceStep::Union(_) => Some(count),
            PlaceStep::Slot(shape) => count.checked_mul(shape.max?),
        })
    }

    /// The collection shapes on the place's path, outermost first.
    pub fn slots(&self) -> impl Iterator<Item = &CollectionShape> {
        self.path.iter().filter_map(|step| match step {
            PlaceStep::Slot(shape) => Some(shape),
            PlaceStep::Field(_) | PlaceStep::Union(_) => None,
        })
    }
}

/// The comparison of an atomic clock constraint `x ~ c` (FR-230
/// `ClockOp`).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
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

/// The slot of a clock collection a guard part reads: one held slot, or
/// the template slot of a [`QuantifiedFamily`], which the reading
/// instantiates at each slot the collection holds.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SlotRef {
    /// The slot at this index.
    Held(u64),
    /// Every held slot in turn.
    Family,
}

/// An atomic clock constraint `x ~ c` over the clock `x` of `self`, or of
/// the object `self` reaches through the reference fields `via`, its bound
/// exact in the model's unit (FR-230 `ClockConstraint`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockConstraint {
    clock: ClockId,
    via: Vec<PlaceStep>,
    slots: Vec<SlotRef>,
    op: ClockOp,
    bound: Rational,
    span: Span,
    /// The position of the operation's `Clock` parameter the constraint
    /// reads (FR-230 "Operation parameters"): it constrains the place an
    /// instance binds that parameter to, when that place is of `clock`.
    parameter: Option<usize>,
}

impl ClockConstraint {
    /// The position of the operation's `Clock` parameter this constrains,
    /// when it reads one: the place an instance binds the parameter to,
    /// when that place is a place of [`Self::clock`].
    pub fn parameter(&self) -> Option<usize> {
        self.parameter
    }

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
    /// quantified collection (FR-230 "Quantified reads"), or the template
    /// slot of a family.
    pub fn slots(&self) -> &[SlotRef] {
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
#[derive(Clone, Debug, PartialEq)]
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
/// bound to the element the receiver's collection holds at the slot. The
/// collection is a field of the receiver or a collection below a record
/// field of it, named by its path of field names.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ElementBinding {
    name: String,
    path: Vec<String>,
    slot: SlotRef,
}

impl ElementBinding {
    /// The element the receiver's collection at `path` holds at `slot`,
    /// read by the parameter `name`.
    pub fn new(name: String, path: Vec<String>, slot: SlotRef) -> Self {
        Self { name, path, slot }
    }

    /// The parameter name the data predicate reads the element by.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The path of field names from the receiver to the collection.
    pub fn path(&self) -> &[String] {
        &self.path
    }

    /// The slot.
    pub fn slot(&self) -> SlotRef {
        self.slot
    }
}

/// One convex part of an operation's guard: its data predicate, `None` for
/// `true`, and a conjunction of atomic clock constraints.
#[derive(Debug)]
pub struct ConvexPart {
    data: Option<CheckedTimedPredicate>,
    presences: Vec<(ClockId, bool)>,
    elements: Vec<ElementBinding>,
    clocks: Vec<ClockConstraint>,
    families: Vec<QuantifiedFamily>,
}

/// A quantifier over a clock collection with no declared maximum, read as
/// the family of its entries indexed by size (FR-230 "Quantified reads"):
/// at a state whose collection holds `n` elements, either every slot below
/// `n` enables one of [`Self::parts`] (`each`), or some slot below `n` does.
/// The parts are templates whose element bindings and clock constraints
/// name [`SlotRef::Family`], read at each slot in turn.
#[derive(Debug)]
pub struct QuantifiedFamily {
    path: Vec<String>,
    each: bool,
    parts: Vec<ConvexPart>,
}

impl QuantifiedFamily {
    /// The path of field names from the receiver to the collection the
    /// quantifier ranges over.
    pub fn path(&self) -> &[String] {
        &self.path
    }

    /// Whether every held slot must enable a part (a `forall` read
    /// positively, an `exists` negated), rather than some slot.
    pub fn each(&self) -> bool {
        self.each
    }

    /// The template parts of the body at one slot.
    pub fn parts(&self) -> &[ConvexPart] {
        &self.parts
    }
}

impl ConvexPart {
    /// The part's data predicate; `None` when the part reads no data.
    pub fn data(&self) -> Option<&CheckedTimedPredicate> {
        self.data.as_ref()
    }

    /// Each `Option<Clock>` place of the receiver whose presence the part
    /// requires, and whether present (FR-230 "Presence bit"): a data
    /// predicate over the bit, read at the state.
    pub fn presences(&self) -> &[(ClockId, bool)] {
        &self.presences
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

    /// The quantifier families the part conjoins, each read at the state.
    pub fn families(&self) -> &[QuantifiedFamily] {
        &self.families
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
#[derive(Clone, Debug, PartialEq)]
pub struct ClockReset {
    clock: ClockId,
    value: Rational,
    span: Span,
    selection: Option<SlotSelection>,
    /// The position of the operation's `Clock` parameter the reset writes,
    /// when it writes one (FR-230 "Operation parameters").
    parameter: Option<usize>,
}

/// The slots a reset `forall(e in self.f: P(e) implies e.x = c)` writes:
/// each slot the collection holds at the pre-state whose element satisfies
/// `P`, every held slot when `P` is absent. A slot not selected keeps its
/// value.
#[derive(Clone, Debug, PartialEq)]
pub struct SlotSelection {
    path: Vec<String>,
    name: String,
    predicate: Option<CheckedTimedPredicate>,
}

impl SlotSelection {
    /// The path of field names from the receiver to the collection.
    pub fn path(&self) -> &[String] {
        &self.path
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
    /// The position of the operation's `Clock` parameter this resets, when
    /// it resets one: the place an instance binds the parameter to, when
    /// that place is a place of [`Self::clock`].
    pub fn parameter(&self) -> Option<usize> {
        self.parameter
    }

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

/// One `Clock` parameter of an operation and its range (FR-230 "Operation
/// parameters"): the clock fields whose places an instance of the operation
/// binds it to.
#[derive(Clone, Debug, PartialEq)]
pub struct ClockParameterRange {
    operation: ClauseOperation,
    position: usize,
    range: Vec<ClockId>,
    receiver: Vec<ClockId>,
}

impl ClockParameterRange {
    /// The operation.
    pub fn operation(&self) -> &ClauseOperation {
        &self.operation
    }

    /// The parameter's position among the operation's declared parameters.
    pub fn position(&self) -> usize {
        self.position
    }

    /// The clock fields an instance binds the parameter to a place of.
    pub fn range(&self) -> &[ClockId] {
        &self.range
    }

    /// The fields of [`Self::range`] the frame names for the receiver only
    /// (`modifies self.f`, FR-103): an instance binds the parameter to
    /// their places on the operation's receiver, never another object's.
    pub fn receiver_only(&self) -> &[ClockId] {
        &self.receiver
    }
}

/// The resets one postcondition writes.
#[derive(Clone, Debug, PartialEq)]
pub struct ClockResets {
    clause: String,
    operation: ClauseOperation,
    resets: Vec<ClockReset>,
    writes: Vec<CollectionWrite>,
    drops: Vec<ClockId>,
    requires: Vec<(ClockId, bool)>,
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

    /// The writes of clock collection fields, in source order (FR-230
    /// "Collection-valued writes").
    pub fn writes(&self) -> &[CollectionWrite] {
        &self.writes
    }

    /// The `Option<Clock>` places of the receiver the postcondition drops,
    /// `self.v = absent` (FR-230 "Presence bit"): each becomes absent, its
    /// clock unconstrained, and no clock is reset.
    pub fn drops(&self) -> &[ClockId] {
        &self.drops
    }

    /// The presences the postcondition requires of the receiver's guarded
    /// places in the post-state, each top-level conjunct `present(self.v)`
    /// (present), `self.v != absent` (present) or `not present(self.v)`
    /// (absent) (FR-230 "Presence bit"): a successor whose presences differ
    /// does not satisfy the postcondition.
    pub fn requires(&self) -> &[(ClockId, bool)] {
        &self.requires
    }
}

/// `time invariant N when { P } { x <= c and ... }`: at a state where `P`
/// holds of an object, time may pass only while the bounds hold of its
/// clocks.
#[derive(Debug)]
pub struct TimeInvariant {
    name: String,
    context: EffectiveId,
    alternatives: Vec<StateAlternative>,
    bounds: Vec<ClockConstraint>,
}

/// One disjunct of a state predicate that reads the presence of
/// `Option<Clock>` places (FR-230 "Presence bit"): the presences it
/// requires of `self`'s places and its data predicate. The predicate holds
/// of an object when one of its alternatives does.
#[derive(Debug)]
pub struct StateAlternative {
    presences: Vec<(ClockId, bool)>,
    predicate: CheckedTimedPredicate,
}

impl StateAlternative {
    /// Each `Option<Clock>` place of `self` whose presence the alternative
    /// requires, and whether present.
    pub fn presences(&self) -> &[(ClockId, bool)] {
        &self.presences
    }

    /// The alternative's data predicate.
    pub fn predicate(&self) -> &CheckedTimedPredicate {
        &self.predicate
    }
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

    /// The `when` predicate's alternatives: one with no presences for a
    /// predicate that reads no `Option<Clock>` presence, else its
    /// disjunctive form over those presences (FR-230 "Presence bit").
    pub fn alternatives(&self) -> &[StateAlternative] {
        &self.alternatives
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
        /// The predicate's alternatives ([`TimeInvariant::alternatives`]).
        alternatives: Vec<StateAlternative>,
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
    /// The `Clock` parameters of the operations whose clauses the model
    /// reads, each with its range.
    clock_parameters: Vec<ClockParameterRange>,
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

    /// The `Clock` parameters of the operations the model's clauses
    /// constrain, each with its range; an operation's parameter listed
    /// nowhere ranges over every clock field.
    pub fn clock_parameters(&self) -> &[ClockParameterRange] {
        &self.clock_parameters
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

    /// Whether the model has a member no S4 emission writes yet: a guard,
    /// reset, time invariant, urgency or `Clock` parameter. Its clock
    /// places alone are written as `clock_place` templates (QSpec FR-322
    /// "Timed-model nodes"); a model with a guard is refused rather than
    /// written with an empty `guards` aggregate (FR-230-AC-50).
    pub fn has_unwritten_members(&self) -> bool {
        !self.guards.is_empty()
            || !self.resets.is_empty()
            || !self.invariants.is_empty()
            || !self.urgency.is_empty()
            || !self.clock_parameters.is_empty()
    }
}

/// One guard part before its data predicate is typed.
struct PartDraft {
    data: Option<ExpressionForm>,
    /// Each `Option<Clock>` place's presence the part requires, and whether
    /// present.
    presences: Vec<(ClockId, bool)>,
    /// Each element the data predicate reads, with its quantifier's span.
    elements: Vec<(ElementBinding, Span)>,
    clocks: Vec<ClockConstraint>,
    /// Each quantifier family the part conjoins: its field, whether every
    /// slot must enable a part, and its template parts.
    families: Vec<(Vec<String>, bool, Vec<PartDraft>)>,
}

/// An unbounded quantifier's body read at [`SlotRef::Family`]:
/// the family of its entries indexed by size (FR-230 "Quantified reads").
#[derive(Debug)]
struct FamilyDraft {
    forall: bool,
    path: Vec<String>,
    positive: Dnf,
    negative: Dnf,
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
        alternatives: Vec<AlternativeDraft>,
    },
    Operation {
        name: String,
        operation: DeclarationKey,
    },
}

struct InvariantDraft {
    name: String,
    alternatives: Vec<AlternativeDraft>,
    bounds: Vec<ClockConstraint>,
}

/// A [`StateAlternative`] before its data predicate is typed.
struct AlternativeDraft {
    presences: Vec<(ClockId, bool)>,
    predicate: PredicateDraft,
}

struct ModelDraft {
    model: String,
    span: Span,
    clocks: Vec<ClockField>,
    clock_parameters: Vec<ClockParameterRange>,
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
    selection: Option<(Vec<String>, String, Option<ExpressionForm>)>,
    /// The `Clock` parameter's position, of a reset of one.
    parameter: Option<usize>,
}

impl ResetDraft {
    /// A reset of one clock.
    fn single(clock: ClockId, value: Rational, span: Span) -> Self {
        Self {
            clock,
            value,
            span,
            selection: None,
            parameter: None,
        }
    }
}

/// One `Clock` parameter of a clause's operation (FR-230 "Operation
/// parameters"): its position among the operation's declared parameters,
/// its name and its range, the clock fields whose places an instance binds
/// it to.
#[derive(Clone, Debug)]
struct ClockParameter {
    position: usize,
    name: String,
    range: Vec<ClockId>,
    /// The fields of `range` the frame names for the receiver only.
    receiver: Vec<ClockId>,
}

/// The resets of one postcondition before they are typed.
struct ResetsDraft {
    clause: String,
    operation: ClauseOperation,
    context: EffectiveId,
    resets: Vec<ResetDraft>,
    writes: Vec<CollectionWrite>,
    drops: Vec<ClockId>,
    requires: Vec<(ClockId, bool)>,
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
        // `value(self.v)` reads an `Option<Clock>`'s clock; `self.v` alone
        // is its option, no clock.
        let (path, bit) = match form.expression.get(id)?.node() {
            ExprNode::Value(operand) => (self_path(form, *operand)?, true),
            _ => (self_path(form, id)?, false),
        };
        self.fields
            .iter()
            .position(|clock| {
                same_place(&clock.path, &path)
                    && clock.guarded == bit
                    && self.model.conforms(context, clock.owner)
            })
            .map(ClockId)
    }

    /// The `Option<Clock>` place the path `self.v` names at `id` of `form`,
    /// for a clause about `context`: the option, read by `present`, written
    /// by a reset or `absent` (FR-230 "Presence bit").
    fn option(&self, form: &ExpressionForm, id: ExprId, context: EffectiveId) -> Option<ClockId> {
        let path = self_path(form, id)?;
        self.fields
            .iter()
            .position(|clock| {
                clock.guarded
                    && same_place(&clock.path, &path)
                    && self.model.conforms(context, clock.owner)
            })
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

/// The clock fields of `model`'s clock places.
fn clock_fields(model: &AdmittedModel) -> Vec<ClockField> {
    AdmittedModel::clock_places(model)
        .iter()
        .map(|place| ClockField {
            owner: place.owner,
            field: place.field.clone(),
            name: place.name(),
            path: place.path.clone(),
            aliases: place.aliases.clone(),
            bit: place.bit,
            guarded: place.bit || place.case,
            rows: place.rows(),
        })
        .collect()
}

/// QSpec FR-430: the span of the first clock read in `form`, an expression
/// about an object of `model`'s type `context`, by FR-230's reading of the
/// clock places of the object: `self.x` for a clock place `x`, a
/// clock-bearing value read whole, a call of one of `clock_reading`, or a
/// clock read through another object; never a data field of a
/// clock-bearing record (`self.t.n`), nor a field that is a clock of no
/// place of the object. `None` when it reads none.
pub(crate) fn first_clock_read(
    form: &ExpressionForm,
    model: &AdmittedModel,
    models: &[AdmittedModel],
    clock_reading: &[super::assemble::ClockReadingDeclaration],
    context: EffectiveId,
) -> Option<Span> {
    let units = TimeUnits::default();
    let declarations = reading::Declarations::new(clock_reading, models);
    let clocks = Clocks {
        model,
        fields: clock_fields(model),
        units: &units,
        target: None,
        rational_bits: 0,
        declarations: &declarations,
    };
    Located::of(form, &clocks, context)?.first(form.expression.root_id())
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
            // A read path names a union case as a field (`self.state.Armed`).
            (
                PlaceStep::Field(left) | PlaceStep::Union(left),
                PlaceStep::Field(right) | PlaceStep::Union(right),
            ) => left == right,
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
    /// The `Option<Clock>` place a node `self.v` names (FR-230 "Presence
    /// bit").
    option: Vec<Option<ClockId>>,
    /// The model's `Option<Clock>` places.
    bits: Vec<ClockId>,
    /// The `Clock` parameter a node names, by its index in `parameters`.
    parameter: Vec<Option<usize>>,
    /// The clause's operation's `Clock` parameters.
    parameters: Vec<ClockParameter>,
    /// The span of the first clock read under each node.
    first: Vec<Option<Span>>,
    /// Whether the first read under each node is a presence read of an
    /// `Option<Clock>` place (`present(p)`, `p = absent`, `p != absent`):
    /// a data read of its presence, never a clock read.
    presence_first: Vec<bool>,
}

impl<'f> Located<'f> {
    /// `form`'s clock reads, for a clause about `context`; `None` when its
    /// spans do not fit it.
    fn of(form: &'f ExpressionForm, clocks: &Clocks<'_>, context: EffectiveId) -> Option<Self> {
        Self::with_parameters(form, clocks, context, Vec::new())
    }

    /// [`Self::of`] for a clause of an operation with the `Clock`
    /// parameters `parameters`, each a place the clause reads by its name.
    fn with_parameters(
        form: &'f ExpressionForm,
        clocks: &Clocks<'_>,
        context: EffectiveId,
        parameters: Vec<ClockParameter>,
    ) -> Option<Self> {
        let spans = form.spans.of_nodes(&form.expression)?;
        let mut clock = vec![None; form.expression.len()];
        let mut parameter = vec![None; form.expression.len()];
        let mut option = vec![None; form.expression.len()];
        let mut first: Vec<Option<Span>> = vec![None; form.expression.len()];
        let mut presence_first = vec![false; form.expression.len()];
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
            // A presence read of an `Option<Clock>` place, of `self` or of
            // another object: a data read of its presence (FR-230
            // "Presence bit"). Another object's place is a field read
            // already taken as a clock read, named like an `Option<Clock>`
            // field.
            let option_place = |operand: ExprId| {
                option.get(operand.index()).copied().flatten().is_some()
                    || clocks.option(form, operand, context).is_some()
                    || (first.get(operand.index()).copied().flatten().is_some()
                        && matches!(
                            form.expression.get(operand).map(|node| node.node()),
                            Some(ExprNode::Field { field, .. })
                                if clocks.fields.iter().any(|clock| clock.bit
                                    && clock.path == [PlaceStep::Field(field.clone())])
                        ))
            };
            let presence = match node.node() {
                ExprNode::Present(operand) => option_place(*operand),
                ExprNode::Binary {
                    operator: BinaryOperator::Equal | BinaryOperator::NotEqual,
                    left,
                    right,
                } => {
                    let absent = |side: ExprId| {
                        matches!(
                            form.expression.get(side).map(|node| node.node()),
                            Some(ExprNode::Absent)
                        )
                    };
                    (option_place(*left) && absent(*right))
                        || (absent(*left) && option_place(*right))
                }
                _ => false,
            };
            if presence {
                first[id.index()] = spans.get(id.index()).copied();
                presence_first[id.index()] = true;
                continue;
            }
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
            if let Some(read) = clocks.option(form, id, context) {
                option[id.index()] = Some(read);
                first[id.index()] = spans.get(id.index()).copied();
                continue;
            }
            if let ExprNode::Name(name) = node.node() {
                if let (1, Some(at)) = (
                    name.segments.len(),
                    parameters
                        .iter()
                        .position(|declared| declared.name == name.name),
                ) {
                    parameter[id.index()] = Some(at);
                    first[id.index()] = spans.get(id.index()).copied();
                    continue;
                }
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
            let reading = node
                .node()
                .children()
                .into_iter()
                .find(|child| first.get(child.index()).copied().flatten().is_some());
            if let Some(child) = reading {
                first[id.index()] = first.get(child.index()).copied().flatten();
                presence_first[id.index()] =
                    presence_first.get(child.index()).copied().unwrap_or(false);
            }
        }
        Some(Self {
            form,
            spans,
            clock,
            option,
            bits: clocks
                .fields
                .iter()
                .enumerate()
                .filter(|(_, field)| field.guarded)
                .map(|(index, _)| ClockId(index))
                .collect(),
            parameter,
            parameters,
            first,
            presence_first,
        })
    }

    /// Whether `clock` is an `Option<Clock>` place.
    fn bit(&self, clock: ClockId) -> bool {
        self.bits.contains(&clock)
    }

    /// The `Option<Clock>` place node `id` names, `self.v`.
    fn option(&self, id: ExprId) -> Option<ClockId> {
        self.option.get(id.index()).copied().flatten()
    }

    /// The `Option<Clock>` place `present(self.v)` at `id` reads.
    fn presence(&self, id: ExprId) -> Option<ClockId> {
        match self.node(id)? {
            ExprNode::Present(operand) => self.option(*operand),
            _ => None,
        }
    }

    /// The `Clock` parameter node `id` names.
    fn parameter(&self, id: ExprId) -> Option<&ClockParameter> {
        let at = self.parameter.get(id.index()).copied().flatten()?;
        self.parameters.get(at)
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
        let span = self.first(id).unwrap_or_else(|| self.span(id));
        // A presence read where this slice reads none: no clock is read.
        if self
            .presence_first
            .get(id.index())
            .copied()
            .unwrap_or(false)
        {
            return TemporalCause::PresenceNotChecked { span };
        }
        TemporalCause::ClockReadOutsideConstraint { span }
    }

    /// The `Option<Clock>` place and the presence `p = absent` or
    /// `p != absent` at `id` tests: present for `!=`.
    fn absent_test(&self, id: ExprId) -> Option<(ClockId, bool)> {
        let Some(ExprNode::Binary {
            operator,
            left,
            right,
        }) = self.node(id)
        else {
            return None;
        };
        let present = match operator {
            BinaryOperator::Equal => false,
            BinaryOperator::NotEqual => true,
            _ => return None,
        };
        let place = match (self.node(*left), self.node(*right)) {
            (_, Some(ExprNode::Absent)) => self.option(*left)?,
            (Some(ExprNode::Absent), _) => self.option(*right)?,
            _ => return None,
        };
        Some((place, present))
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
    /// The span is the quantifier's.
    Element(std::rc::Rc<ExpressionForm>, ElementBinding, Span),
    /// A quantifier over a collection with no declared maximum, read as the
    /// family of its entries indexed by size: it holds or fails at a state.
    Family(std::rc::Rc<FamilyDraft>),
    /// `present(self.v)` of an `Option<Clock>` place: its presence bit
    /// (FR-230 "Presence bit").
    Presence(ClockId),
}

impl DataLiteral {
    /// The literal's own form, `None` for a node of the clause.
    fn form(&self) -> Option<&ExpressionForm> {
        match self {
            Self::Site(_) | Self::Family(_) | Self::Presence(_) => None,
            Self::Formed(form) | Self::Element(form, ..) => Some(form),
        }
    }
}

impl PartialEq for DataLiteral {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Site(left), Self::Site(right)) => left == right,
            (Self::Formed(left), Self::Formed(right)) => std::rc::Rc::ptr_eq(left, right),
            // One element's literal: the same binding and the same
            // predicate over it. Two literals of one slot are distinct.
            (Self::Element(left, left_binding, _), Self::Element(right, right_binding, _)) => {
                left_binding == right_binding && left.expression == right.expression
            }
            (Self::Family(left), Self::Family(right)) => std::rc::Rc::ptr_eq(left, right),
            (Self::Presence(left), Self::Presence(right)) => left == right,
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
    // `present(self.v)`, `self.v != absent` and `self.v = absent`: a data
    // predicate over the place's presence bit.
    let presence = located
        .presence(id)
        .map(|clock| (clock, true))
        .or_else(|| located.absent_test(id));
    if let Some((clock, present)) = presence {
        let literal = |positive: bool| {
            vec![(
                vec![(DataLiteral::Presence(clock), positive == present)],
                Vec::new(),
            )]
        };
        return Ok(Some((literal(true), literal(false))));
    }
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
                DataLiteral::Formed(form) | DataLiteral::Element(form, ..) => (**form).clone(),
                // A family and a presence bit are read at the state, not by
                // the data predicate.
                DataLiteral::Family(_) | DataLiteral::Presence(_) => return None,
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

/// The guard part `literals` and `clocks` state: its data predicate, the
/// elements it reads, and each quantifier family it conjoins, read with the
/// template parts of the polarity it takes.
fn draft_part(
    located: &Located<'_>,
    literals: Vec<(DataLiteral, bool)>,
    clocks: Vec<ClockConstraint>,
) -> Result<PartDraft, CheckCause> {
    let (presences, literals): (Vec<_>, Vec<_>) = literals
        .into_iter()
        .partition(|(literal, _)| matches!(literal, DataLiteral::Presence(_)));
    let mut presences: Vec<(ClockId, bool)> = presences
        .into_iter()
        .filter_map(|(literal, positive)| match literal {
            DataLiteral::Presence(clock) => Some((clock, positive)),
            _ => None,
        })
        .collect();
    // A part requiring a place both present and absent is never enabled:
    // no state meets both requirements.
    presences.sort();
    presences.dedup();
    // FR-230-AC-39: a constraint on an `Option<Clock>` reads its clock only
    // where the part establishes its presence.
    if let Some(unproved) = clocks.iter().find(|constraint| {
        located.bit(constraint.clock) && !presences.contains(&(constraint.clock, true))
    }) {
        return Err(timed(TemporalCause::UnprovedPresence {
            span: unproved.span,
        }));
    }
    let (families, literals): (Vec<_>, Vec<_>) = literals
        .into_iter()
        .partition(|(literal, _)| matches!(literal, DataLiteral::Family(_)));
    let data = if literals.is_empty() {
        None
    } else {
        Some(
            part_predicate(located, &literals)
                .ok_or_else(|| timed(located.outside(located.root())))?,
        )
    };
    let mut elements: Vec<(ElementBinding, Span)> = Vec::new();
    for (literal, _) in &literals {
        if let DataLiteral::Element(_, binding, span) = literal {
            if !elements.iter().any(|(held, _)| held == binding) {
                elements.push((binding.clone(), *span));
            }
        }
    }
    elements.sort_by_key(|(binding, _)| binding.slot);
    let families = families
        .into_iter()
        .filter_map(|(literal, positive)| match literal {
            DataLiteral::Family(family) => Some((family, positive)),
            _ => None,
        })
        .map(|(family, positive)| {
            let template = if positive {
                &family.positive
            } else {
                &family.negative
            };
            let parts = template
                .iter()
                .map(|(literals, clocks)| draft_part(located, literals.clone(), clocks.clone()))
                .collect::<Result<Vec<_>, CheckCause>>()?;
            Ok((family.path.clone(), family.forall == positive, parts))
        })
        .collect::<Result<Vec<_>, CheckCause>>()?;
    Ok(PartDraft {
        data,
        presences,
        elements,
        clocks,
        families,
    })
}

/// What one state clause contributes to its operation's timing.
enum ClauseClocks {
    /// The clause reads no clock.
    None,
    /// A precondition's guard parts.
    Guard(Vec<PartDraft>),
    /// A postcondition's resets.
    Resets(ResetCut),
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
    writes: Vec<CollectionWrite>,
    /// The `Option<Clock>` places the postcondition drops.
    drops: Vec<ClockId>,
    /// The guarded places written so far, by a reset or a drop: an
    /// `Option<Clock>`, or each clock case of a union field written whole.
    option_writes: BTreeSet<ClockId>,
    /// The post-state presences the postcondition requires.
    requires: Vec<(ClockId, bool)>,
}

/// The post-state presence the top-level postcondition conjunct `id`
/// requires of a guarded place of the receiver: `present(p)`, `p !=
/// absent` and `not (p = absent)` present, `not present(p)` and `not (p !=
/// absent)` absent. `p = absent` is a drop, not a requirement.
fn post_presence(located: &Located<'_>, id: ExprId) -> Option<(ClockId, bool)> {
    if let Some(clock) = located.presence(id) {
        return Some((clock, true));
    }
    if let Some((clock, true)) = located.absent_test(id) {
        return Some((clock, true));
    }
    match located.node(id)? {
        // `not present(p)`, and `not (p = absent)` / `not (p != absent)`,
        // the negation of the test.
        ExprNode::Not(operand) => located
            .presence(*operand)
            .map(|clock| (clock, false))
            .or_else(|| {
                located
                    .absent_test(*operand)
                    .map(|(clock, present)| (clock, !present))
            }),
        _ => None,
    }
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
        // `present(self.v)`, `self.v != absent` or `not present(self.v)`: a
        // requirement on the post-state's presence, read as a data
        // predicate over it (FR-230 "Presence bit").
        if let Some(required) = post_presence(located, conjunct) {
            cut.leaves.insert(conjunct, ExprNode::Boolean(true));
            cut.requires.push(required);
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
            return Err(nested_write(located, conjunct, clocks, clause.context));
        };
        if let Some((field, element)) =
            collection_write::collection_field(located, *left, clocks, clause.context)
        {
            let key = clocks
                .fields
                .iter()
                .find(|clock| {
                    clock.path.first() == element.first()
                        && clocks.model.conforms(clause.context, clock.owner)
                })
                .map(|clock| &clock.field);
            let writing = collection_write::Writing {
                record: RecordWrite {
                    located,
                    context: clause.context,
                    clocks,
                    written: &written,
                    span,
                },
                element,
            };
            let write = writing.write(field, key, *right, &mut cut)?;
            if cut
                .writes
                .iter()
                .any(|earlier| earlier.field() == write.field())
            {
                return Err(TemporalCause::ConflictingClockWrite { span });
            }
            cut.writes.push(write);
            continue;
        }
        if let Some(write) = union_write(located, *left, *right, clocks, clause.context) {
            let write = write?;
            if !written.contains(write.field) {
                return Err(TemporalCause::ClockWrite { span });
            }
            // A second write of the union field, another case or the same
            // one, gives its clock cases no one presence and value.
            let places: Vec<ClockId> = write
                .reset
                .iter()
                .map(|(clock, _)| *clock)
                .chain(write.drops.iter().copied())
                .collect();
            if places.iter().any(|clock| cut.option_writes.contains(clock)) {
                return Err(TemporalCause::ConflictingClockWrite { span });
            }
            cut.option_writes.extend(places);
            if let Some(name) = write.nullary {
                cut.leaves.insert(*right, ExprNode::Name(name));
            }
            if let Some((clock, value)) = write.reset {
                cut.resets.push(ResetDraft::single(clock, value, span));
            }
            cut.drops.extend(write.drops);
            continue;
        }
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
        // FR-230 "Presence bit": `self.v = c` of an `Option<Clock>` makes
        // the place present and resets its clock to `c`.
        if let Some(clock) = located.option(*left) {
            let in_frame = clocks
                .fields
                .get(clock.0)
                .is_some_and(|field| written.contains(&field.field));
            // A second write of the place, a reset and a drop or two
            // resets, gives it no one presence and value.
            if !cut.option_writes.insert(clock) {
                return Err(TemporalCause::ConflictingClockWrite { span });
            }
            // `self.v = absent` drops the place: absent, its clock
            // unconstrained, and no reset.
            if matches!(located.node(*right), Some(ExprNode::Absent)) {
                if !in_frame {
                    return Err(TemporalCause::ClockWrite { span });
                }
                cut.leaves.insert(conjunct, ExprNode::Boolean(true));
                cut.drops.push(clock);
                continue;
            }
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
            continue;
        }
        if let Some(parameter) = located.parameter(*left) {
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
            // FR-230 "Operation parameters": a reset of the parameter
            // writes the places of the clock fields the frame names.
            if parameter.range.is_empty() {
                return Err(TemporalCause::ClockWrite { span });
            }
            cut.leaves.insert(conjunct, ExprNode::Boolean(true));
            for clock in &parameter.range {
                let mut reset = ResetDraft::single(*clock, value.clone(), span);
                reset.parameter = Some(parameter.position);
                cut.resets.push(reset);
            }
            continue;
        }
        let Some(clock) = located.clock(*left) else {
            if !located.reads_clock(*left) && located.reads_clock(*right) {
                return Err(located.outside(*right));
            }
            return Err(nested_write(located, conjunct, clocks, clause.context));
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

/// A postcondition write `self.f = M::T::C(c)` or `self.f = M::T::D` of a
/// variant field whose cases hold clocks (FR-230 "Records, tuples and
/// unions"): the field it writes, the reset of the written case's clock to
/// `c`, which makes it present, the drops of every other case's clock, and
/// the nullary member the data write keeps in place of `M::T::C(c)`, whose
/// clock is no data payload.
struct UnionWrite<'c> {
    field: &'c DeclarationKey,
    reset: Option<(ClockId, Rational)>,
    drops: Vec<ClockId>,
    nullary: Option<qsl_forms::NameForm>,
}

/// The [`UnionWrite`] the conjunct `left = right` is, `None` when `left` is
/// no variant field holding a case's clock.
fn union_write<'c>(
    located: &Located<'_>,
    left: ExprId,
    right: ExprId,
    clocks: &'c Clocks<'_>,
    context: EffectiveId,
) -> Option<Result<UnionWrite<'c>, TemporalCause>> {
    let path = self_path(located.form, left)?;
    let cases: Vec<(ClockId, &ClockField)> = clocks
        .fields
        .iter()
        .enumerate()
        .filter(|(_, clock)| {
            clock.path.len() == path.len() + 1
                && same_place(&clock.path[..path.len()], &path)
                && matches!(clock.path.last(), Some(PlaceStep::Union(_)))
                && clocks.model.conforms(context, clock.owner)
        })
        .map(|(index, clock)| (ClockId(index), clock))
        .collect();
    let (_, first) = cases.first()?;
    let span = located.span(right);
    let (name, arguments) = match located.node(right) {
        Some(ExprNode::Call { name, arguments }) => (name, arguments.as_slice()),
        Some(ExprNode::Name(name)) => (name, &[][..]),
        _ => return Some(Err(located.outside(right))),
    };
    let case = name.segments.last()?;
    let mut write = UnionWrite {
        field: &first.field,
        reset: None,
        drops: Vec::new(),
        nullary: None,
    };
    for (clock, field) in &cases {
        let Some(PlaceStep::Union(held)) = field.path.last() else {
            continue;
        };
        if held != case {
            write.drops.push(*clock);
            continue;
        }
        let [argument] = arguments else {
            return Some(Err(TemporalCause::ClockWrite { span }));
        };
        if located.reads_clock(*argument) {
            return Some(Err(TemporalCause::ClockWrite { span }));
        }
        let value = match located.constant(*argument, clocks) {
            Ok(value) => value,
            Err(TemporalCause::ClockBoundNotConstant { .. }) => {
                return Some(Err(TemporalCause::ClockWrite { span }))
            }
            Err(cause) => return Some(Err(cause)),
        };
        write.reset = Some((*clock, value));
        write.nullary = Some(name.clone());
    }
    Some(Ok(write))
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
        let mut components = Vec::new();
        self.components(path, literal, cut, &mut components)?;
        cut.resets.extend(
            components
                .into_iter()
                .map(|(clock, value, span)| ResetDraft::single(clock, value, span)),
        );
        Ok(())
    }

    /// The clock components of the record literal `literal` written at
    /// `self.<path>`, each clock with its constant and the constant's span,
    /// as [`Self::resets`] reads them; the constants are cut from the
    /// literal, which stays a data write.
    fn components(
        &self,
        path: Vec<PlaceStep>,
        literal: ExprId,
        cut: &mut ResetCut,
        out: &mut Vec<(ClockId, Rational, Span)>,
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
                PlaceStep::Slot(_) | PlaceStep::Union(_) => false,
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
                out.push((ClockId(index), constant, value_span));
            } else if bears {
                self.components(below, *value, cut, out)?;
            }
        }
        Ok(())
    }
}

/// A postcondition's clock read under `conjunct` that is no top-level reset:
/// `clock-write` at an equation `self.x = e` that sets a clock, such as one nested under
/// `implies`, and otherwise `clock-read-outside-constraint` at the read.
fn nested_write(
    located: &Located<'_>,
    conjunct: ExprId,
    clocks: &Clocks<'_>,
    context: EffectiveId,
) -> TemporalCause {
    let mut pending = vec![conjunct];
    while let Some(id) = pending.pop() {
        if let Some(ExprNode::Binary {
            operator: BinaryOperator::Equal,
            left,
            ..
        }) = located.node(id)
        {
            // A clock, or a clock collection written whole (FR-230
            // "Collection-valued writes").
            if located.clock(*left).is_some()
                || collection_write::collection_field(located, *left, clocks, context).is_some()
            {
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

/// The names a postcondition's top-level conjuncts `name = c` write: the
/// `Clock` parameters it may reset.
fn reset_names(clause: &StateClauseDeclaration) -> Vec<String> {
    let body = &clause.body;
    let mut pending = vec![body.root_id()];
    let mut names = Vec::new();
    while let Some(id) = pending.pop() {
        match body.get(id).map(|node| node.node()) {
            Some(ExprNode::Binary {
                operator: BinaryOperator::And,
                left,
                right,
            }) => pending.extend([*left, *right]),
            Some(ExprNode::Binary {
                operator: BinaryOperator::Equal,
                left,
                ..
            }) => {
                if let Some(ExprNode::Name(name)) = body.get(*left).map(|node| node.node()) {
                    if name.segments.len() == 1 {
                        names.push(name.name.clone());
                    }
                }
            }
            _ => {}
        }
    }
    names
}

/// The `Clock` parameters of `clause`'s operation (FR-230 "Operation
/// parameters"), each with its range: the clock fields the operation's
/// frame names when a postcondition of it resets the parameter (in
/// `resets`), every clock field of the model otherwise. A field the frame
/// names only by a receiver-scoped entry (`modifies self.f`, FR-103) is
/// written on the receiver alone, so its places bind on the receiver.
fn clock_parameters(
    clause: &StateClauseDeclaration,
    clocks: &Clocks<'_>,
    resets: &BTreeSet<(DeclarationKey, String)>,
) -> Vec<ClockParameter> {
    let (Some(operation), Some(key)) = (&clause.operation, &clause.operation_key) else {
        return Vec::new();
    };
    let effect = operation.declaration.effect();
    let written: BTreeSet<&DeclarationKey> = effect.written_fields().collect();
    operation
        .declaration
        .clock_parameters()
        .iter()
        .map(|(position, name)| {
            let reset = resets.contains(&(key.clone(), name.clone()));
            let range: Vec<ClockId> = clocks
                .fields
                .iter()
                .enumerate()
                .filter(|(_, field)| !reset || written.contains(&field.field))
                .map(|(index, _)| ClockId(index))
                .collect();
            let receiver = if reset {
                range
                    .iter()
                    .copied()
                    .filter(|clock| {
                        clocks
                            .fields
                            .get(clock.0)
                            .is_some_and(|field| !effect.modifies.contains(&field.field))
                    })
                    .collect()
            } else {
                Vec::new()
            };
            ClockParameter {
                position: *position,
                name: name.clone(),
                range,
                receiver,
            }
        })
        .collect()
}

/// One state clause, its clocks read out: a precondition's guard parts or a
/// postcondition's resets, and the clause body with each clock constraint
/// and reset replaced by the Boolean it contributes to the data reading.
fn read_clause(
    clause: &mut StateClauseDeclaration,
    clocks: &Clocks<'_>,
    parameters: Vec<ClockParameter>,
    budget: &mut PartBudget,
) -> Result<ClauseClocks, CheckCause> {
    let form = ExpressionForm {
        expression: clause.body.clone(),
        spans: clause.spans.body.clone(),
    };
    let Some(located) = Located::with_parameters(&form, clocks, clause.context, parameters) else {
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
        StateClauseKind::Invariant | StateClauseKind::Constraint => {
            // A clock collection written whole anywhere but in a
            // postcondition (FR-230 "Collection-valued writes").
            let written = located.form.expression.iter().find(|node| {
                matches!(node.node(), ExprNode::Binary { operator: BinaryOperator::Equal, left, .. }
                    if collection_write::collection_field(&located, *left, clocks, clause.context).is_some())
            });
            if let Some(node) = written {
                return Err(timed(TemporalCause::ClockWrite {
                    span: located.span(node.id()),
                }));
            }
            return Err(timed(located.outside(root)));
        }
        StateClauseKind::Precondition => {
            if let Some(span) = stray_literal(&located, clocks, &BTreeSet::new()) {
                return Err(timed(TemporalCause::ClockWrite { span }));
            }
            let (dnf, polarity) = guard(&located, clocks, clause.context, budget)?;
            let parts = dnf
                .into_iter()
                .map(|(literals, clocks)| draft_part(&located, literals, clocks))
                .collect::<Result<Vec<_>, CheckCause>>()?;
            let leaves = polarity
                .into_iter()
                .map(|(id, positive)| (id, ExprNode::Boolean(positive)))
                .collect();
            (leaves, ClauseClocks::Guard(parts))
        }
        StateClauseKind::Postcondition => {
            let mut cut = resets(&located, clause, clocks).map_err(timed)?;
            if let Some(span) = stray_literal(&located, clocks, &cut.literals) {
                return Err(timed(TemporalCause::ClockWrite { span }));
            }
            // A collection written whole and a clock of it reset in one
            // postcondition give that clock no one value.
            if let Some(reset) = cut.resets.iter().find(|reset| {
                let Some(first) = clocks
                    .fields
                    .get(reset.clock.0)
                    .and_then(|clock| clock.path.first())
                else {
                    return false;
                };
                cut.writes
                    .iter()
                    .any(|write| matches!(first, PlaceStep::Field(field) if field == write.field()))
            }) {
                return Err(timed(TemporalCause::ConflictingClockWrite {
                    span: reset.span,
                }));
            }
            removed = std::mem::take(&mut cut.removed);
            (std::mem::take(&mut cut.leaves), ClauseClocks::Resets(cut))
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
    budget: &mut PartBudget,
) -> Result<InvariantDraft, CheckCause> {
    let alternatives = state_alternatives(&form.guard, clocks, context, population, budget)?;
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
                    parameter: None,
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
            .collect::<Result<Vec<_>, TemporalCause>>()
            .map_err(timed)?;
    // FR-230 "Guarded places": `when P { value(p) <= c }` bounds `p` where
    // `P` establishes `present(p)`.
    if let Some(unproved) = bounds.iter().find(|bound| {
        clocks
            .fields
            .get(bound.clock.0)
            .is_some_and(|field| field.guarded)
            && alternatives
                .iter()
                .any(|alternative| !alternative.presences.contains(&(bound.clock, true)))
    }) {
        return Err(timed(TemporalCause::UnprovedPresence {
            span: unproved.span,
        }));
    }
    Ok(InvariantDraft {
        name: form.name.name.clone(),
        alternatives,
        bounds,
    })
}

/// The alternatives of the state predicate `predicate` about `context`
/// (FR-230 "Presence bit"): the predicate itself, with no presences, when it
/// reads no presence of an `Option<Clock>` place; otherwise its disjunctive
/// form over those presences, `present(self.v)`, `self.v = absent` and
/// `self.v != absent` under `and`, `or` and `not`, each disjunct its
/// presences and its data literals, a disjunct requiring a place both
/// present and absent left out.
///
/// # Errors
///
/// A clock read anywhere in the predicate, or a presence read this slice
/// does not read (another object's place, a quantifier body).
fn state_alternatives(
    predicate: &ExpressionForm,
    clocks: &Clocks<'_>,
    context: EffectiveId,
    population: Option<PopulationDomain>,
    budget: &mut PartBudget,
) -> Result<Vec<AlternativeDraft>, CheckCause> {
    let located = Located::of(predicate, clocks, context);
    let Some(located) = located.filter(|located| located.presence_first.contains(&true)) else {
        return Ok(vec![AlternativeDraft {
            presences: Vec::new(),
            predicate: state_predicate(predicate, clocks, context, population).map_err(timed)?,
        }]);
    };
    let (dnf, _) = guard(&located, clocks, context, budget)?;
    let mut alternatives = Vec::new();
    for (literals, constraints) in dnf {
        if let Some(constraint) = constraints.first() {
            return Err(timed(TemporalCause::ClockReadOutsideConstraint {
                span: constraint.span,
            }));
        }
        let (presences, literals): (Vec<_>, Vec<_>) = literals
            .into_iter()
            .partition(|(literal, _)| matches!(literal, DataLiteral::Presence(_)));
        let mut presences: Vec<(ClockId, bool)> = presences
            .into_iter()
            .filter_map(|(literal, positive)| match literal {
                DataLiteral::Presence(clock) => Some((clock, positive)),
                _ => None,
            })
            .collect();
        presences.sort();
        presences.dedup();
        if presences.windows(2).any(|pair| pair[0].0 == pair[1].0) {
            continue;
        }
        let data = if literals.is_empty() {
            let span = located.span(located.root());
            let expression = qsl_forms::Expression::boolean(true);
            let spans = qsl_forms::ExpressionSpans::at_point(&expression, span.start)
                .map_err(|_| timed(located.outside(located.root())))?;
            ExpressionForm { expression, spans }
        } else {
            part_predicate(&located, &literals)
                .ok_or_else(|| timed(located.outside(located.root())))?
        };
        alternatives.push(AlternativeDraft {
            presences,
            predicate: PredicateDraft {
                context,
                population,
                predicate: data,
            },
        });
    }
    Ok(alternatives)
}

/// The time invariants the package's `max` constraints on clocks state
/// (FR-230 "Clock constraints on a type"): for each constraint and each
/// object type holding a clock it bounds, `true` with the bound `x <= c` on
/// each such clock, read at every live place of it (each held slot of a
/// collection, a present optional member). A constraint on a clock alias
/// bounds the clocks typed by the alias; one on `ix://quire/native/Clock`
/// every clock of the model.
///
/// # Errors
///
/// `time-invariant-shape` for a `min` constraint, which no conjunction of
/// upper bounds states.
fn clock_bounds(
    model: &AdmittedModel,
    clocks: &Clocks<'_>,
    inputs: &Inputs<'_>,
    span: Span,
) -> Result<Vec<InvariantDraft>, CheckCause> {
    let mut drafts = Vec::new();
    for bound in model.clock_bounds() {
        if !bound.upper {
            return Err(timed(TemporalCause::TimeInvariantShape { span }));
        }
        let mut by_owner: BTreeMap<EffectiveId, Vec<ClockConstraint>> = BTreeMap::new();
        for (index, field) in clocks.fields.iter().enumerate() {
            if bound
                .alias
                .as_ref()
                .is_some_and(|alias| !field.aliases.contains(alias))
            {
                continue;
            }
            let slots = field
                .path
                .iter()
                .filter(|step| matches!(step, PlaceStep::Slot(_)))
                .map(|_| SlotRef::Family)
                .collect();
            by_owner
                .entry(field.owner)
                .or_default()
                .push(ClockConstraint {
                    parameter: None,
                    clock: ClockId(index),
                    via: Vec::new(),
                    slots,
                    op: ClockOp::LessOrEqual,
                    bound: bound.value.clone(),
                    span,
                });
        }
        for (owner, bounds) in by_owner {
            let population = population_of(inputs.models, owner, inputs.types).map_err(|_| {
                timed(TemporalCause::TimedStateObject {
                    ambiguous: true,
                    span,
                })
            })?;
            let expression = qsl_forms::Expression::boolean(true);
            let spans = qsl_forms::ExpressionSpans::at_point(&expression, span.start)
                .map_err(|_| timed(TemporalCause::TimeInvariantShape { span }))?;
            drafts.push(InvariantDraft {
                name: bound.key.node.clone(),
                // A bound reads a guarded place only where it is live.
                alternatives: vec![AlternativeDraft {
                    presences: Vec::new(),
                    predicate: PredicateDraft {
                        context: owner,
                        population,
                        predicate: ExpressionForm { expression, spans },
                    },
                }],
                bounds,
            });
        }
    }
    Ok(drafts)
}

fn urgency(
    form: &UrgentForm,
    model: &AdmittedModel,
    clocks: &Clocks<'_>,
    state: impl FnOnce() -> Result<(EffectiveId, Option<PopulationDomain>), TemporalCause>,
    guarded: &BTreeMap<DeclarationKey, String>,
    budget: &mut PartBudget,
) -> Result<UrgencyDraft, CheckCause> {
    match &form.target {
        UrgentTarget::When(predicate) => {
            let (context, population) = state().map_err(timed)?;
            Ok(UrgencyDraft::When {
                name: form.name.name.clone(),
                alternatives: state_alternatives(predicate, clocks, context, population, budget)?,
            })
        }
        UrgentTarget::Operation(operation) => {
            let keys = model.tick_operation_keys(operation);
            let [key] = keys.as_slice() else {
                return Err(timed(TemporalCause::UrgentOperation {
                    name: operation.name.clone(),
                    ambiguous: !keys.is_empty(),
                    span: operation.span,
                }));
            };
            if let Some(clock) = guarded.get(*key) {
                return Err(timed(TemporalCause::UrgentGuardReadsClock {
                    clock: clock.clone(),
                    span: form.span,
                }));
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
        let fields = clock_fields(model);
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
        // A clock, or a bound on clocks (FR-230-AC-25), needs dense time.
        let field = match (clocks.fields.first(), model.clock_bounds().next()) {
            (Some(field), _) => field.name.clone(),
            (None, Some(bound)) => bound.key.node.clone(),
            (None, None) => continue,
        };
        let span = match source_of(model) {
            Some(source) if matches!(source.source, TimeSourceKind::Dense) => {
                // A place under an optional member is guarded by a presence
                // FR-230 does not yet give S3 a reading of (STD-493 item
                // 17).
                if AdmittedModel::clock_places(model).iter().any(|place| {
                    place.optional && !((place.bit && place.path.len() == 1) || place.case)
                }) {
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
            return Err(timed(TemporalCause::ClockWithoutDenseTime { field, span }));
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
        // The `Clock` parameters some postcondition of their operation
        // resets, by the operation's key and the parameter's name.
        let operation_resets: BTreeSet<(DeclarationKey, String)> = state_clauses
            .iter()
            .filter(|clause| clause.kind == StateClauseKind::Postcondition)
            .filter_map(|clause| Some((clause.operation_key.clone()?, clause)))
            .flat_map(|(key, clause)| {
                reset_names(clause)
                    .into_iter()
                    .map(move |name| (key.clone(), name))
            })
            .collect();
        let mut guards = Vec::new();
        let mut resets = Vec::new();
        let mut parameter_ranges = Vec::new();
        // The clock each guarded operation's guard reads first, for
        // `urgent O`.
        let mut guarded = BTreeMap::new();
        for (index, clause) in state_clauses.iter_mut().enumerate() {
            if model.type_key(clause.context).is_none() {
                continue;
            }
            let parameters = clock_parameters(clause, &clocks, &operation_resets);
            if let Some(operation) = &clause.operation {
                for parameter in &parameters {
                    let listed = parameter_ranges.iter().any(|listed: &ClockParameterRange| {
                        listed.position == parameter.position
                            && listed.operation.declaring == operation.declaring
                            && listed.operation.declaration.name() == operation.declaration.name()
                    });
                    if !listed {
                        parameter_ranges.push(ClockParameterRange {
                            operation: operation.clone(),
                            position: parameter.position,
                            range: parameter.range.clone(),
                            receiver: parameter.receiver.clone(),
                        });
                    }
                }
            }
            match read_clause(clause, &clocks, parameters, &mut budget)? {
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
                ClauseClocks::Resets(cut) => {
                    if let (false, Some(operation)) = (
                        cut.resets.is_empty()
                            && cut.writes.is_empty()
                            && cut.drops.is_empty()
                            && cut.requires.is_empty(),
                        &clause.operation,
                    ) {
                        resets.push(ResetsDraft {
                            clause: clause.name.clone(),
                            operation: operation.clone(),
                            context: clause.context,
                            resets: cut.resets,
                            writes: cut.writes,
                            drops: cut.drops,
                            requires: cut.requires,
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
                    invariants.push(time_invariant(
                        form,
                        &clocks,
                        context,
                        population,
                        &mut budget,
                    )?);
                }
                TimedDeclaration::Urgent(form) => {
                    urgency_drafts.push(urgency(
                        form,
                        model,
                        &clocks,
                        state,
                        &guarded,
                        &mut budget,
                    )?);
                }
                TimedDeclaration::TimeSource(_) | TimedDeclaration::ClockType(_) => {}
            }
        }
        invariants.extend(clock_bounds(model, &clocks, inputs, source.span)?);
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
            clock_parameters: parameter_ranges,
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
    path: &[String],
    span: Span,
) -> Result<ValueType, CheckRefusal> {
    let not_checked = || {
        refusal(CheckCause::TimedNotChecked {
            construct: TimedConstruct::Clock,
            span,
        })
    };
    let types = checking.declarations.scope.types();
    let (first, below) = path.split_first().ok_or_else(not_checked)?;
    let mut value_type = types
        .attribute(context, first)
        .map(|attribute| attribute.field().value_type().clone())
        .ok_or_else(not_checked)?;
    // Each record field on the way to the collection.
    for name in below {
        let ValueType::Composite(key) = &value_type else {
            return Err(not_checked());
        };
        let Some(CompositeShape::Record(fields)) =
            types.composite(*key).map(|declaration| declaration.shape())
        else {
            return Err(not_checked());
        };
        value_type = fields
            .iter()
            .find(|field| field.name() == name)
            .map(|field| field.value_type().clone())
            .ok_or_else(not_checked)?;
    }
    match &value_type {
        ValueType::Collection(collection) => Ok(collection.element().clone()),
        _ => Err(not_checked()),
    }
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

/// `alternative` with its data predicate typed as a state predicate.
fn typed_alternative(
    alternative: AlternativeDraft,
    checking: &Checking<'_, '_>,
    cx: &mut CheckContext<'_, ClauseDeclarations<'_>>,
    nodes_used: &mut u64,
) -> Result<StateAlternative, CheckRefusal> {
    let (predicate, used) = typed(
        &state_view(&alternative.predicate, checking.complete_value),
        &alternative.predicate.predicate,
        checking,
        cx,
    )?;
    *nodes_used = (*nodes_used).max(used);
    Ok(StateAlternative {
        presences: alternative.presences,
        predicate,
    })
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
/// The guard part `part` of the precondition `clause` with its data
/// predicate typed, each element it reads typed as an element of the
/// receiver's collection field, and its families' template parts typed
/// alike.
fn check_part(
    part: PartDraft,
    clause: &StateClauseDeclaration,
    operation: &ClauseOperation,
    checking: &Checking<'_, '_>,
    cx: &mut CheckContext<'_, ClauseDeclarations<'_>>,
    nodes_used: &mut u64,
) -> Result<ConvexPart, CheckRefusal> {
    let binders = part
        .elements
        .iter()
        .map(|(binding, span)| {
            let element = element_type(checking, clause.context, &binding.path, *span)?;
            Ok((binding.name.clone(), element))
        })
        .collect::<Result<Vec<_>, CheckRefusal>>()?;
    let data = match &part.data {
        None => None,
        Some(form) => {
            let view = ClauseView {
                kind: StateClauseKind::Precondition,
                complete_value: super::state_clause::is_complete_value(&clause.selection),
                context: Some(clause.context),
                receiver: Some(("self", ValueType::Reference(clause.context))),
                operation: Some(operation),
                context_population: clause.context_population,
                frame_population: clause.frame_population,
                binders: &binders,
                body: &form.expression,
            };
            let (predicate, used) = typed(&view, form, checking, cx)?;
            *nodes_used = (*nodes_used).max(used);
            Some(predicate)
        }
    };
    let families = part
        .families
        .into_iter()
        .map(|(path, each, parts)| {
            let parts = parts
                .into_iter()
                .map(|part| check_part(part, clause, operation, checking, cx, nodes_used))
                .collect::<Result<Vec<_>, CheckRefusal>>()?;
            Ok(QuantifiedFamily { path, each, parts })
        })
        .collect::<Result<Vec<_>, CheckRefusal>>()?;
    Ok(ConvexPart {
        data,
        presences: part.presences,
        elements: part
            .elements
            .into_iter()
            .map(|(binding, _)| binding)
            .collect(),
        clocks: part.clocks,
        families,
    })
}

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
                checked.push(check_part(
                    part,
                    clause,
                    operation,
                    checking,
                    cx,
                    &mut nodes_used,
                )?);
            }
            guards.push(ClockGuard {
                clause: clause.name.clone(),
                operation: operation.clone(),
                parts: checked,
            });
        }
        let mut invariants = Vec::with_capacity(draft.invariants.len());
        for invariant in draft.invariants {
            let context = invariant
                .alternatives
                .first()
                .map(|alternative| alternative.predicate.context);
            let alternatives = invariant
                .alternatives
                .into_iter()
                .map(|alternative| typed_alternative(alternative, checking, cx, &mut nodes_used))
                .collect::<Result<Vec<_>, CheckRefusal>>()?;
            let Some(context) = context else {
                continue;
            };
            invariants.push(TimeInvariant {
                name: invariant.name,
                context,
                alternatives,
                bounds: invariant.bounds,
            });
        }
        let mut urgency = Vec::with_capacity(draft.urgency.len());
        for declared in draft.urgency {
            urgency.push(match declared {
                UrgencyDraft::When { name, alternatives } => {
                    let Some(context) = alternatives
                        .first()
                        .map(|alternative| alternative.predicate.context)
                    else {
                        continue;
                    };
                    let alternatives = alternatives
                        .into_iter()
                        .map(|alternative| {
                            typed_alternative(alternative, checking, cx, &mut nodes_used)
                        })
                        .collect::<Result<Vec<_>, CheckRefusal>>()?;
                    Urgency::When {
                        name,
                        context,
                        alternatives,
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
                    Some((path, name, predicate)) => {
                        let predicate = match predicate {
                            None => None,
                            Some(form) => {
                                let element =
                                    element_type(checking, written.context, &path, reset.span)?;
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
                            path,
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
                    parameter: reset.parameter,
                });
            }
            resets.push(ClockResets {
                clause: written.clause,
                operation: written.operation,
                resets: checked,
                writes: written.writes,
                drops: written.drops,
                requires: written.requires,
            });
        }
        models.push(TimedModel {
            model: draft.model,
            span: draft.span,
            clocks: draft.clocks,
            clock_parameters: draft.clock_parameters,
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
