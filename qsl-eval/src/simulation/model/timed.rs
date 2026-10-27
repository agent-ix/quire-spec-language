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
    PlaceStep, TimeSourceKind, TimedModel, Urgency,
};

use super::encode::declaration_text;
use super::state_predicate::PredicateReading;
use super::walk::{expand_enabled, stop};
use super::BROKEN;
use super::{ModelSimState, ModelSystem};
use crate::simulation::expansion::ExpansionStop;
use crate::temporal::UndefinedCause;

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
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlaceConstraint {
    place: usize,
    op: ClockOp,
    bound: Rational,
}

impl PlaceConstraint {
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

    /// The resets, each a place and its new value, in source order.
    pub fn resets(&self) -> &[(usize, Rational)] {
        &self.resets
    }

    /// `clocks` with the resets applied.
    pub fn reset(&self, clocks: &ClockValuation) -> ClockValuation {
        let mut values = clocks.0.clone();
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
    /// A clock owner's population has no universe, so its clocks cannot be
    /// listed.
    #[error("the clocks of `{clock}` have no universe to range over")]
    Unbounded {
        /// The clock field's name.
        clock: String,
    },
    /// A clock lies in a collection that declares no maximum, so its slots
    /// cannot be listed (FR-230 "Clock collections": `max: None`).
    #[error("the clock `{clock}` lies in a collection with no declared maximum")]
    UnboundedCollection {
        /// The clock place's name.
        clock: String,
    },
}

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
        let mut places = Vec::new();
        for (clock, field) in timed.clock_fields() {
            let maxima = field
                .slots()
                .map(|shape| shape.max.and_then(|max| usize::try_from(max).ok()))
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| TimedSubjectRefusal::UnboundedCollection {
                    clock: field.name().to_owned(),
                })?;
            let slots = slot_vectors(&maxima);
            for (instance, object) in instances_of(field.owner(), field.name())? {
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
        if place.slots.is_empty() {
            return true;
        }
        let Some(field) = self.model.clock(place.clock) else {
            return false;
        };
        let typing = self.system.typing();
        let mut slots = place.slots.iter();
        // The steps through the innermost collection: the clock below it is
        // no data component to read.
        let through = field
            .path()
            .iter()
            .rposition(|step| matches!(step, PlaceStep::Slot(_)))
            .map_or(0, |last| last + 1);
        let mut steps = field.path()[..through].iter();
        let Some(PlaceStep::Field(first)) = steps.next() else {
            return false;
        };
        let Ok(Some(fields)) = self.system.model.fields(&stored.type_identity) else {
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
                PlaceStep::Slot(_) => match (value, slots.next()) {
                    (quire_exact::Value::Collection(collection), Some(slot)) => {
                        collection.elements().get(*slot)
                    }
                    _ => None,
                },
            };
            match next {
                Some(next) => value = next,
                None => return false,
            }
        }
        true
    }

    /// The initial timed states: each initial model state with every clock 0.
    pub fn initial(&self) -> Vec<TimedState> {
        // A slot the initial state does not hold is not live; its value is
        // reset when the slot becomes live.
        let zero = ClockValuation(vec![zero(); self.places.len()]);
        self.system
            .initial
            .iter()
            .map(|discrete| TimedState {
                discrete: discrete.clone(),
                clocks: zero.clone(),
            })
            .collect()
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
        let readings = self.system.state_predicates(state, &bound)?;
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
            let readings = self.about(state, invariant.predicate(), invariant.context(), || {
                TimedSite::Invariant {
                    name: invariant.name().to_owned(),
                }
            })?;
            for (object, holds) in readings {
                if holds {
                    bounds.extend(self.over(&object, invariant.bounds()));
                }
            }
        }
        Ok(bounds)
    }

    /// `constraints` over `object`'s places.
    fn over<'c>(
        &'c self,
        object: &'c ObjectReference,
        constraints: &'c [ClockConstraint],
    ) -> impl Iterator<Item = PlaceConstraint> + 'c {
        constraints.iter().filter_map(move |constraint| {
            Some(PlaceConstraint {
                place: self.place(object, constraint.clock())?,
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
                predicate,
            } = urgency
            {
                let readings = self.about(state, predicate, *context, || TimedSite::Urgency {
                    name: name.clone(),
                })?;
                if readings.iter().any(|(_, holds)| *holds) {
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
        let (expansion, enabled) = expand_enabled(self.system, state)?;
        let mut steps = Vec::new();
        for vector in enabled {
            for position in vector.successors.clone() {
                let Some((transition, post)) = expansion.successors.get(position) else {
                    continue;
                };
                let mut parts = vec![Vec::new()];
                for guard in self.model.guards() {
                    let operation = guard.operation();
                    if operation.declaring != vector.declaring
                        || operation.declaration.name() != vector.name
                    {
                        continue;
                    }
                    let mut clause_parts = Vec::new();
                    for part in guard.parts() {
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
                            clause_parts.push(self.placed(
                                state,
                                &vector.receiver,
                                part.clocks(),
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
                let mut resets: Vec<(usize, Rational)> = Vec::new();
                for (clause, reset) in self
                    .model
                    .resets()
                    .iter()
                    .filter(|resets| {
                        resets.operation().declaring == vector.declaring
                            && resets.operation().declaration.name() == vector.name
                    })
                    .flat_map(|resets| resets.resets().iter().map(|reset| (resets.clause(), reset)))
                {
                    let Some(selection) = reset.selection() else {
                        if let Some(place) = self.place(&vector.receiver, reset.clock()) {
                            resets.push((place, reset.value().clone()));
                        }
                        continue;
                    };
                    // Each slot the pre-state holds whose element the
                    // selection's predicate admits.
                    for slot in 0_u64.. {
                        let Some(place) = self.place_at(&vector.receiver, reset.clock(), &[slot])
                        else {
                            break;
                        };
                        let binding = ElementBinding::new(
                            selection.name().to_owned(),
                            selection.field().to_owned(),
                            slot,
                        );
                        let Some(elements) =
                            self.elements(state, &vector.receiver, std::slice::from_ref(&binding))
                        else {
                            continue;
                        };
                        let selected = match selection.predicate() {
                            None => true,
                            Some(predicate) => self.about_one(
                                state,
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
                // A clock whose object or slot the step makes live takes
                // the constant of its write, 0 when the step writes none
                // (QSpec FR-181 "Timed subjects").
                for (place, clock) in self.places.iter().enumerate() {
                    if !self.live(clock, state)
                        && self.live(clock, post)
                        && resets.iter().all(|(reset, _)| *reset != place)
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
                });
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
                        self.place_at(&holder, constraint.clock(), constraint.slots())
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

    /// The element each of `elements` names of `object` at `state`: the
    /// value its collection field holds at its slot; `None` when the
    /// collection does not hold one of the slots.
    fn elements(
        &self,
        state: &ModelSimState,
        object: &ObjectReference,
        elements: &[ElementBinding],
    ) -> Option<Vec<quire_exact::Value>> {
        if elements.is_empty() {
            return Some(Vec::new());
        }
        let model = &self.system.model;
        let stored = self.stored(state, object)?;
        let fields = model.fields(&stored.type_identity).ok()??;
        elements
            .iter()
            .map(|element| {
                let field = fields.iter().find(|field| field.name == element.field())?;
                let quire_exact::Value::Collection(collection) = stored.fields.get(&field.key)?
                else {
                    return None;
                };
                collection
                    .elements()
                    .get(usize::try_from(element.slot()).ok()?)
                    .cloned()
            })
            .collect()
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
        let [reading] = &self
            .system
            .state_predicates_with(state, &[(predicate, Some(object), elements)])?[..]
        else {
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
        let last = self.last();
        let delayed = system
            .delay(&last.state, delay)?
            .ok_or(MoveRefusal::Delay)?;
        let (step, state) = system
            .steps(&delayed)?
            .into_iter()
            .find(|(step, _)| &step.transition == transition)
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

/// One step of a clock place's path in the state digest's byte form: a
/// field by name, a slot by its index, taken from `slots` in path order.
fn place_step_enc<'s>(
    step: &PlaceStep,
    slots: &mut impl Iterator<Item = &'s usize>,
) -> super::encode::Enc {
    match step {
        PlaceStep::Field(name) => super::encode::Enc::tagged(
            "field",
            vec![("name", super::encode::Enc::text(name.as_str()))],
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
