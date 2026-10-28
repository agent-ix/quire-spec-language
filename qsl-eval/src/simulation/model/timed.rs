// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-231 (ADR-026 TS-1 to TS-7): the timed subject, a model subject whose
//! model has a `time` member, read as timed states and moves.
//!
//! A timed state is a model state together with a clock valuation: one
//! exact non-negative rational per clock place, a clock field of one object
//! of the subject's universe, every one 0 in an initial state. A model
//! snapshot carries no clock value.
//!
//! Two moves leave a timed state `(s, v)`:
//!
//! - a **delay** by `d >= 0` to `(s, v + d)`, admissible when `d = 0`, or
//!   when `s` is not urgent and every time invariant that applies at `s`
//!   holds at `v + d`;
//! - a **discrete step** by transition identity `t` to `(s', v')`, when `t`
//!   is enabled at `s` by its data precondition (FR-120), a convex part of
//!   its guard holds at `v`, `v'` is `v` with `t`'s resets applied, and every
//!   time invariant that applies at `s'` holds at `v'`.
//!
//! The guards, resets, invariants and urgency are S3's checked
//! [`TimedModel`] (FR-230). [`TimedSystem::guarded_steps`] and
//! [`TimedSystem::invariants`] give them over clock places, the symbolic
//! form the zone engine (FR-239) reads; [`TimedSystem::delay`] and
//! [`TimedSystem::steps`] are the concrete moves replay (FR-237) reads.
//! Every comparison and sum is exact.
//!
//! A predicate of a guard part, a time invariant or an `urgent when` is read
//! through the one clause evaluator with its `self` bound to the object it
//! is about. An object the state does not hold has no clocks to constrain:
//! a predicate about it does not apply.

use quire_exact::{EffectiveId, Integer, ObjectReference, Rational, ValueType};
use quire_semantic_value::catalog::UndefinedReason;
use quire_semantic_value::model_transition::ModelTransition;

use qsl_semantics::check::{
    CheckedTimedPredicate, ClockConstraint, ClockId, ClockOp, ElementBinding, ModelTimeSource,
    PlaceStep, QuantifiedFamily, SlotRef, TimeSourceKind, TimedModel, Urgency,
};

use qsl_semantics::model::object_environment::PlacePresences;
use quire_checked::key::DeclarationKey;

use super::encode::declaration_text;
use super::state_predicate::PredicateReading;
use super::walk::{expand_enabled, stop};
use super::BROKEN;
use super::{ModelSimState, ModelSystem};
use crate::simulation::expansion::ExpansionStop;
use crate::simulation::reduction::Permutation;
use crate::temporal::UndefinedCause;
use crate::value::Provenance;

mod over_time;
pub use over_time::{DelayInterval, TimeLock, TimedTerminalClass};

/// One clock of the subject: a clock field of one object of its universe.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockPlace {
    object: ObjectReference,
    clock: ClockId,
    /// The slot index of each collection on the clock's path, outermost
    /// first (FR-230 "Clock collections"); empty outside collections.
    slots: Vec<usize>,
    /// The population and key the object is held under.
    instance: crate::model_subject::ModelInstance,
}

impl ClockPlace {
    /// Whether `state` holds the place's object: a clock of an absent object
    /// has no value (QSpec FR-181 "Timed subjects"). A slot place is live
    /// only while its collection also holds the slot:
    /// [`TimedSystem::live`].
    pub fn present(&self, state: &ModelSimState) -> bool {
        self.stored(state).is_some()
    }

    /// The stored object of the place at `state`.
    fn stored<'s>(
        &self,
        state: &'s ModelSimState,
    ) -> Option<&'s qsl_semantics::model::state::StateObject> {
        state
            .state()
            .populations
            .get(&self.instance.population)?
            .get(&self.instance.key)
    }

    /// The slot of each collection on the clock's path, outermost first.
    pub fn slots(&self) -> &[usize] {
        &self.slots
    }

    /// The object whose clock it is.
    pub fn object(&self) -> &ObjectReference {
        &self.object
    }

    /// Its clock field in the timed model.
    pub fn clock(&self) -> ClockId {
        self.clock
    }

    fn holds(&self, object: &ObjectReference) -> bool {
        self.object.universe() == object.universe() && self.object.object() == object.object()
    }
}

/// The value of every clock place, in [`TimedSystem::places`] order.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ClockValuation(Vec<Rational>);

impl ClockValuation {
    /// The valuation holding `values`, in [`TimedSystem::places`] order.
    pub fn new(values: Vec<Rational>) -> Self {
        Self(values)
    }

    /// The value of the place at `index`.
    pub fn get(&self, index: usize) -> Option<&Rational> {
        self.0.get(index)
    }

    /// Every value, in place order.
    pub fn values(&self) -> &[Rational] {
        &self.0
    }

    /// Every clock advanced by `delay`.
    #[must_use]
    pub fn delayed(&self, delay: &Rational) -> Self {
        Self(self.0.iter().map(|value| value.add(delay)).collect())
    }
}

/// A timed state (FR-231 `TimedState`): a model state and a clock
/// valuation.
#[derive(Clone, Debug)]
pub struct TimedState {
    discrete: ModelSimState,
    clocks: ClockValuation,
}

impl TimedState {
    /// The timed state of `discrete` at `clocks`.
    pub fn new(discrete: ModelSimState, clocks: ClockValuation) -> Self {
        Self { discrete, clocks }
    }

    /// The model state.
    pub fn discrete(&self) -> &ModelSimState {
        &self.discrete
    }

    /// The clock valuation.
    pub fn clocks(&self) -> &ClockValuation {
        &self.clocks
    }
}

/// An atomic clock constraint over one clock place.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PlaceConstraint {
    place: usize,
    op: ClockOp,
    bound: Rational,
}

impl PlaceConstraint {
    /// `place op bound`, over a place index the caller names: one of
    /// [`TimedSystem::places`], or a clock an engine adds past them, such as
    /// a claim automaton's horizon clock (FR-242).
    pub fn new(place: usize, op: ClockOp, bound: Rational) -> Self {
        Self { place, op, bound }
    }

    /// The constrained place's index in [`TimedSystem::places`].
    pub fn place(&self) -> usize {
        self.place
    }

    /// The comparison.
    pub fn op(&self) -> ClockOp {
        self.op
    }

    /// The bound, in the model's unit.
    pub fn bound(&self) -> &Rational {
        &self.bound
    }

    /// Whether `clocks` satisfies the constraint.
    pub fn holds(&self, clocks: &ClockValuation) -> bool {
        clocks
            .get(self.place)
            .is_some_and(|value| self.op.holds(value, &self.bound))
    }
}

/// One discrete step enabled at a model state by its data precondition,
/// with the clock part of its guard over places.
#[derive(Clone, Debug)]
pub struct GuardedStep {
    transition: ModelTransition,
    receiver: ObjectReference,
    operation: String,
    post: ModelSimState,
    parts: Vec<Vec<PlaceConstraint>>,
    resets: Vec<(usize, Rational)>,
    carries: Vec<(usize, usize)>,
    guards: Vec<String>,
}

impl GuardedStep {
    /// The transition identity.
    pub fn transition(&self) -> &ModelTransition {
        &self.transition
    }

    /// The receiver.
    pub fn receiver(&self) -> &ObjectReference {
        &self.receiver
    }

    /// The operation's declaration identity text (FR-181).
    pub fn operation(&self) -> &str {
        &self.operation
    }

    /// The post-state.
    pub fn post(&self) -> &ModelSimState {
        &self.post
    }

    /// The convex parts of the guard whose data predicate holds at the
    /// state: the step is enabled at a valuation satisfying every
    /// constraint of one part. An unguarded step has one empty part; a step
    /// whose every part's data predicate is false has none.
    pub fn parts(&self) -> &[Vec<PlaceConstraint>] {
        &self.parts
    }

    /// The names of the precondition clauses whose clock guards apply to
    /// the step, in model order: what a guard refusal names (FR-237).
    pub fn guards(&self) -> &[String] {
        &self.guards
    }

    /// The resets, each a place and its new value, in source order.
    pub fn resets(&self) -> &[(usize, Rational)] {
        &self.resets
    }

    /// The carries of a collection write (FR-230 "Collection-valued
    /// writes"), each a place of the post-state and the place of the
    /// pre-state whose value it takes: a clock moves with its element.
    pub fn carries(&self) -> &[(usize, usize)] {
        &self.carries
    }

    /// `clocks` with the carries, then the resets, applied: every carry
    /// reads `clocks` as they were before the step.
    pub fn reset(&self, clocks: &ClockValuation) -> ClockValuation {
        let mut values = clocks.0.clone();
        for (to, from) in &self.carries {
            if let (Some(value), Some(slot)) = (clocks.0.get(*from), values.get_mut(*to)) {
                slot.clone_from(value);
            }
        }
        for (place, value) in &self.resets {
            if let Some(slot) = values.get_mut(*place) {
                slot.clone_from(value);
            }
        }
        ClockValuation(values)
    }
}

/// Which timed member a reading is about.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TimedSite {
    /// A precondition's guard.
    Guard {
        /// The precondition's name.
        clause: String,
    },
    /// A time invariant.
    Invariant {
        /// The invariant's name.
        name: String,
    },
    /// An `urgent when` declaration.
    Urgency {
        /// The declaration's name.
        name: String,
    },
    /// A postcondition's reset by selection.
    Reset {
        /// The postcondition's name.
        clause: String,
    },
}

/// Why a timed move could not be decided.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TimedStop {
    /// The model state could not be expanded or observed.
    Expansion(ExpansionStop),
    /// A timed member's predicate had no truth value at the state.
    Undecided(Box<UndecidedReading>),
    /// The state's live clocks exceed `max_clocks` (FR-230 "Clock
    /// collections", QSpec FR-415 "Cardinality bound"): the analysis stops
    /// `limit-reached` naming `max_clocks`.
    MaxClocks {
        /// The bound, `model_check.max_clocks`.
        max_clocks: u64,
    },
    /// A broken invariant of the place listing: the state holds an element
    /// whose clocks have no place, though its live clocks are within
    /// `max_clocks`. Never stepped over.
    UnplacedElement {
        /// The state's live clocks.
        live: u64,
        /// Those with a place.
        placed: u64,
    },
}

/// A timed member's predicate with no truth value at a state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UndecidedReading {
    /// The member.
    pub site: TimedSite,
    /// The object it was read about.
    pub object: ObjectReference,
    /// Why it had none, when it was undefined.
    pub cause: Option<UndefinedCause>,
}

impl TimedStop {
    fn undecided(site: TimedSite, object: ObjectReference, cause: Option<UndefinedCause>) -> Self {
        Self::Undecided(Box::new(UndecidedReading {
            site,
            object,
            cause,
        }))
    }
}

impl From<ExpansionStop> for TimedStop {
    fn from(stop: ExpansionStop) -> Self {
        Self::Expansion(stop)
    }
}

/// Why a [`TimedSystem`] cannot be built over a subject.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TimedSubjectRefusal {
    /// The subject's package has no timed model for the alias.
    #[error("the subject's package has no timed model `{model}`")]
    NoTimedModel {
        /// The model alias.
        model: String,
    },
    /// A guard part's data predicate reads an operation parameter, which
    /// this reading does not bind.
    #[error("the guard of `{clause}` reads an operation parameter")]
    ParameterizedGuard {
        /// The precondition's name.
        clause: String,
    },
    /// A clock lies under two collections with no declared maximum, whose
    /// places cannot be listed within `max_clocks` for every state that
    /// keeps its live clocks within it.
    #[error("the clock `{clock}` lies under two collections with no declared maximum")]
    NestedUnboundedCollection {
        /// The clock place's name.
        clock: String,
    },
    /// A clock owner's population has no universe, so its clocks cannot be
    /// listed.
    #[error("the clocks of `{clock}` have no universe to range over")]
    Unbounded {
        /// The clock field's name.
        clock: String,
    },
    /// FR-120's bound check refuses the subject's system: a timed subject
    /// is built only to be expanded, and a system with an unbounded root is
    /// never expanded.
    #[error(transparent)]
    NotSimulated(#[from] crate::simulation::NotSimulated),
}

/// The held slots of `constraint`; `None` when one is a family's template
/// slot, which only [`TimedSystem::placed_at`] reads.
fn held_slots(constraint: &ClockConstraint) -> Option<Vec<u64>> {
    constraint
        .slots()
        .iter()
        .map(|slot| match slot {
            SlotRef::Held(slot) => Some(*slot),
            SlotRef::Family => None,
        })
        .collect()
}

/// The default bound on the live clocks of a state: QSpec FR-415's
/// `model_check.max_clocks`.
pub const DEFAULT_MAX_CLOCKS: std::num::NonZeroU64 = std::num::NonZeroU64::MIN.saturating_add(1023);

/// FR-231's timed subject over one model system.
pub struct TimedSystem<'s, 'a> {
    system: &'s ModelSystem<'a>,
    model: &'s TimedModel,
    source: &'s ModelTimeSource,
    places: Vec<ClockPlace>,
    /// The object references each predicate's `self` type ranges over.
    objects: std::collections::BTreeMap<EffectiveId, Vec<ObjectReference>>,
    /// The declaration identity text of each `urgent O` operation.
    urgent_operations: Vec<String>,
    /// The declaration identity text of a tick source's operation.
    tick: Option<(String, Rational)>,
    /// The bound on the live clocks of a state (QSpec FR-415).
    max_clocks: std::num::NonZeroU64,
    /// Each clock field with each object of its owner's universe, to count
    /// a state's live clocks by.
    holders: Vec<ClockPlace>,
}

impl<'s, 'a> TimedSystem<'s, 'a> {
    /// The timed subject of `system` under the timed model of `model`.
    ///
    /// # Errors
    ///
    /// [`TimedSubjectRefusal`] when the package has no such timed model, a
    /// guard part reads an operation parameter, or a clock owner has no
    /// universe.
    pub fn new(system: &'s ModelSystem<'a>, model: &str) -> Result<Self, TimedSubjectRefusal> {
        Self::with_max_clocks(system, model, DEFAULT_MAX_CLOCKS)
    }

    /// [`Self::new`] under the bound `max_clocks` on the live clocks of a
    /// state (QSpec FR-415 "Cardinality bound"): a state whose live clocks
    /// exceed it stops [`TimedStop::MaxClocks`] when it is expanded. A
    /// collection with no declared maximum (FR-230 "Clock collections":
    /// `max: None`) lists, for each of its slots, as many places as one
    /// element's clocks fit within `max_clocks` times over, so every state
    /// within the bound has a place for each of its live clocks.
    ///
    /// # Errors
    ///
    /// As [`Self::new`], and [`TimedSubjectRefusal::NestedUnboundedCollection`]
    /// for a clock under two collections with no declared maximum.
    pub fn with_max_clocks(
        system: &'s ModelSystem<'a>,
        model: &str,
        max_clocks: std::num::NonZeroU64,
    ) -> Result<Self, TimedSubjectRefusal> {
        super::check_model_bounds(system, crate::simulation::BOUND_POSITIONS)?;
        let graph = system.package.graph();
        let missing = || TimedSubjectRefusal::NoTimedModel {
            model: model.to_owned(),
        };
        let timed = graph.timed_model(model).ok_or_else(missing)?;
        let source = graph
            .time_sources()
            .iter()
            .find(|source| source.model == model)
            .ok_or_else(missing)?;
        for guard in timed.guards() {
            // `self` and the elements the part reads; an operation
            // parameter is read by no guard here.
            if guard.parts().iter().any(|part| {
                part.data()
                    .is_some_and(|data| data.parameters().len() > 1 + part.elements().len())
            }) {
                return Err(TimedSubjectRefusal::ParameterizedGuard {
                    clause: guard.clause().to_owned(),
                });
            }
        }
        let mut objects = std::collections::BTreeMap::new();
        let mut held = std::collections::BTreeMap::new();
        let mut instances_of = |owner: EffectiveId, name: &str| {
            if let Some(found) = held.get(&owner) {
                return Ok::<
                    Vec<(crate::model_subject::ModelInstance, ObjectReference)>,
                    TimedSubjectRefusal,
                >(Vec::clone(found));
            }
            let unbounded = || TimedSubjectRefusal::Unbounded {
                clock: name.to_owned(),
            };
            let instances = system
                .instances(&ValueType::Reference(owner))
                .map_err(|_| unbounded())?;
            let references = instances
                .iter()
                .map(|instance| system.bound_reference(owner, instance))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| unbounded())?;
            objects.insert(owner, references.clone());
            let pairs: Vec<_> = instances.into_iter().zip(references).collect();
            held.insert(owner, pairs.clone());
            Ok(pairs)
        };
        // The fewest clocks one element of each collection with no declared
        // maximum holds, by owner and the path through that collection's
        // slot: each clock below it times the declared minima of the
        // collections between. Listing `max_clocks` over it slots, every
        // state within `max_clocks` has a place for each live clock.
        let mut per_element: std::collections::BTreeMap<(EffectiveId, Vec<PlaceStep>), u64> =
            std::collections::BTreeMap::new();
        for (_, field) in timed.clock_fields() {
            let path = field.path();
            let mut open = path
                .iter()
                .enumerate()
                .filter(|(_, step)| matches!(step, PlaceStep::Slot(shape) if shape.max.is_none()));
            let Some((at, _)) = open.next() else {
                continue;
            };
            if open.next().is_some() {
                return Err(TimedSubjectRefusal::NestedUnboundedCollection {
                    clock: field.name().to_owned(),
                });
            }
            let below = path[at + 1..]
                .iter()
                .filter_map(|step| match step {
                    PlaceStep::Slot(shape) => Some(shape.min),
                    PlaceStep::Field(_) | PlaceStep::Union(_) => None,
                })
                .fold(1_u64, u64::saturating_mul);
            let held = per_element
                .entry((field.owner(), path[..=at].to_vec()))
                .or_insert(0);
            *held = held.saturating_add(below);
        }
        let mut places = Vec::new();
        let mut holders = Vec::new();
        for (clock, field) in timed.clock_fields() {
            // An undeclared slot lists as many elements as fit within
            // `max_clocks`: a state holding more exceeds the bound.
            let open = field
                .path()
                .iter()
                .position(|step| matches!(step, PlaceStep::Slot(shape) if shape.max.is_none()));
            let listed = open
                .and_then(|at| per_element.get(&(field.owner(), field.path()[..=at].to_vec())))
                .map_or(0, |held| max_clocks.get() / (*held).max(1));
            let maxima: Vec<usize> = field
                .slots()
                .map(|shape| usize::try_from(shape.max.unwrap_or(listed)).unwrap_or(usize::MAX))
                .collect();
            let slots = slot_vectors(&maxima);
            for (instance, object) in instances_of(field.owner(), field.name())? {
                holders.push(ClockPlace {
                    object: object.clone(),
                    clock,
                    slots: Vec::new(),
                    instance: instance.clone(),
                });
                for slots in &slots {
                    places.push(ClockPlace {
                        object: object.clone(),
                        clock,
                        slots: slots.clone(),
                        instance: instance.clone(),
                    });
                }
            }
        }
        for invariant in timed.invariants() {
            instances_of(invariant.context(), invariant.name())?;
        }
        for urgency in timed.urgency() {
            if let Urgency::When { name, context, .. } = urgency {
                instances_of(*context, name)?;
            }
        }
        let urgent_operations = timed
            .urgency()
            .iter()
            .filter_map(|urgency| match urgency {
                Urgency::Operation { operation, .. } => {
                    declaration_text(operation, quire_canonical::Limits::new(u64::MAX)).ok()
                }
                Urgency::When { .. } => None,
            })
            .collect();
        let tick = match (&source.source, &source.tick_operation_key) {
            (TimeSourceKind::Tick { period, .. }, Some(key)) => {
                declaration_text(key, quire_canonical::Limits::new(u64::MAX))
                    .ok()
                    .map(|text| (text, period.clone()))
            }
            _ => None,
        };
        Ok(Self {
            system,
            model: timed,
            source,
            places,
            objects,
            urgent_operations,
            tick,
            max_clocks,
            holders,
        })
    }

    /// The bound on the live clocks of a state.
    pub fn max_clocks(&self) -> std::num::NonZeroU64 {
        self.max_clocks
    }

    /// The live clocks of `state` (QSpec FR-415 "Cardinality bound"): for
    /// each object the state holds and each clock field of its type, one
    /// clock outside collections, and below a collection one for each
    /// element it holds, whether or not the element has a listed place.
    pub fn live_clocks(&self, state: &ModelSimState) -> u64 {
        let typing = self.system.typing();
        let mut live = 0_u64;
        for holder in &self.holders {
            let Some(field) = self.model.clock(holder.clock) else {
                continue;
            };
            let Some(stored) = holder.stored(state) else {
                continue;
            };
            // The steps through the innermost collection: the clock below
            // it is no data component to read.
            let through = field
                .path()
                .iter()
                .rposition(|step| matches!(step, PlaceStep::Slot(_) | PlaceStep::Union(_)))
                .map_or(0, |last| last + 1);
            if through == 0 {
                // An absent `Option<Clock>` holds no clock.
                let absent = field.presence_bit()
                    && !matches!(
                        stored.fields.get(field.field()),
                        Some(quire_exact::Value::Boolean(true))
                    );
                if !absent {
                    live = live.saturating_add(1);
                }
                continue;
            }
            let Some(PlaceStep::Field(first)) = field.path().first() else {
                continue;
            };
            let Some(fields) = self.system.fields.get(&stored.type_identity) else {
                continue;
            };
            let Some(value) = fields
                .iter()
                .find(|declared| declared.name == *first)
                .and_then(|declared| stored.fields.get(&declared.key))
            else {
                continue;
            };
            // Every value the path reaches, element by element.
            let mut pending = vec![(value, &field.path()[1..through])];
            while let Some((value, rest)) = pending.pop() {
                match rest.split_first() {
                    // An element of a `Sequence<Option<Clock>>` is live
                    // while its bit is set.
                    None if field.presence_bit()
                        && !matches!(value, quire_exact::Value::Boolean(true)) => {}
                    None => live = live.saturating_add(1),
                    Some((PlaceStep::Field(name), rest)) => {
                        if let Some(next) = typing.component(value, name) {
                            pending.push((next, rest));
                        }
                    }
                    // A union case's clock is live while the value is
                    // that case.
                    Some((PlaceStep::Union(case), rest)) => {
                        if is_case(value, case) {
                            pending.push((value, rest));
                        }
                    }
                    Some((PlaceStep::Slot(_), rest)) => match value {
                        quire_exact::Value::Collection(collection) => {
                            pending.extend(
                                collection.elements().iter().map(|element| (element, rest)),
                            );
                        }
                        // A collection of bare clocks: one live clock per
                        // element its count member holds.
                        quire_exact::Value::Integer(count) => {
                            live = live.saturating_add(count.to_u64().unwrap_or(u64::MAX));
                        }
                        _ => {}
                    },
                }
            }
        }
        live
    }

    /// [`TimedStop::MaxClocks`] when the live clocks of `state` exceed
    /// `max_clocks`.
    ///
    /// # Errors
    ///
    /// [`TimedStop::UnplacedElement`] when a live clock has no place: the
    /// listing is sized so that this never happens within the bound.
    fn within_max_clocks(&self, state: &ModelSimState) -> Result<(), TimedStop> {
        let live = self.live_clocks(state);
        if live > self.max_clocks.get() {
            return Err(TimedStop::MaxClocks {
                max_clocks: self.max_clocks.get(),
            });
        }
        let placed = self
            .places
            .iter()
            .filter(|place| self.live(place, state))
            .count() as u64;
        if placed < live {
            return Err(TimedStop::UnplacedElement { live, placed });
        }
        Ok(())
    }

    /// Whether a step of the model may carry a clock between slots
    /// ([`GuardedStep::carries`]): a step writes a clock collection whole
    /// (FR-230 "Collection-valued writes"), or a clock lies below a set or
    /// bag slot of elements with data, which a step matches by equal data
    /// wherever they land (FR-120-AC-26). The refinement zone checker has
    /// not yet mapped these carried clock places.
    pub fn carries_clocks(&self) -> bool {
        self.model
            .resets()
            .iter()
            .any(|resets| !resets.writes().is_empty())
            || self.model.clocks().iter().any(|clock| match clock.path() {
                // A collection of bare clocks: its slots hold no data, so
                // they keep their clocks by position.
                [PlaceStep::Field(_), PlaceStep::Slot(_)] => false,
                path => path
                    .iter()
                    .any(|step| matches!(step, PlaceStep::Slot(shape) if !shape.ordered)),
            })
    }

    /// The underlying model system.
    pub fn system(&self) -> &'s ModelSystem<'a> {
        self.system
    }

    /// The checked timed model.
    pub fn model(&self) -> &'s TimedModel {
        self.model
    }

    /// The time source.
    pub fn source(&self) -> &'s ModelTimeSource {
        self.source
    }

    /// The clock places: for each clock field in the timed model's order,
    /// each object of its owner's universe, in canonical order, and for a
    /// clock in collections each slot up to the declared maxima, in
    /// ascending slot order.
    pub fn places(&self) -> &[ClockPlace] {
        &self.places
    }

    /// QSpec FR-430 "Reductions over a dense-time subject": the image of
    /// each clock place under `permutation`, in place order. The place of
    /// clock `c` at slots `s` on the object held under key `k` maps to the
    /// place of `c` at the same slots on the object held under the key `k`
    /// is mapped to; `None` when that object has no such place.
    pub fn place_images(&self, permutation: &Permutation) -> Option<Vec<usize>> {
        self.places
            .iter()
            .map(|place| {
                let population = &place.instance.population;
                let key = permutation.image(population, &place.instance.key);
                self.places.iter().position(|other| {
                    other.clock == place.clock
                        && other.slots == place.slots
                        && other.instance.population == *population
                        && other.instance.key == key
                })
            })
            .collect()
    }

    /// Whether the presence bits of `post` are the ones `vector`'s step
    /// writes and its postconditions require (FR-230 "Presence bit"): a
    /// guarded place a postcondition requires present or absent is so at
    /// `post`; a reset of an `Option<Clock>` of the receiver, or a write of
    /// a union field's clock case constructor, makes it present, a drop
    /// (`self.v = absent`, a write of another case) makes it absent, and
    /// every other guarded place of an object `pre` holds keeps its
    /// presence, except that a union value may leave a clock case (QSpec
    /// FR-415-AC-21). A frame writing the field enumerates every value; this
    /// keeps the written one, so a case switch into a clock case that no
    /// write makes leaves no successor: there is no implicit reset. A reset
    /// of a `Clock` parameter writes the place `instance` binds it to.
    fn bits_follow(
        &self,
        vector: &super::frames::EnabledVector,
        instance: &[(usize, usize)],
        pre: &ModelSimState,
        post: &ModelSimState,
    ) -> bool {
        let clauses: Vec<_> = self
            .model
            .resets()
            .iter()
            .filter(|resets| {
                resets.operation().declaring == vector.declaring
                    && resets.operation().declaration.name() == vector.name
            })
            .collect();
        let set: Vec<ClockId> = clauses
            .iter()
            .flat_map(|resets| resets.resets())
            .filter(|reset| reset.selection().is_none() && reset.parameter().is_none())
            .map(|reset| reset.clock())
            .collect();
        // The places a reset of a `Clock` parameter writes for `instance`.
        let bound: Vec<usize> = clauses
            .iter()
            .flat_map(|resets| resets.resets())
            .filter(|reset| reset.selection().is_none())
            .filter_map(|reset| {
                let parameter = reset.parameter()?;
                instance
                    .iter()
                    .find(|(position, _)| *position == parameter)
                    .map(|(_, place)| *place)
                    .filter(|place| {
                        self.places
                            .get(*place)
                            .is_some_and(|held| held.clock == reset.clock())
                    })
            })
            .collect();
        let dropped: Vec<ClockId> = clauses
            .iter()
            .flat_map(|resets| resets.drops())
            .copied()
            .collect();
        // A postcondition's presence requirements on the receiver's
        // post-state (`present(self.v)`, `not present(self.v)`).
        let required = clauses.iter().flat_map(|resets| resets.requires()).copied();
        if !self.presences_hold(post, &vector.receiver, required) {
            return false;
        }
        self.places.iter().enumerate().all(|(index, place)| {
            // An `Option<Clock>` place, or a union case's clock outside
            // collections: guarded by its presence.
            // An element of a `Sequence<Option<Clock>>` follows once the
            // step's resets are known ([`Self::slot_bits_follow`]).
            let guarded = place.slots.is_empty()
                && self.model.clock(place.clock).is_some_and(|field| {
                    field.presence_bit()
                        || field
                            .path()
                            .iter()
                            .any(|step| matches!(step, PlaceStep::Union(_)))
                });
            if !guarded {
                return true;
            }
            if place.stored(pre).is_none() || place.stored(post).is_none() {
                return true;
            }
            let (before, after) = (self.live(place, pre), self.live(place, post));
            let receiver = place.holds(&vector.receiver);
            if receiver && dropped.contains(&place.clock) {
                return !after;
            }
            let written = (receiver && set.contains(&place.clock)) || bound.contains(&index);
            // QSpec FR-415-AC-21: a place becomes present only by a write
            // of its clock. An `Option<Clock>` bit does not change unless
            // written; a union value may leave its case, dropping the case's
            // clock, but never enters a clock case unwritten.
            let union = self
                .model
                .clock(place.clock)
                .is_some_and(|field| !field.presence_bit());
            if union {
                !after || written || before
            } else {
                after == (written || before)
            }
        })
    }

    /// The presence at `state` of every guarded place outside collections
    /// a predicate names `r.f` or `r.f.c`, by object, field and union case:
    /// what `present(r.f)` of another object reads (FR-230 "Presence is
    /// derived").
    fn place_presences(&self, state: &ModelSimState) -> PlacePresences {
        // FR-415-AC-20: each `Sequence<Option<Clock>>`'s element bits, which
        // `forall(e in r.q: present(e))` reads.
        let mut slots = std::collections::BTreeMap::new();
        for holder in &self.holders {
            let Some(field) = self
                .model
                .clock(holder.clock)
                .filter(|field| field.presence_bit())
            else {
                continue;
            };
            let [PlaceStep::Field(name), PlaceStep::Slot(_)] = field.path() else {
                continue;
            };
            let Some(quire_exact::Value::Collection(bits)) = holder
                .stored(state)
                .and_then(|stored| stored.fields.get(field.field()))
            else {
                continue;
            };
            slots.insert(
                (holder.object.clone(), name.clone()),
                bits.elements()
                    .iter()
                    .map(|bit| matches!(bit, quire_exact::Value::Boolean(true)))
                    .collect(),
            );
        }
        let places = self
            .places
            .iter()
            .filter(|place| place.slots.is_empty())
            .filter_map(|place| {
                let field = self.model.clock(place.clock)?;
                let (name, case) = match field.path() {
                    [PlaceStep::Field(name)] if field.presence_bit() => (name, None),
                    [PlaceStep::Field(name), PlaceStep::Union(case)] => (name, Some(case.clone())),
                    _ => return None,
                };
                Some((
                    (place.object.clone(), name.clone(), case),
                    self.live(place, state),
                ))
            })
            .collect();
        PlacePresences { places, slots }
    }

    /// Whether every element of a `Sequence<Option<Clock>>` that its
    /// collection holds at both `pre` and `post` keeps its presence bit,
    /// unless `resets` resets its place, which makes it present (FR-230
    /// "Presence bit"; no implicit reset or drop, as of an `Option<Clock>`
    /// field). An element the step adds or removes follows the collection.
    fn slot_bits_follow(
        &self,
        pre: &ModelSimState,
        post: &ModelSimState,
        resets: &[(usize, Rational)],
    ) -> bool {
        let bits = |state: &ModelSimState, place: &ClockPlace, field: &DeclarationKey| match place
            .stored(state)?
            .fields
            .get(field)?
        {
            quire_exact::Value::Collection(bits) => Some(bits.elements().len()),
            _ => None,
        };
        self.places.iter().enumerate().all(|(at, place)| {
            let Some(field) = self
                .model
                .clock(place.clock)
                .filter(|field| field.presence_bit())
            else {
                return true;
            };
            let [slot] = place.slots[..] else {
                return true;
            };
            let held = |state: &ModelSimState| {
                bits(state, place, field.field()).is_some_and(|len| slot < len)
            };
            if !held(pre) || !held(post) {
                return true;
            }
            let (before, after) = (self.live(place, pre), self.live(place, post));
            before == after || (after && resets.iter().any(|(reset, _)| *reset == at))
        })
    }

    /// Whether each `Option<Clock>` place of `object` in `presences` is
    /// present at `state` exactly when its entry says so (FR-230 "Presence
    /// bit").
    fn presences_hold(
        &self,
        state: &ModelSimState,
        object: &ObjectReference,
        presences: impl IntoIterator<Item = (ClockId, bool)>,
    ) -> bool {
        presences.into_iter().all(|(clock, present)| {
            let live = self
                .place(object, clock)
                .and_then(|place| self.places.get(place))
                .is_some_and(|place| self.live(place, state));
            live == present
        })
    }

    /// The place of `clock` on `object`, `None` for a clock in a collection,
    /// which has one place for each slot.
    pub fn place(&self, object: &ObjectReference, clock: ClockId) -> Option<usize> {
        self.places
            .iter()
            .position(|place| place.clock == clock && place.slots.is_empty() && place.holds(object))
    }

    /// The place of `clock` on `object` at the slots `slots`, one for each
    /// collection on the clock's path.
    fn place_at(&self, object: &ObjectReference, clock: ClockId, slots: &[u64]) -> Option<usize> {
        self.places.iter().position(|place| {
            place.clock == clock
                && place.holds(object)
                && place.slots.len() == slots.len()
                && place
                    .slots
                    .iter()
                    .zip(slots)
                    .all(|(held, wanted)| u64::try_from(*held).is_ok_and(|held| held == *wanted))
        })
    }

    /// Whether `place` has a value at `state`: `state` holds its object
    /// and every collection on its path holds its slot (FR-230 "Clock
    /// collections": a slot is live exactly while the collection holds it).
    pub fn live(&self, place: &ClockPlace, state: &ModelSimState) -> bool {
        let Some(stored) = place.stored(state) else {
            return false;
        };
        let Some(field) = self.model.clock(place.clock) else {
            return false;
        };
        // An `Option<Clock>` place is live while its presence bit is set
        // (FR-230 "Presence bit").
        if field.presence_bit() && place.slots.is_empty() {
            return matches!(
                stored.fields.get(field.field()),
                Some(quire_exact::Value::Boolean(true))
            );
        }
        let guarded = field
            .path()
            .iter()
            .any(|step| matches!(step, PlaceStep::Union(_)));
        if place.slots.is_empty() && !guarded {
            return true;
        }
        let typing = self.system.typing();
        let mut slots = place.slots.iter();
        // The steps through the innermost collection: the clock below it is
        // no data component to read.
        let through = field
            .path()
            .iter()
            .rposition(|step| matches!(step, PlaceStep::Slot(_) | PlaceStep::Union(_)))
            .map_or(0, |last| last + 1);
        let mut steps = field.path()[..through].iter();
        let Some(PlaceStep::Field(first)) = steps.next() else {
            return false;
        };
        let Some(fields) = self.system.fields.get(&stored.type_identity) else {
            return false;
        };
        let Some(mut value) = fields
            .iter()
            .find(|declared| declared.name == *first)
            .and_then(|declared| stored.fields.get(&declared.key))
        else {
            return false;
        };
        for step in steps {
            let next = match step {
                PlaceStep::Field(name) => typing.component(value, name),
                // FR-230: a union case's place is live while the value is
                // that case.
                PlaceStep::Union(case) => is_case(value, case).then_some(value),
                PlaceStep::Slot(_) => match (value, slots.next()) {
                    (quire_exact::Value::Collection(collection), Some(slot)) => {
                        collection.elements().get(*slot)
                    }
                    // A collection of bare clocks: its count member
                    // (FR-120-AC-26) holds the slots below it.
                    (quire_exact::Value::Integer(count), Some(slot)) => {
                        return quire_exact::Integer::from(*slot as u64) < *count;
                    }
                    _ => None,
                },
            };
            match next {
                Some(next) => value = next,
                None => return false,
            }
        }
        // An element of a `Sequence<Option<Clock>>` is live while the
        // collection holds it and its bit is set (FR-230-AC-41).
        !field.presence_bit() || matches!(value, quire_exact::Value::Boolean(true))
    }

    /// The initial timed states: each initial model state with every clock 0.
    pub fn initial(&self) -> Vec<TimedState> {
        // A slot the initial state does not hold is not live; its value is
        // reset when the slot becomes live.
        let zero = ClockValuation(vec![zero(); self.places.len()]);
        let mut discrete: Vec<(u64, ModelSimState)> = Vec::new();
        for state in &self.system.initial {
            discrete.extend(self.unbounded_counts(state));
        }
        // Ascending count (FR-230 "Initial element count"), each base
        // state's own order kept among equal counts.
        discrete.sort_by_key(|(added, _)| *added);
        discrete
            .into_iter()
            .map(|(_, discrete)| TimedState {
                discrete,
                clocks: zero.clone(),
            })
            .collect()
    }

    /// `state` with each element count of a pure-clock collection with no
    /// declared maximum ranging up from its `min` (FR-230 "Initial element
    /// count"), each with the elements it adds: every distribution of `k`
    /// added elements over the counts, for `k` = 0, 1, … while the live
    /// clocks stay within `max_clocks`, then one state past the bound, at
    /// which the analysis stops `limit-reached`. Every element adds one live
    /// clock, so every distribution of one `k` is within the bound or none
    /// is.
    fn unbounded_counts(&self, state: &ModelSimState) -> Vec<(u64, ModelSimState)> {
        let holders: Vec<(DeclarationKey, String, DeclarationKey, u64)> = self
            .system
            .clock_counts
            .iter()
            .filter(|(_, _, _, max)| max.is_none())
            .flat_map(|(type_key, field, min, _)| {
                state
                    .state
                    .populations
                    .iter()
                    .flat_map(move |(population, objects)| {
                        objects
                            .iter()
                            .filter(move |(_, object)| object.type_identity == *type_key)
                            .map(move |(key, _)| {
                                (population.clone(), key.clone(), field.key.clone(), *min)
                            })
                    })
            })
            .collect();
        if holders.is_empty() {
            return vec![(0, state.clone())];
        }
        let grown = |added: &[u64]| {
            let mut next = state.clone();
            for ((population, key, field, min), extra) in holders.iter().zip(added) {
                let Some(object) = next
                    .state
                    .populations
                    .get_mut(population)
                    .and_then(|objects| objects.get_mut(key))
                else {
                    continue;
                };
                object.fields.insert(
                    field.clone(),
                    quire_exact::Value::Integer(Integer::from(min.saturating_add(*extra))),
                );
            }
            next
        };
        let mut states = Vec::new();
        for total in 0_u64.. {
            let level: Vec<ModelSimState> = distributions(total, holders.len())
                .iter()
                .map(|added| grown(added))
                .collect();
            let Some(first) = level.first() else {
                break;
            };
            if self.live_clocks(first) > self.max_clocks.get() {
                states.push((total, first.clone()));
                break;
            }
            states.extend(level.into_iter().map(|state| (total, state)));
        }
        states
    }

    /// Each predicate at `state` about each object of `context`'s universe:
    /// `true` when it holds, `false` when it does not or the state holds no
    /// such object.
    fn about(
        &self,
        state: &ModelSimState,
        predicate: &CheckedTimedPredicate,
        context: EffectiveId,
        site: impl Fn() -> TimedSite,
    ) -> Result<Vec<(ObjectReference, bool)>, TimedStop> {
        let objects = self.objects.get(&context).map_or(&[][..], Vec::as_slice);
        let bound: Vec<_> = objects
            .iter()
            .map(|object| (predicate, Some(object)))
            .collect();
        let bound: Vec<_> = bound
            .into_iter()
            .map(|(predicate, over)| (predicate, over, Vec::new()))
            .collect();
        let readings =
            self.system
                .state_predicates_with(state, &bound, self.place_presences(state))?;
        objects
            .iter()
            .zip(readings)
            .map(|(object, reading)| {
                let holds = match reading {
                    PredicateReading::True => true,
                    PredicateReading::False => false,
                    PredicateReading::Undefined { cause, .. } if absent(&cause) => false,
                    PredicateReading::Undefined { cause, .. } => {
                        return Err(TimedStop::undecided(site(), object.clone(), Some(cause)))
                    }
                    PredicateReading::Refused(_) => {
                        return Err(TimedStop::undecided(site(), object.clone(), None))
                    }
                };
                Ok((object.clone(), holds))
            })
            .collect()
    }

    /// The upper bounds of every time invariant that applies at `state`,
    /// over places: an invariant applies to an object when its `when`
    /// predicate holds of it.
    ///
    /// # Errors
    ///
    /// [`TimedStop`] when the state cannot be observed or a predicate has no
    /// truth value.
    pub fn invariants(&self, state: &ModelSimState) -> Result<Vec<PlaceConstraint>, TimedStop> {
        let mut bounds = Vec::new();
        for invariant in self.model.invariants() {
            let holding = self.holding(
                state,
                invariant.alternatives(),
                invariant.context(),
                &|| TimedSite::Invariant {
                    name: invariant.name().to_owned(),
                },
            )?;
            for object in holding {
                bounds.extend(self.over(&object, invariant.bounds(), state));
            }
        }
        Ok(bounds)
    }

    /// The name of the first time invariant, in model order, that applies
    /// at `state`'s discrete state and whose bound `state`'s clocks break:
    /// what a delay or target refusal names (FR-237). `None` when every
    /// applying bound holds.
    ///
    /// # Errors
    ///
    /// As [`Self::invariants`].
    pub fn broken_invariant(&self, state: &TimedState) -> Result<Option<String>, TimedStop> {
        for invariant in self.model.invariants() {
            let holding = self.holding(
                &state.discrete,
                invariant.alternatives(),
                invariant.context(),
                &|| TimedSite::Invariant {
                    name: invariant.name().to_owned(),
                },
            )?;
            for object in holding {
                if !self
                    .over(&object, invariant.bounds(), &state.discrete)
                    .all(|bound| bound.holds(&state.clocks))
                {
                    return Ok(Some(invariant.name().to_owned()));
                }
            }
        }
        Ok(None)
    }

    /// The objects of type `context` at `state` of which one of
    /// `alternatives` holds: its data predicate holds of the object and the
    /// object's `Option<Clock>` places have the presences it requires
    /// (FR-230 "Presence bit"), in object order.
    fn holding(
        &self,
        state: &ModelSimState,
        alternatives: &[qsl_semantics::check::StateAlternative],
        context: EffectiveId,
        site: &dyn Fn() -> TimedSite,
    ) -> Result<Vec<ObjectReference>, TimedStop> {
        let mut holding: Vec<ObjectReference> = Vec::new();
        for alternative in alternatives {
            for (object, holds) in self.about(state, alternative.predicate(), context, site)? {
                if holds
                    && !holding.contains(&object)
                    && self.presences_hold(state, &object, alternative.presences().iter().copied())
                {
                    holding.push(object);
                }
            }
        }
        let objects = self.objects.get(&context).map_or(&[][..], Vec::as_slice);
        holding.sort_by_key(|object| objects.iter().position(|held| held == object));
        Ok(holding)
    }

    /// `constraints` over `object`'s places live at `state`: a clock place
    /// of the object, and for a constraint whose slots are each
    /// [`SlotRef::Family`] every live slot place of its clock
    /// (FR-230 "Clock constraints on a type"). A place that is not live has
    /// no value to bound.
    fn over<'c>(
        &'c self,
        object: &'c ObjectReference,
        constraints: &'c [ClockConstraint],
        state: &'c ModelSimState,
    ) -> impl Iterator<Item = PlaceConstraint> + 'c {
        constraints.iter().flat_map(move |constraint| {
            let every_slot = !constraint.slots().is_empty()
                && constraint
                    .slots()
                    .iter()
                    .all(|slot| *slot == SlotRef::Family);
            self.places
                .iter()
                .enumerate()
                .filter(move |(_, place)| {
                    place.clock == constraint.clock()
                        && place.holds(object)
                        && if every_slot {
                            place.slots.len() == constraint.slots().len()
                        } else {
                            place.slots.is_empty()
                        }
                        && self.live(place, state)
                })
                .map(move |(place, _)| PlaceConstraint {
                    place,
                    op: constraint.op(),
                    bound: constraint.bound().clone(),
                })
        })
    }

    /// Whether `state` is urgent: an `urgent when` predicate holds of some
    /// object, or an `urgent O` operation is enabled.
    ///
    /// # Errors
    ///
    /// [`TimedStop`] when the state cannot be expanded or a predicate has no
    /// truth value.
    pub fn is_urgent(&self, state: &ModelSimState) -> Result<bool, TimedStop> {
        for urgency in self.model.urgency() {
            if let Urgency::When {
                name,
                context,
                alternatives,
            } = urgency
            {
                let holding = self.holding(state, alternatives, *context, &|| {
                    TimedSite::Urgency { name: name.clone() }
                })?;
                if !holding.is_empty() {
                    return Ok(true);
                }
            }
        }
        if self.urgent_operations.is_empty() {
            return Ok(false);
        }
        Ok(self.guarded_steps(state)?.iter().any(|step| {
            !step.parts.is_empty()
                && self
                    .urgent_operations
                    .iter()
                    .any(|urgent| urgent == &step.operation)
        }))
    }

    /// Whether `state` is quiescent (FR-231 "Quiescent states"): no time
    /// invariant applies at it and it is not urgent.
    ///
    /// # Errors
    ///
    /// As [`Self::invariants`] and [`Self::is_urgent`].
    pub fn is_quiescent(&self, state: &ModelSimState) -> Result<bool, TimedStop> {
        Ok(self.invariants(state)?.is_empty() && !self.is_urgent(state)?)
    }

    /// Every step enabled at `state` by its data precondition, in canonical
    /// transition order, with the clock part of its guard and its resets
    /// over places.
    ///
    /// # Errors
    ///
    /// [`TimedStop`] when the state cannot be expanded or a guard part's
    /// predicate has no truth value.
    pub fn guarded_steps(&self, state: &ModelSimState) -> Result<Vec<GuardedStep>, TimedStop> {
        self.within_max_clocks(state)?;
        let (expansion, enabled) = expand_enabled(self.system, state)?;
        let mut steps = Vec::new();
        for vector in enabled {
            // Each binding of the operation's `Clock` parameters to places
            // (FR-230 "Operation parameters"): one instance per binding.
            let instances = self.instances(&vector, state);
            for position in vector.successors.clone() {
                let Some((transition, post)) = expansion.successors.get(position) else {
                    continue;
                };
                self.within_max_clocks(post)?;
                for instance in &instances {
                    if !self.bits_follow(&vector, instance, state, post) {
                        continue;
                    }
                    let transition = &self
                        .instance_transition(transition, instance)
                        .ok_or_else(|| stop(BROKEN))?;
                    let mut parts = vec![Vec::new()];
                    let mut guards = Vec::new();
                    for guard in self.model.guards() {
                        let operation = guard.operation();
                        if operation.declaring != vector.declaring
                            || operation.declaration.name() != vector.name
                        {
                            continue;
                        }
                        guards.push(guard.clause().to_owned());
                        let mut clause_parts = Vec::new();
                        for part in guard.parts() {
                            if !self.presences_hold(
                                state,
                                &vector.receiver,
                                part.presences().iter().copied(),
                            ) {
                                continue;
                            }
                            let data = match (
                                part.data(),
                                self.elements(state, &vector.receiver, part.elements()),
                            ) {
                                (None, _) => true,
                                // A part reading a slot the collection does not
                                // hold is not enabled.
                                (Some(_), None) => false,
                                (Some(data), Some(elements)) => {
                                    self.about_one(state, data, &vector.receiver, elements, || {
                                        TimedSite::Guard {
                                            clause: guard.clause().to_owned(),
                                        }
                                    })?
                                }
                            };
                            if data {
                                // A part reading a `Clock` parameter bound to a
                                // place of another clock is not this instance's.
                                let Some(base) =
                                    self.bound(state, &vector.receiver, part.clocks(), instance)?
                                else {
                                    continue;
                                };
                                clause_parts.extend(self.instantiated(
                                    state,
                                    &vector.receiver,
                                    part.families(),
                                    base,
                                    &|| TimedSite::Guard {
                                        clause: guard.clause().to_owned(),
                                    },
                                )?);
                            }
                        }
                        parts = parts
                            .iter()
                            .flat_map(|left| {
                                clause_parts.iter().map(move |right| {
                                    left.iter().chain(right).cloned().collect::<Vec<_>>()
                                })
                            })
                            .collect();
                    }
                    // The clocks the step carries with their elements and the
                    // fresh elements' zeros first: a reset of the post-state
                    // then writes the element the post-state holds.
                    let mut resets: Vec<(usize, Rational)> = Vec::new();
                    let mut carries = self.written(&vector.receiver, post, &mut resets);
                    self.matched(&vector.receiver, state, post, &mut resets, &mut carries);
                    for (clause, reset) in self
                        .model
                        .resets()
                        .iter()
                        .filter(|resets| {
                            resets.operation().declaring == vector.declaring
                                && resets.operation().declaration.name() == vector.name
                        })
                        .flat_map(|resets| {
                            resets.resets().iter().map(|reset| (resets.clause(), reset))
                        })
                    {
                        let Some(selection) = reset.selection() else {
                            let place = match reset.parameter() {
                                Some(parameter) => instance
                                    .iter()
                                    .find(|(position, _)| *position == parameter)
                                    .map(|(_, place)| *place)
                                    .filter(|place| {
                                        self.places
                                            .get(*place)
                                            .is_some_and(|held| held.clock == reset.clock())
                                    }),
                                None => self.place(&vector.receiver, reset.clock()),
                            };
                            if let Some(place) = place {
                                resets.push((place, reset.value().clone()));
                            }
                            continue;
                        };
                        // Each slot the post-state holds whose element the
                        // selection's predicate admits: the postcondition reads
                        // the collection the step leaves.
                        for slot in 0_u64.. {
                            let Some(place) =
                                self.place_at(&vector.receiver, reset.clock(), &[slot])
                            else {
                                break;
                            };
                            let binding = ElementBinding::new(
                                selection.name().to_owned(),
                                selection.path().to_vec(),
                                SlotRef::Held(slot),
                            );
                            let Some(elements) = self.elements(
                                post,
                                &vector.receiver,
                                std::slice::from_ref(&binding),
                            ) else {
                                continue;
                            };
                            let selected = match selection.predicate() {
                                None => true,
                                Some(predicate) => self.about_one(
                                    post,
                                    predicate,
                                    &vector.receiver,
                                    elements,
                                    || TimedSite::Reset {
                                        clause: clause.to_owned(),
                                    },
                                )?,
                            };
                            if selected {
                                resets.push((place, reset.value().clone()));
                            }
                        }
                    }
                    if !self.slot_bits_follow(state, post, &resets) {
                        continue;
                    }
                    // A clock whose object or slot the step makes live takes
                    // the constant of its write, 0 when the step writes none
                    // (QSpec FR-181 "Timed subjects").
                    for (place, clock) in self.places.iter().enumerate() {
                        if !self.live(clock, state)
                            && self.live(clock, post)
                            && resets.iter().all(|(reset, _)| *reset != place)
                            && carries.iter().all(|(carried, _)| *carried != place)
                        {
                            resets.push((place, zero()));
                        }
                    }
                    steps.push(GuardedStep {
                        transition: transition.clone(),
                        receiver: vector.receiver.clone(),
                        operation: vector.operation.clone(),
                        post: post.clone(),
                        parts,
                        resets,
                        carries,
                        guards,
                    });
                }
            }
        }
        // FR-101 canonical transition order: by the identity's canonical
        // bytes.
        let mut keyed = steps
            .into_iter()
            .map(|step| {
                let key = self
                    .system
                    .transition_key(&step.transition)
                    .map_err(|_| stop(BROKEN))?;
                Ok((key, step))
            })
            .collect::<Result<Vec<_>, ExpansionStop>>()?;
        keyed.sort_by(|left, right| left.0.cmp(&right.0));
        let steps = keyed.into_iter().map(|(_, step)| step).collect();
        Ok(steps)
    }

    /// Each binding of `vector`'s operation's `Clock` parameters to places
    /// live at `state`, by parameter position and place index (FR-230
    /// "Operation parameters"): a parameter ranges over the places of the
    /// clock fields of its range, every clock field when the model lists
    /// none. One empty binding for an operation with none. A parameter the
    /// postcondition resets also binds an absent `Option<Clock>` place
    /// outside collections, which the reset makes present (QSpec
    /// FR-415-AC-21).
    fn instances(
        &self,
        vector: &super::frames::EnabledVector,
        state: &ModelSimState,
    ) -> Vec<Vec<(usize, usize)>> {
        let Some(declaration) = self
            .system
            .operations
            .declared_by(vector.declaring)
            .iter()
            .find(|declared| declared.name() == vector.name)
        else {
            return vec![Vec::new()];
        };
        let mut instances: Vec<Vec<(usize, usize)>> = vec![Vec::new()];
        for (position, _) in declaration.clock_parameters() {
            let reset = self.model.resets().iter().any(|resets| {
                resets.operation().declaring == vector.declaring
                    && resets.operation().declaration.name() == vector.name
                    && resets
                        .resets()
                        .iter()
                        .any(|reset| reset.parameter() == Some(*position))
            });
            let range = self.model.clock_parameters().iter().find(|listed| {
                listed.position() == *position
                    && listed.operation().declaring == vector.declaring
                    && listed.operation().declaration.name() == vector.name
            });
            let places: Vec<usize> = self
                .places
                .iter()
                .enumerate()
                .filter(|(_, place)| {
                    range.is_none_or(|range| {
                        range.range().contains(&place.clock)
                            && (!range.receiver_only().contains(&place.clock)
                                || place.holds(&vector.receiver))
                    }) && (self.live(place, state)
                        || (reset
                            && place.slots.is_empty()
                            && self
                                .model
                                .clock(place.clock)
                                .is_some_and(|field| field.presence_bit())))
                })
                .map(|(index, _)| index)
                .collect();
            instances = instances
                .into_iter()
                .flat_map(|bound| {
                    places.iter().map(move |place| {
                        let mut next = bound.clone();
                        next.push((*position, *place));
                        next
                    })
                })
                .collect();
        }
        instances
    }

    /// `transition` with each `Clock` parameter's argument the place
    /// `instance` binds it to: an opaque `clock_place` document naming the
    /// object, the clock field and the slots, which no data value encodes
    /// as. The place arguments follow the data arguments, in declared
    /// order, so a data argument keeps its index among the data parameters
    /// that every reader (hyper step reads, footprints) indexes by.
    fn instance_transition(
        &self,
        transition: &quire_semantic_value::model_transition::ModelTransition,
        instance: &[(usize, usize)],
    ) -> Option<quire_semantic_value::model_transition::ModelTransition> {
        if instance.is_empty() {
            return Some(transition.clone());
        }
        let mut arguments: Vec<String> = transition.arguments().to_vec();
        let mut bound: Vec<&(usize, usize)> = instance.iter().collect();
        bound.sort();
        for (_, place) in bound {
            let text = self
                .place_enc(*place)?
                .canonical_text(quire_canonical::Limits::new(u64::MAX))
                .ok()?;
            arguments.push(text);
        }
        quire_semantic_value::model_transition::ModelTransition::new(
            transition.operation().to_owned(),
            transition.receiver().map(str::to_owned),
            arguments,
            quire_canonical::Limits::new(u64::MAX),
        )
        .ok()
    }

    /// The place at index `place` as an opaque `clock_place` document.
    fn place_enc(&self, place: usize) -> Option<super::encode::Enc> {
        let place = self.places.get(place)?;
        let field = self.model.clock(place.clock)?;
        let mut slots = place.slots.iter();
        Some(super::encode::Enc::tagged(
            "clock_place",
            vec![
                ("object", super::encode::reference_enc(&place.object)),
                ("field", super::encode::declaration_enc(field.field())),
                (
                    "path",
                    super::encode::Enc::Array(
                        field
                            .path()
                            .iter()
                            .map(|step| place_step_enc(step, &mut slots))
                            .collect(),
                    ),
                ),
            ],
        ))
    }

    /// [`Self::placed`] for an instance: a constraint reading a `Clock`
    /// parameter constrains the place `instance` binds it to; `None` when
    /// that place is not of the constraint's clock, so the part is not this
    /// instance's.
    fn bound(
        &self,
        state: &ModelSimState,
        receiver: &ObjectReference,
        constraints: &[ClockConstraint],
        instance: &[(usize, usize)],
    ) -> Result<Option<Vec<PlaceConstraint>>, ExpansionStop> {
        let (parameters, own): (Vec<&ClockConstraint>, Vec<&ClockConstraint>) = constraints
            .iter()
            .partition(|constraint| constraint.parameter().is_some());
        let own: Vec<ClockConstraint> = own.into_iter().cloned().collect();
        let mut placed = self.placed(state, receiver, &own)?;
        for constraint in parameters {
            let place = instance
                .iter()
                .find(|(position, _)| Some(*position) == constraint.parameter())
                .map(|(_, place)| *place)
                .filter(|place| {
                    self.places.get(*place).is_some_and(|held| {
                        held.clock == constraint.clock() && self.live(held, state)
                    })
                });
            // A guard reading the parameter where the instance binds it to an
            // absent place has no part.
            let Some(place) = place else {
                return Ok(None);
            };
            placed.push(PlaceConstraint {
                place,
                op: constraint.op(),
                bound: constraint.bound().clone(),
            });
        }
        Ok(Some(placed))
    }

    /// The carries of the collections the step to `post` wrote whole on
    /// `receiver` (FR-230 "Collection-valued writes", QSpec FR-181-AC-25):
    /// each clock of a slot carried from slot `j` of `g` takes the value of
    /// the clock at the same path below slot `j` of `g`, and each clock of a
    /// fresh slot its literal's constant, pushed onto `resets`.
    fn written(
        &self,
        receiver: &ObjectReference,
        post: &ModelSimState,
        resets: &mut Vec<(usize, Rational)>,
    ) -> Vec<(usize, usize)> {
        let mut carries = Vec::new();
        let Some(effect) = post.reached_by() else {
            return carries;
        };
        for written in &effect.written {
            for (slot, provenance) in written.provenance.iter().enumerate() {
                match provenance {
                    Provenance::Fresh(clocks) => {
                        for (clock, value) in clocks {
                            if let Some(place) = u64::try_from(slot)
                                .ok()
                                .and_then(|slot| self.place_at(receiver, *clock, &[slot]))
                            {
                                resets.push((place, value.clone()));
                            }
                        }
                    }
                    Provenance::Carried { field, slot: from } => {
                        for (to, place) in self.places.iter().enumerate() {
                            if !place.holds(receiver) || place.slots.first() != Some(&slot) {
                                continue;
                            }
                            let Some(source) =
                                self.carried_clock(place.clock, &written.field, field)
                            else {
                                continue;
                            };
                            let mut slots: Vec<u64> = Vec::with_capacity(place.slots.len());
                            for held in std::iter::once(from).chain(&place.slots[1..]) {
                                match u64::try_from(*held) {
                                    Ok(held) => slots.push(held),
                                    Err(_) => break,
                                }
                            }
                            if let Some(source) = self.place_at(receiver, source, &slots) {
                                carries.push((to, source));
                            }
                        }
                    }
                }
            }
        }
        carries
    }

    /// The clocks of every collection the step to `post` changes and does
    /// not write whole, on every object (FR-120 "collection domains",
    /// FR-120-AC-26): each clock place live at `post` keeps the clock of
    /// the element it is in, read collection by collection along its path.
    /// A sequence slot keeps its position; a set or bag element is matched
    /// to a pre-state element with equal data, one to one in ascending slot
    /// order. A place whose element moved is a carry from the place its
    /// element held; one in an element with no match is fresh, at 0. The
    /// receiver's collections a postcondition writes whole are
    /// [`Self::written`]'s.
    fn matched(
        &self,
        receiver: &ObjectReference,
        state: &ModelSimState,
        post: &ModelSimState,
        resets: &mut Vec<(usize, Rational)>,
        carries: &mut Vec<(usize, usize)>,
    ) {
        let written: Vec<&str> = post
            .reached_by()
            .map(|effect| {
                effect
                    .written
                    .iter()
                    .map(|written| written.field.as_str())
                    .collect()
            })
            .unwrap_or_default();
        let index: std::collections::BTreeMap<(&ObjectReference, ClockId, &[usize]), usize> = self
            .places
            .iter()
            .enumerate()
            .map(|(at, place)| ((&place.object, place.clock, place.slots.as_slice()), at))
            .collect();
        for (to, place) in self.places.iter().enumerate() {
            if place.slots.is_empty() || !self.live(place, post) {
                continue;
            }
            let Some(field) = self.model.clock(place.clock) else {
                continue;
            };
            if place.holds(receiver)
                && matches!(field.path().first(), Some(PlaceStep::Field(head)) if written.contains(&head.as_str()))
            {
                continue;
            }
            let source = match self.source_slots(place, state, post) {
                Some(Some(slots)) if slots == place.slots => continue,
                Some(Some(slots)) => index
                    .get(&(&place.object, place.clock, slots.as_slice()))
                    .copied(),
                Some(None) => None,
                None => continue,
            };
            match source {
                Some(from) => carries.push((to, from)),
                // A clock that was live and is fresh now; one that was not
                // live is zeroed with every newly live clock.
                None if self.live(place, state) => resets.push((to, zero())),
                None => {}
            }
        }
    }

    /// The slots of the pre-state element whose clock `place` keeps at
    /// `post`, read collection by collection along the clock's path:
    /// `Some(None)` when its element is fresh, `None` when the path cannot
    /// be read.
    fn source_slots(
        &self,
        place: &ClockPlace,
        state: &ModelSimState,
        post: &ModelSimState,
    ) -> Option<Option<Vec<usize>>> {
        let field = self.model.clock(place.clock)?;
        let path = field.path();
        let through = path
            .iter()
            .rposition(|step| matches!(step, PlaceStep::Slot(_)))
            .map_or(0, |last| last + 1);
        let Some(PlaceStep::Field(first)) = path.first() else {
            return None;
        };
        let after = place.stored(post)?;
        let Some(before) = place.stored(state) else {
            return Some(None);
        };
        fn read<'v>(
            fields: &std::collections::BTreeMap<
                quire_checked::key::DeclarationKey,
                Vec<super::StateField>,
            >,
            stored: &'v qsl_semantics::model::state::StateObject,
            first: &str,
        ) -> Option<&'v quire_exact::Value> {
            fields
                .get(&stored.type_identity)?
                .iter()
                .find(|declared| declared.name == first)
                .and_then(|declared| stored.fields.get(&declared.key))
        }
        let typing = self.system.typing();
        let equal = super::encode::values_equal;
        let mut post_value = read(&self.system.fields, after, first)?;
        let mut pre_value = read(&self.system.fields, before, first);
        let mut slots = place.slots.iter();
        let mut source = Vec::with_capacity(place.slots.len());
        for step in path.get(1..through)? {
            match step {
                PlaceStep::Field(name) => {
                    post_value = typing.component(post_value, name)?;
                    pre_value = pre_value.and_then(|value| typing.component(value, name));
                }
                PlaceStep::Union(case) => {
                    if !is_case(post_value, case) {
                        return None;
                    }
                    pre_value = pre_value.filter(|value| is_case(value, case));
                }
                PlaceStep::Slot(shape) => {
                    let slot = *slots.next()?;
                    let Some(pre) = pre_value else {
                        return Some(None);
                    };
                    match (pre, post_value) {
                        (
                            quire_exact::Value::Collection(before),
                            quire_exact::Value::Collection(after),
                        ) => {
                            let element = after.elements().get(slot)?;
                            let from = if shape.ordered {
                                (slot < before.elements().len()).then_some(slot)
                            } else {
                                // One to one, in ascending slot order.
                                let mut used = vec![false; before.elements().len()];
                                let mut found = None;
                                for (at, held) in after.elements().iter().enumerate() {
                                    let matched = before.elements().iter().enumerate().position(
                                        |(index, candidate)| !used[index] && equal(candidate, held),
                                    );
                                    if let Some(index) = matched {
                                        used[index] = true;
                                    }
                                    if at == slot {
                                        found = matched;
                                        break;
                                    }
                                }
                                found
                            };
                            let Some(from) = from else {
                                return Some(None);
                            };
                            source.push(from);
                            post_value = element;
                            pre_value = before.elements().get(from);
                        }
                        // A collection of bare clocks: by position.
                        (quire_exact::Value::Integer(before), quire_exact::Value::Integer(_)) => {
                            if Integer::from(slot as u64) >= *before {
                                return Some(None);
                            }
                            source.push(slot);
                        }
                        _ => return None,
                    }
                }
            }
        }
        Some(Some(source))
    }

    /// The clock of the collection `from` at the path below its slot that
    /// `clock`, a clock of the collection `to`, has below its own.
    fn carried_clock(&self, clock: ClockId, to: &str, from: &str) -> Option<ClockId> {
        let path = self.model.clock(clock)?.path();
        let [PlaceStep::Field(head), PlaceStep::Slot(_), below @ ..] = path else {
            return None;
        };
        if head != to {
            return None;
        }
        if to == from {
            return Some(clock);
        }
        self.model.clock_fields().find_map(|(candidate, field)| {
            let [PlaceStep::Field(head), PlaceStep::Slot(_), rest @ ..] = field.path() else {
                return None;
            };
            let same = head == from
                && rest.len() == below.len()
                && rest
                    .iter()
                    .zip(below)
                    .all(|(left, right)| match (left, right) {
                        (PlaceStep::Field(left), PlaceStep::Field(right)) => left == right,
                        (PlaceStep::Slot(_), PlaceStep::Slot(_)) => true,
                        _ => false,
                    });
            same.then_some(candidate)
        })
    }

    /// `constraints` of an operation's guard part over places, each of
    /// `receiver`'s own clock or of the object `receiver` reaches at `state`
    /// through the constraint's reference fields (FR-230-AC-10).
    ///
    /// # Errors
    ///
    /// A broken-invariant [`ExpansionStop`] when a reference field does not
    /// resolve to a present object of the universe, which FR-120's closure
    /// of a required single reference rules out.
    fn placed(
        &self,
        state: &ModelSimState,
        receiver: &ObjectReference,
        constraints: &[ClockConstraint],
    ) -> Result<Vec<PlaceConstraint>, ExpansionStop> {
        constraints
            .iter()
            .map(|constraint| {
                let place = self
                    .holder(state, receiver, constraint.via())
                    .and_then(|holder| {
                        self.place_at(&holder, constraint.clock(), &held_slots(constraint)?)
                    })
                    .filter(|place| {
                        constraint.via().is_empty()
                            || self
                                .places
                                .get(*place)
                                .is_some_and(|held| held.present(state))
                    })
                    .ok_or_else(|| stop(BROKEN))?;
                Ok(PlaceConstraint {
                    place,
                    op: constraint.op(),
                    bound: constraint.bound().clone(),
                })
            })
            .collect()
    }

    /// The guard part `base` conjoined with each quantifier family of the
    /// part, read at `state` (FR-230 "Quantified reads"): a family whose
    /// collection holds `n` elements is read at each slot below `n`, every
    /// slot enabling one of its template parts when it is `each`, some slot
    /// otherwise. Each alternative is one conjunction of place constraints.
    ///
    /// # Errors
    ///
    /// As [`Self::placed`] and the data predicates of the template parts;
    /// `resource_exhausted`/`insufficient-next-charge` naming
    /// `max_candidates` when the alternatives, a product over the held
    /// slots, would exceed the expansion's `max_candidates`.
    fn instantiated(
        &self,
        state: &ModelSimState,
        receiver: &ObjectReference,
        families: &[QuantifiedFamily],
        base: Vec<PlaceConstraint>,
        site: &dyn Fn() -> TimedSite,
    ) -> Result<Vec<Vec<PlaceConstraint>>, TimedStop> {
        let mut alternatives = vec![base];
        let mut budget = super::walk::Budget::new(self.system.limits.max_candidates);
        for family in families {
            let held = self.held(state, receiver, family.path());
            let mut per_slot = Vec::with_capacity(held);
            for slot in 0..held {
                let slot = u64::try_from(slot).map_err(|_| stop(BROKEN))?;
                let mut enabled = Vec::new();
                for part in family.parts() {
                    let elements: Vec<ElementBinding> = part
                        .elements()
                        .iter()
                        .map(|binding| at_slot(binding, slot))
                        .collect();
                    if !self.presences_hold(state, receiver, part.presences().iter().copied()) {
                        continue;
                    }
                    let data = match (part.data(), self.elements(state, receiver, &elements)) {
                        (None, _) => true,
                        (Some(_), None) => false,
                        (Some(data), Some(values)) => {
                            self.about_one(state, data, receiver, values, site)?
                        }
                    };
                    if data {
                        enabled.push(self.placed_at(state, receiver, part.clocks(), slot)?);
                    }
                }
                per_slot.push(enabled);
            }
            let options: Vec<Vec<Vec<PlaceConstraint>>> = if family.each() {
                per_slot
            } else {
                vec![per_slot.into_iter().flatten().collect()]
            };
            for option in options {
                budget
                    .charge_many(alternatives.len().saturating_mul(option.len()))
                    .map_err(TimedStop::Expansion)?;
                alternatives = alternatives
                    .iter()
                    .flat_map(|left| {
                        option
                            .iter()
                            .map(move |right| left.iter().chain(right).cloned().collect())
                    })
                    .collect();
            }
        }
        Ok(alternatives)
    }

    /// How many elements the collection of `object` at `path` holds at
    /// `state`.
    fn held(&self, state: &ModelSimState, object: &ObjectReference, path: &[String]) -> usize {
        self.collection(state, object, path)
            .map_or(0, |collection| collection.elements().len())
    }

    /// [`Self::placed`] with each family slot of a constraint read at `slot`.
    fn placed_at(
        &self,
        state: &ModelSimState,
        receiver: &ObjectReference,
        constraints: &[ClockConstraint],
        slot: u64,
    ) -> Result<Vec<PlaceConstraint>, ExpansionStop> {
        constraints
            .iter()
            .map(|constraint| {
                let slots: Vec<u64> = constraint
                    .slots()
                    .iter()
                    .map(|held| match held {
                        SlotRef::Family => slot,
                        SlotRef::Held(held) => *held,
                    })
                    .collect();
                let place = self
                    .holder(state, receiver, constraint.via())
                    .and_then(|holder| self.place_at(&holder, constraint.clock(), &slots))
                    .ok_or_else(|| stop(BROKEN))?;
                Ok(PlaceConstraint {
                    place,
                    op: constraint.op(),
                    bound: constraint.bound().clone(),
                })
            })
            .collect()
    }

    /// The element each of `elements` names of `object` at `state`: the
    /// value its collection holds at its slot; `None` when the collection
    /// does not hold one of the slots.
    fn elements(
        &self,
        state: &ModelSimState,
        object: &ObjectReference,
        elements: &[ElementBinding],
    ) -> Option<Vec<quire_exact::Value>> {
        elements
            .iter()
            .map(|element| {
                self.collection(state, object, element.path())?
                    .elements()
                    .get(match element.slot() {
                        SlotRef::Held(slot) => usize::try_from(slot).ok()?,
                        SlotRef::Family => return None,
                    })
                    .cloned()
            })
            .collect()
    }

    /// The collection `object` holds at `state` at the field path `path`:
    /// its field, or a collection below a record field of it.
    fn collection<'t>(
        &self,
        state: &'t ModelSimState,
        object: &ObjectReference,
        path: &[String],
    ) -> Option<&'t quire_exact::CollectionValue> {
        let (first, below) = path.split_first()?;
        let stored = self.stored(state, object)?;
        let fields = self.system.fields.get(&stored.type_identity)?;
        let field = fields.iter().find(|field| field.name == *first)?;
        let typing = self.system.typing();
        let mut value = stored.fields.get(&field.key)?;
        for name in below {
            value = typing.component(value, name)?;
        }
        match value {
            quire_exact::Value::Collection(collection) => Some(collection),
            _ => None,
        }
    }

    /// The stored object `object` names at `state`: held under its key in
    /// the population whose objects its universe names.
    fn stored<'t>(
        &self,
        state: &'t ModelSimState,
        object: &ObjectReference,
    ) -> Option<&'t qsl_semantics::model::state::StateObject> {
        let model = &self.system.model;
        state.state().populations.values().find_map(|objects| {
            let stored = objects.get(object.object().as_str())?;
            let reference = model
                .object_reference(&stored.type_identity, object.object().as_str())
                .ok()?;
            (reference.universe() == object.universe()).then_some(stored)
        })
    }

    /// The object `object` reaches at `state` through the reference fields
    /// `via`, each a required single reference: `object` itself for none.
    fn holder(
        &self,
        state: &ModelSimState,
        object: &ObjectReference,
        via: &[PlaceStep],
    ) -> Option<ObjectReference> {
        let model = &self.system.model;
        let mut at = object.clone();
        for step in via {
            let PlaceStep::Field(name) = step else {
                return None;
            };
            let stored = self.stored(state, &at)?;
            let fields = model.fields(&stored.type_identity).ok()??;
            let field = fields.iter().find(|field| field.name == *name)?;
            match stored.fields.get(&field.key)? {
                quire_exact::Value::Reference(next) => at = next.clone(),
                _ => return None,
            }
        }
        Some(at)
    }

    /// `predicate` at `state` with `self` bound to `object` and the slots
    /// after it to `elements`.
    fn about_one(
        &self,
        state: &ModelSimState,
        predicate: &CheckedTimedPredicate,
        object: &ObjectReference,
        elements: Vec<quire_exact::Value>,
        site: impl Fn() -> TimedSite,
    ) -> Result<bool, TimedStop> {
        let [reading] = &self.system.state_predicates_with(
            state,
            &[(predicate, Some(object), elements)],
            self.place_presences(state),
        )?[..] else {
            return Err(TimedStop::undecided(site(), object.clone(), None));
        };
        match reading {
            PredicateReading::True => Ok(true),
            PredicateReading::False => Ok(false),
            PredicateReading::Undefined { cause, .. } => Err(TimedStop::undecided(
                site(),
                object.clone(),
                Some(cause.clone()),
            )),
            PredicateReading::Refused(_) => Err(TimedStop::undecided(site(), object.clone(), None)),
        }
    }

    /// The state a delay by `delay` reaches from `state`, or `None` when the
    /// delay is not admissible there (FR-231 "Delay"). Under a tick source
    /// only a delay of 0 is admissible.
    ///
    /// # Errors
    ///
    /// As [`Self::invariants`] and [`Self::is_urgent`].
    pub fn delay(
        &self,
        state: &TimedState,
        delay: &Rational,
    ) -> Result<Option<TimedState>, TimedStop> {
        if delay.numerator().is_negative() {
            return Ok(None);
        }
        if delay.is_zero() {
            return Ok(Some(state.clone()));
        }
        if self.tick.is_some() || self.is_urgent(&state.discrete)? {
            return Ok(None);
        }
        let clocks = state.clocks.delayed(delay);
        let admitted = self
            .invariants(&state.discrete)?
            .iter()
            .all(|bound| bound.holds(&clocks));
        Ok(admitted.then(|| TimedState {
            discrete: state.discrete.clone(),
            clocks,
        }))
    }

    /// Every discrete step from `state` (FR-231 "Discrete step"), in
    /// canonical transition order: its transition identity and the timed
    /// state it reaches.
    ///
    /// # Errors
    ///
    /// As [`Self::guarded_steps`] and [`Self::invariants`].
    pub fn steps(&self, state: &TimedState) -> Result<Vec<(GuardedStep, TimedState)>, TimedStop> {
        let mut found = Vec::new();
        for step in self.guarded_steps(&state.discrete)? {
            if !step
                .parts
                .iter()
                .any(|part| part.iter().all(|c| c.holds(&state.clocks)))
            {
                continue;
            }
            let clocks = step.reset(&state.clocks);
            if !self
                .invariants(&step.post)?
                .iter()
                .all(|bound| bound.holds(&clocks))
            {
                continue;
            }
            let reached = TimedState {
                discrete: step.post.clone(),
                clocks,
            };
            found.push((step, reached));
        }
        Ok(found)
    }

    /// FR-236's post-state digest of `state`: the canonical identity of its
    /// model state (FR-181's `simulation-state`) together with its exact
    /// clock valuation, each clock named by its object, the field it is
    /// under and its path below the object (FR-230 "Places"). A clock of
    /// an object the state does not hold has no value and is left out
    /// (STD-493's byte form).
    ///
    /// # Errors
    ///
    /// [`crate::simulation::EncodingRefusal`] when the state has no
    /// canonical encoding within the identity limits.
    pub fn state_digest(
        &self,
        state: &TimedState,
    ) -> Result<qsl_foundation::digest::DigestRecord, crate::simulation::EncodingRefusal> {
        let clocks = self
            .places
            .iter()
            .zip(state.clocks.values())
            .filter(|(place, _)| self.live(place, &state.discrete))
            .map(|(place, value)| {
                let (field, path) = self.model.clock(place.clock).map_or(
                    (super::encode::Enc::Null, super::encode::Enc::Null),
                    |field| {
                        let mut slots = place.slots.iter();
                        (
                            super::encode::declaration_enc(field.field()),
                            super::encode::Enc::Array(
                                field
                                    .path()
                                    .iter()
                                    .map(|step| place_step_enc(step, &mut slots))
                                    .collect(),
                            ),
                        )
                    },
                );
                super::encode::Enc::tagged(
                    "clock",
                    vec![
                        ("object", super::encode::reference_enc(&place.object)),
                        ("field", field),
                        ("path", path),
                        (
                            "value",
                            super::encode::value_enc(
                                &quire_exact::Value::Rational(value.clone()),
                                self.system.typing(),
                            ),
                        ),
                    ],
                )
            })
            .collect();
        let enc = super::encode::Enc::tagged(
            "timed-state",
            vec![
                (
                    "state",
                    super::encode::model_key(
                        &self.system.model,
                        state.discrete.state(),
                        self.system.typing(),
                    )
                    .0,
                ),
                ("clocks", super::encode::Enc::Array(clocks)),
            ],
        );
        crate::simulation::key::state_key(&enc, super::identity_limits()).map(|(_, digest)| digest)
    }

    /// The time a step of `operation` advances a tick source's time stamp:
    /// its period when `operation` is the tick operation, 0 otherwise and
    /// under a dense source.
    pub fn tick_of(&self, operation: &str) -> Rational {
        match &self.tick {
            Some((tick, period)) if tick == operation => period.clone(),
            _ => zero(),
        }
    }

    /// A tick source's period, the stamp its terminal stutter step adds
    /// (FR-231 "Digital time"); `None` under a dense source.
    pub fn tick_period(&self) -> Option<&Rational> {
        self.tick.as_ref().map(|(_, period)| period)
    }
}

/// One position of a timed behaviour (FR-231 "Behaviours, positions and
/// time stamps").
#[derive(Clone, Debug)]
pub struct TimedPosition {
    state: TimedState,
    stamp: Rational,
    step: Option<GuardedStep>,
}

impl TimedPosition {
    /// The timed state.
    pub fn state(&self) -> &TimedState {
        &self.state
    }

    /// The time stamp: the sum of the delays before steps 1 to `i`, or the
    /// tick count times the period under a tick source.
    pub fn stamp(&self) -> &Rational {
        &self.stamp
    }

    /// The step that reached the position; `None` at position 0.
    pub fn step(&self) -> Option<&GuardedStep> {
        self.step.as_ref()
    }
}

/// Why a timed behaviour could not take a move.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MoveRefusal {
    /// The delay is not admissible at the last position.
    Delay,
    /// No enabled step after the delay has the transition identity.
    Step,
    /// The move could not be decided.
    Stopped(TimedStop),
    /// The behaviour ends at a state that is not quiescent, so it has no
    /// idle tail and no divergent reading (FR-231).
    NotDivergent,
}

impl From<TimedStop> for MoveRefusal {
    fn from(stop: TimedStop) -> Self {
        Self::Stopped(stop)
    }
}

/// A finite timed behaviour from an initial state: its positions.
#[derive(Clone, Debug)]
pub struct TimedBehaviour {
    positions: Vec<TimedPosition>,
}

impl TimedBehaviour {
    /// The behaviour of the one position `initial`, at time 0.
    pub fn new(initial: TimedState) -> Self {
        Self {
            positions: vec![TimedPosition {
                state: initial,
                stamp: zero(),
                step: None,
            }],
        }
    }

    /// The positions.
    pub fn positions(&self) -> &[TimedPosition] {
        &self.positions
    }

    /// The last position.
    pub fn last(&self) -> &TimedPosition {
        self.positions
            .last()
            .expect("a behaviour always holds its initial position")
    }

    /// Delay by `delay`, then take the step `transition`.
    ///
    /// # Errors
    ///
    /// [`MoveRefusal`] when the delay is not admissible, no step of that
    /// identity is enabled after it, or the move cannot be decided.
    pub fn advance(
        &mut self,
        system: &TimedSystem<'_, '_>,
        delay: &Rational,
        transition: &ModelTransition,
    ) -> Result<(), MoveRefusal> {
        self.advance_where(system, delay, |step, _| &step.transition == transition)
    }

    /// Delay by `delay`, then take the first enabled step that `accept`
    /// accepts with the timed state it reaches: where one transition
    /// identity has several successors, the one a replay recorded.
    ///
    /// # Errors
    ///
    /// As [`Self::advance`].
    pub fn advance_where(
        &mut self,
        system: &TimedSystem<'_, '_>,
        delay: &Rational,
        accept: impl Fn(&GuardedStep, &TimedState) -> bool,
    ) -> Result<(), MoveRefusal> {
        let last = self.last();
        let delayed = system
            .delay(&last.state, delay)?
            .ok_or(MoveRefusal::Delay)?;
        let (step, state) = system
            .steps(&delayed)?
            .into_iter()
            .find(|(step, state)| accept(step, state))
            .ok_or(MoveRefusal::Step)?;
        let stamp = last.stamp.add(delay).add(&system.tick_of(&step.operation));
        self.positions.push(TimedPosition {
            state,
            stamp,
            step: Some(step),
        });
        Ok(())
    }
}

fn zero() -> Rational {
    Rational::from_integer(Integer::zero())
}

/// Whether `cause` is the absent-key undefinedness of an object the state
/// does not hold.
fn absent(cause: &UndefinedCause) -> bool {
    matches!(cause, UndefinedCause::Family(record) if record.reason == UndefinedReason::AbsentKey)
}

impl TimedBehaviour {
    /// The behaviour's time stamps as a timed trace (FR-231 "Quiescent states
    /// and the idle tail"). With `idle_tail`, a behaviour whose last state is
    /// quiescent is read under infinite-trace and the timed profile as that
    /// state's stutter step repeated forever, each after a positive delay:
    /// the lasso that loops on its last position with a delay of one unit,
    /// or of the tick period under a tick source, whose stutter step counts
    /// as a tick. A behaviour that ends anywhere else has no such reading
    /// and is not admitted. Without `idle_tail`, as under a bounded profile,
    /// the trace closes at its last position.
    ///
    /// # Errors
    ///
    /// [`MoveRefusal::Stopped`] when quiescence cannot be decided;
    /// [`MoveRefusal::Delay`] when the stamps do not form a timed trace;
    /// [`MoveRefusal::NotDivergent`] with `idle_tail` when the last state is
    /// not quiescent.
    pub fn timed_trace(
        &self,
        system: &TimedSystem<'_, '_>,
        idle_tail: bool,
    ) -> Result<crate::temporal::TimedTrace, MoveRefusal> {
        let stamps: Vec<Rational> = self
            .positions
            .iter()
            .map(|position| position.stamp.clone())
            .collect();
        let quiescent = idle_tail && system.is_quiescent(&self.last().state.discrete)?;
        if idle_tail && !quiescent {
            // FR-231 admits a behaviour only when its time stamps diverge;
            // one that ends where time is bounded is no admitted behaviour.
            return Err(MoveRefusal::NotDivergent);
        }
        let trace = if quiescent {
            let last = u64::try_from(self.positions.len().saturating_sub(1)).unwrap_or(u64::MAX);
            let delay = system
                .tick_period()
                .cloned()
                .unwrap_or_else(|| Rational::from_integer(Integer::one()));
            crate::temporal::TimedTrace::lasso(stamps, last, delay)
        } else {
            crate::temporal::TimedTrace::new(stamps)
        };
        trace.map_err(|_| MoveRefusal::Delay)
    }
}

/// How a temporal clause reads a timed behaviour (FR-231 "Quiescent states
/// and the idle tail", "Untimed reading"): a position trace for an untimed
/// claim, or a time-stamped trace for a claim under the timed profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BehaviourReading {
    /// The positions an ADR-018 claim reads, with the clock valuation part
    /// of each position's state.
    Untimed(crate::temporal::TemporalTrace),
    /// The time-stamped positions a claim under the timed profile reads.
    Timed(crate::temporal::TimedTrace),
}

impl TimedBehaviour {
    /// The trace a temporal clause checked under `profile` reads this
    /// behaviour as (FR-231). Under a bounded profile (event-position or
    /// fixed-sample false extension, or a finite window) the idle tail
    /// closes the execution at its last discrete step: the closed trace of
    /// its positions. Under infinite-trace the quiescent last state's
    /// stutter step repeats forever: the lasso looping on the last position.
    /// Under the timed profile, the timed lasso of [`Self::timed_trace`].
    ///
    /// # Errors
    ///
    /// As [`Self::timed_trace`] with the idle tail, for infinite-trace and
    /// the timed profile.
    pub fn clause_reading(
        &self,
        system: &TimedSystem<'_, '_>,
        profile: qsl_semantics::check::TemporalProfile,
    ) -> Result<BehaviourReading, MoveRefusal> {
        let len = u64::try_from(self.positions.len()).unwrap_or(u64::MAX);
        if profile.is_bounded() {
            return Ok(BehaviourReading::Untimed(
                crate::temporal::TemporalTrace::Finite { len },
            ));
        }
        let timed = self.timed_trace(system, true)?;
        if profile == qsl_semantics::check::TemporalProfile::Timed {
            return Ok(BehaviourReading::Timed(timed));
        }
        Ok(BehaviourReading::Untimed(
            crate::temporal::TemporalTrace::Lasso {
                prefix: len.saturating_sub(1),
                loop_len: 1,
            },
        ))
    }
}

/// The letters of one checked temporal clause over a timed behaviour: a
/// position reads its model state, and a position past the represented
/// behaviour the state it wraps to on the idle tail's loop.
pub struct BehaviourLetters<'b, L> {
    letters: &'b L,
    instance: &'b crate::model_subject::ModelInstance,
    behaviour: &'b TimedBehaviour,
}

impl<'b, L> BehaviourLetters<'b, L> {
    /// The letters `letters` reads for `instance`, over `behaviour`.
    pub fn new(
        letters: &'b L,
        instance: &'b crate::model_subject::ModelInstance,
        behaviour: &'b TimedBehaviour,
    ) -> Self {
        Self {
            letters,
            instance,
            behaviour,
        }
    }

    fn state_at(&self, position: crate::temporal::TemporalPosition) -> &ModelSimState {
        let positions = &self.behaviour.positions;
        let index = usize::try_from(position.index()).unwrap_or(usize::MAX);
        &positions
            .get(index)
            .unwrap_or_else(|| self.behaviour.last())
            .state
            .discrete
    }
}

impl<L: crate::model_subject::ModelLetters> crate::temporal::TemporalLetters
    for BehaviourLetters<'_, L>
{
    fn atom(
        &mut self,
        atom: qsl_semantics::check::FormulaNodeId,
        position: crate::temporal::TemporalPosition,
        meter: &mut quire_exact::Meter,
    ) -> Result<crate::temporal::AtomReading, crate::temporal::LetterStop> {
        Ok(self
            .letters
            .atom(atom, self.instance, self.state_at(position), meter)?)
    }

    fn activation(
        &mut self,
        position: crate::temporal::TemporalPosition,
        meter: &mut quire_exact::Meter,
    ) -> Result<crate::temporal::AtomReading, crate::temporal::LetterStop> {
        Ok(self
            .letters
            .activation(self.instance, self.state_at(position), meter)?)
    }

    fn captures(
        &mut self,
        position: crate::temporal::TemporalPosition,
        meter: &mut quire_exact::Meter,
    ) -> Result<crate::temporal::CaptureReading, crate::temporal::LetterStop> {
        Ok(self
            .letters
            .captures(self.instance, self.state_at(position), meter)?)
    }
}

/// Whether `value` is a union value of the case named `case`.
fn is_case(value: &quire_exact::Value, case: &str) -> bool {
    matches!(value, quire_exact::Value::Union(union) if union.member().identifier().as_str() == case)
}

/// One step of a clock place's path in the state digest's byte form: a
/// field by name, a union case by name, a slot by its index, taken from
/// `slots` in path order.
fn place_step_enc<'s>(
    step: &PlaceStep,
    slots: &mut impl Iterator<Item = &'s usize>,
) -> super::encode::Enc {
    match step {
        PlaceStep::Field(name) => super::encode::Enc::tagged(
            "field",
            vec![("name", super::encode::Enc::text(name.as_str()))],
        ),
        PlaceStep::Union(case) => super::encode::Enc::tagged(
            "union",
            vec![("case", super::encode::Enc::text(case.as_str()))],
        ),
        PlaceStep::Slot(_) => super::encode::Enc::tagged(
            "slot",
            vec![(
                "index",
                super::encode::Enc::text(
                    slots.next().map_or_else(String::new, ToString::to_string),
                ),
            )],
        ),
    }
}

/// Every vector of slot indices below `maxima`, one index per collection,
/// in ascending lexicographic order; one empty vector for none.
fn slot_vectors(maxima: &[usize]) -> Vec<Vec<usize>> {
    maxima.iter().fold(vec![Vec::new()], |vectors, max| {
        vectors
            .iter()
            .flat_map(|vector| {
                (0..*max).map(move |slot| {
                    let mut next = vector.clone();
                    next.push(slot);
                    next
                })
            })
            .collect()
    })
}

/// `binding` read at `slot` when it names a family's template slot.
fn at_slot(binding: &ElementBinding, slot: u64) -> ElementBinding {
    if binding.slot() == SlotRef::Family {
        ElementBinding::new(
            binding.name().to_owned(),
            binding.path().to_vec(),
            SlotRef::Held(slot),
        )
    } else {
        binding.clone()
    }
}

/// Every way to distribute `total` over `parts` counts, in lexicographic
/// order.
fn distributions(total: u64, parts: usize) -> Vec<Vec<u64>> {
    let Some(rest) = parts.checked_sub(1) else {
        return if total == 0 {
            vec![Vec::new()]
        } else {
            Vec::new()
        };
    };
    let mut all = Vec::new();
    for first in 0..=total {
        for mut tail in distributions(total - first, rest) {
            tail.insert(0, first);
            all.push(tail);
        }
    }
    all
}
