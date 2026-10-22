// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-155 "Instantiation" and "Read enforcement": the footprint of a model
//! transition, and the check that the `pre` and `post` clauses of a step read
//! nothing outside it.
//!
//! The symbolic footprint of an operation is derived once, from its checked
//! clauses and frame, by `qsl_semantics::check::footprint`. Instantiating it
//! replaces the receiver and each reference parameter with the object the
//! transition binds. Enforcement runs a step's clauses with a read log
//! attached to their observations and compares the log with the footprint.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, PoisonError};

use qsl_semantics::check::footprint::{
    derive_atom_reads, derive_constraint_reads, derive_footprint, derive_selected_footprint,
    derive_slot_reads, FootprintObject, FootprintRefusal, SymbolicFootprint, SymbolicLocation,
};
use qsl_semantics::check::CheckedNode;
use qsl_semantics::model::clause_observation::Observation;
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::object_environment::{ObservedRead, ReadLog};
use qsl_semantics::model::observation::AdmittedObservations;
use qsl_semantics::model::state::StateModel;
use quire_exact::{EffectiveId, NodeKey, ObjectReference, Value, ValueType};
use quire_semantic_value::model_transition::ModelTransition;

use crate::model_subject::ModelInstance;

use super::encode::{parse_optional_reference, parse_reference};
use super::walk::stop;
use super::{Application, ModelSimState, ModelSystem, BROKEN};
use crate::simulation::expansion::{Expansion, ExpansionStop};
use crate::simulation::footprint::{Footprint, Location};

/// Why a transition has no footprint.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum FootprintError {
    /// The operation's clauses read through a form QSpec FR-383 gives no
    /// footprint rule.
    #[error(transparent)]
    Refused(#[from] FootprintRefusal),
    /// The transition names no operation of its receiver's type, or a
    /// receiver or reference argument that is no reference text.
    #[error("the transition names no operation of its receiver's type")]
    UnknownTransition,
    /// The package declares no state clause of the name.
    #[error("the package declares no state constraint of that name")]
    UnknownConstraint,
}

/// A step read a location outside its footprint (FR-155 "Read
/// enforcement"): an internal failure of the model checker, never a value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FootprintViolation {
    /// The transition whose clause read the location.
    pub transition: ModelTransition,
    /// The location read.
    pub location: Location,
    /// The instant the read observed.
    pub observation: Observation,
}

impl FootprintViolation {
    /// The catalog code the expansion stops with.
    pub fn cause(&self) -> qsl_foundation::CatalogCode {
        BROKEN
    }
}

/// Why [`ModelSystem::successors_within`] returned no expansion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WithinStop {
    /// The expansion stopped as `successors` stops it.
    Expansion(ExpansionStop),
    /// A transition has no footprint to enforce.
    Footprint(FootprintError),
    /// A step read outside its footprint.
    Violation(Box<FootprintViolation>),
}

/// What an enforcing expansion found wrong, kept until the expansion's
/// error unwinds to [`ModelSystem::successors_within`].
pub(super) enum Fault {
    Footprint(FootprintError),
    Violation(Box<FootprintViolation>),
}

type CacheKey = (
    EffectiveId,
    String,
    Vec<(String, EffectiveId)>,
    Option<Arc<BTreeSet<NodeKey>>>,
);

/// Symbolic footprints already derived, by the receiver's type, the operation,
/// the types of the bound reference arguments and the clauses selected, all
/// when `None`. A selection is shared, so a lookup clones no set.
#[derive(Default)]
pub(super) struct FootprintCache(
    Mutex<BTreeMap<CacheKey, Result<SymbolicFootprint, FootprintRefusal>>>,
);

/// The footprints an enforcing expansion checks each step against.
pub(super) struct Enforcement<'e> {
    withhold: &'e dyn Fn(&ModelTransition) -> BTreeSet<Location>,
    fault: RefCell<Option<Fault>>,
}

impl Enforcement<'_> {
    /// The footprint `transition` is enforced against: the derived footprint
    /// of `application`, instantiated, less the locations the caller
    /// withholds.
    pub(super) fn footprint_of(
        &self,
        system: &ModelSystem<'_>,
        application: &Application,
        receiver: &ObjectReference,
        arguments: &[(String, Value)],
        transition: &ModelTransition,
    ) -> Result<Footprint, ExpansionStop> {
        let bound: Vec<(String, Option<ObjectReference>)> = arguments
            .iter()
            .map(|(name, value)| (name.clone(), referenced(value).cloned()))
            .collect();
        match system.footprint_for(application, receiver, &bound, None) {
            Ok(mut footprint) => {
                let withheld = (self.withhold)(transition);
                for locations in [
                    &mut footprint.reads,
                    &mut footprint.writes,
                    &mut footprint.enabling,
                ] {
                    locations.retain(|location| !withheld.contains(location));
                }
                Ok(footprint)
            }
            Err(refusal) => {
                self.fail(Fault::Footprint(FootprintError::Refused(refusal)));
                Err(stop(BROKEN))
            }
        }
    }

    fn fail(&self, fault: Fault) {
        self.fault.borrow_mut().get_or_insert(fault);
    }
}

/// The read log of one clause-set evaluation, and the footprint it is held
/// to.
pub(super) struct ReadCheck<'c, 'e> {
    model: &'c StateModel,
    enforcement: &'c Enforcement<'e>,
    footprint: &'c Footprint,
    transition: &'c ModelTransition,
    log: ReadLog,
}

impl<'c, 'e> ReadCheck<'c, 'e> {
    pub(super) fn new(
        model: &'c StateModel,
        enforcement: &'c Enforcement<'e>,
        footprint: &'c Footprint,
        transition: &'c ModelTransition,
    ) -> Self {
        Self {
            model,
            enforcement,
            footprint,
            transition,
            log: ReadLog::new(),
        }
    }

    /// `observations` with every environment logging into this check.
    pub(super) fn attach(&self, mut observations: AdmittedObservations) -> AdmittedObservations {
        for observation in [
            &mut observations.current,
            &mut observations.pre,
            &mut observations.post,
        ]
        .into_iter()
        .flatten()
        {
            observation.environment =
                std::mem::take(&mut observation.environment).with_read_log(self.log.clone());
        }
        observations
    }

    /// Hold the logged reads to the footprint.
    ///
    /// # Errors
    ///
    /// `runtime_invariant`/`established-invariant-broken` for the first read
    /// outside the footprint; the violation is kept for
    /// [`ModelSystem::successors_within`].
    pub(super) fn verify(&self) -> Result<(), ExpansionStop> {
        for read in self.log.reads() {
            let (observation, location) = self.location_of(read)?;
            let admitted = match observation {
                Observation::Pre => self.admits(&self.footprint.reads, &location),
                Observation::Current | Observation::Post => {
                    self.admits(&self.footprint.reads, &location)
                        || self.admits(&self.footprint.writes, &location)
                }
            };
            if !admitted {
                self.enforcement
                    .fail(Fault::Violation(Box::new(FootprintViolation {
                        transition: self.transition.clone(),
                        location,
                        observation,
                    })));
                return Err(stop(BROKEN));
            }
        }
        Ok(())
    }

    fn location_of(&self, read: ObservedRead) -> Result<(Observation, Location), ExpansionStop> {
        match read {
            ObservedRead::Field {
                observation,
                object,
                owner,
                name,
            } => {
                let owner = self.model.type_key(owner).ok_or_else(|| stop(BROKEN))?;
                Ok((
                    observation,
                    Location::Field {
                        object,
                        field: DeclarationKey {
                            package: owner.package.clone(),
                            node: format!("{}/{name}", owner.node),
                        },
                    },
                ))
            }
            ObservedRead::Membership {
                observation,
                population,
                object: Some(object),
            } => Ok((observation, Location::Membership { population, object })),
            ObservedRead::Membership {
                observation,
                population,
                object: None,
            } => Ok((observation, Location::AnyMembership { population })),
        }
    }

    /// Whether some location of `allowed` covers the whole of `read`.
    fn admits(&self, allowed: &BTreeSet<Location>, read: &Location) -> bool {
        allowed.iter().any(|location| match (location, read) {
            (Location::AnyField { field }, Location::Field { field: read, .. })
            | (Location::AnyField { field }, Location::AnyField { field: read }) => field == read,
            (
                Location::AnyMembership { population },
                Location::Membership {
                    population: read, ..
                }
                | Location::AnyMembership { population: read },
            ) => population == read,
            _ => location == read,
        })
    }
}

/// The object an argument value refers to: the reference itself, or the
/// payload of a present optional reference.
fn referenced(value: &Value) -> Option<&ObjectReference> {
    match value {
        Value::Reference(reference) => Some(reference),
        Value::Option(option) => match option.payload() {
            Some(Value::Reference(reference)) => Some(reference),
            _ => None,
        },
        _ => None,
    }
}

/// `symbolic` with the receiver and each reference parameter replaced by the
/// object the transition binds (FR-155 "Instantiation"). A location about a
/// parameter whose argument is absent is dropped.
fn instantiate(
    symbolic: &SymbolicFootprint,
    receiver: &ObjectReference,
    arguments: &[(String, Option<ObjectReference>)],
) -> Footprint {
    let bind = |object: &FootprintObject| match object {
        FootprintObject::Receiver => Some(receiver.clone()),
        FootprintObject::Param(name) => arguments
            .iter()
            .find(|(parameter, _)| parameter == name)
            .and_then(|(_, bound)| bound.clone()),
    };
    let locations = |symbolic: &BTreeSet<SymbolicLocation>| -> BTreeSet<Location> {
        symbolic
            .iter()
            .filter_map(|location| lower(location, &bind))
            .collect()
    };
    Footprint {
        reads: locations(&symbolic.reads),
        writes: locations(&symbolic.writes),
        enabling: locations(&symbolic.enabling),
    }
}

/// `location` with the object it names replaced by `bind`'s object; `None`
/// when a location about one object has no object to name.
fn lower(
    location: &SymbolicLocation,
    bind: &dyn Fn(&FootprintObject) -> Option<ObjectReference>,
) -> Option<Location> {
    match location {
        SymbolicLocation::Field { object, field } => bind(object).map(|object| Location::Field {
            object,
            field: field.clone(),
        }),
        SymbolicLocation::AnyField { field } => Some(Location::AnyField {
            field: field.clone(),
        }),
        SymbolicLocation::Membership { population, object } => {
            bind(object).map(|object| Location::Membership {
                population: population.clone(),
                object,
            })
        }
        SymbolicLocation::AnyMembership { population } => Some(Location::AnyMembership {
            population: population.clone(),
        }),
    }
}

/// The object type a reference or optional-reference parameter binds.
fn declared_target(value_type: &ValueType) -> Option<EffectiveId> {
    match value_type {
        ValueType::Reference(target) => Some(*target),
        ValueType::Option(payload) => declared_target(payload),
        _ => None,
    }
}

/// Whether a value of `value_type` can hold an object reference, through
/// options and collections; a composite or population value is taken to.
fn holds_reference(value_type: &ValueType) -> bool {
    match value_type {
        ValueType::Reference(_) => true,
        ValueType::Option(payload) => holds_reference(payload),
        ValueType::Collection(collection) => holds_reference(collection.element()),
        // A composite's members are not followed; taken to hold one.
        ValueType::Composite(_) | ValueType::Population(_) => true,
        ValueType::Boolean
        | ValueType::Int(_)
        | ValueType::Integer
        | ValueType::Rational(_)
        | ValueType::Decimal(_)
        | ValueType::Float(_)
        | ValueType::Quantity(_)
        | ValueType::Text(_)
        | ValueType::String
        | ValueType::Uuid
        | ValueType::Timestamp
        | ValueType::Enum(_) => false,
    }
}

/// Whether `value_type` holds an object reference a footprint can bind.
fn binds_object(value_type: &ValueType) -> bool {
    match value_type {
        ValueType::Reference(_) => true,
        ValueType::Option(payload) => binds_object(payload),
        _ => false,
    }
}

impl<'a> ModelSystem<'a> {
    /// The footprint of `application` on `receiver` with the reference
    /// arguments `arguments`, derived from the object types the identities
    /// carry (QSpec FR-383) and instantiated.
    /// With `selected`, only the clauses it names are read (ADR-027 FT-1);
    /// with `None`, every clause of the operation.
    pub(super) fn footprint_for(
        &self,
        application: &Application,
        receiver: &ObjectReference,
        arguments: &[(String, Option<ObjectReference>)],
        selected: Option<&Arc<BTreeSet<NodeKey>>>,
    ) -> Result<Footprint, FootprintRefusal> {
        let types: Vec<(String, EffectiveId)> = arguments
            .iter()
            .filter_map(|(name, bound)| {
                bound
                    .as_ref()
                    .map(|bound| (name.clone(), bound.object_type()))
            })
            .collect();
        let key = (
            receiver.object_type(),
            application.text.clone(),
            types,
            selected.cloned(),
        );
        let mut cache = self
            .footprints
            .0
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let symbolic = cache
            .entry(key)
            .or_insert_with_key(|(_, _, types, selected)| {
                derive_selected_footprint(
                    self.package.graph(),
                    &self.model,
                    receiver.object_type(),
                    application.declaring,
                    &application.declaration,
                    types,
                    &|clause| selected.as_ref().is_none_or(|set| set.contains(&clause)),
                )
            });
        symbolic
            .as_ref()
            .map(|symbolic| instantiate(symbolic, receiver, arguments))
            .map_err(Clone::clone)
    }

    /// The footprint of `transition` (FR-155 "Instantiation"): the operation's
    /// symbolic footprint with the receiver replaced by the transition's
    /// receiver and each reference parameter by its argument. It reads the
    /// transition's canonical texts alone, so a disabled transition, or one
    /// over an object that does not exist yet, has a footprint, and it
    /// depends on the checked package and the transition identity alone.
    ///
    /// # Errors
    ///
    /// [`FootprintError::Refused`] when the operation's clauses read through a
    /// form with no footprint rule, and [`FootprintError::UnknownTransition`]
    /// when the transition names no operation of its receiver's type, or a
    /// receiver or reference argument that is no reference text.
    pub fn footprint(&self, transition: &ModelTransition) -> Result<Footprint, FootprintError> {
        self.footprint_selecting(transition, None)
    }

    /// What enumerating the records of `value_type` reads (ADR-027 FT-3): a
    /// type that holds an object reference ranges over the objects of the
    /// state, so it reads the any-object membership of every population;
    /// any other type reads nothing.
    pub(crate) fn record_reads(&self, value_type: &ValueType) -> BTreeSet<Location> {
        if !holds_reference(value_type) {
            return BTreeSet::new();
        }
        self.model
            .populations()
            .into_iter()
            .map(|population| Location::AnyMembership {
                population: population.key,
            })
            .collect()
    }

    /// [`Self::footprint`] over only the clauses `selected` names: a protocol
    /// attempt's application with the contracts its node selects (ADR-027
    /// FT-1).
    pub(crate) fn footprint_selecting(
        &self,
        transition: &ModelTransition,
        selected: Option<&Arc<BTreeSet<NodeKey>>>,
    ) -> Result<Footprint, FootprintError> {
        let unknown = || FootprintError::UnknownTransition;
        let receiver = transition
            .receiver()
            .and_then(parse_reference)
            .ok_or_else(unknown)?;
        let application = self
            .applications
            .get(&receiver.object_type())
            .and_then(|applications| {
                applications
                    .iter()
                    .find(|application| application.text == transition.operation())
            })
            .ok_or_else(unknown)?;

        if application.declaration.parameters().len() != transition.arguments().len() {
            return Err(unknown());
        }
        let mut arguments = Vec::new();
        for ((name, value_type), text) in application
            .declaration
            .parameters()
            .iter()
            .zip(transition.arguments())
        {
            if binds_object(value_type) {
                arguments.push((
                    name.clone(),
                    parse_optional_reference(text).ok_or_else(unknown)?,
                ));
            }
        }
        Ok(self.footprint_for(application, &receiver, &arguments, selected)?)
    }

    /// Whether every operation of the subject has a derivable footprint, each
    /// reference parameter taken at its declared type. A partial-order run
    /// asks for this before it expands a state, so an operation whose clauses
    /// read through a form with no footprint rule refuses the run instead of
    /// stopping an expansion.
    ///
    /// # Errors
    ///
    /// [`FootprintError::Refused`] naming the first clause with no rule.
    pub fn check_footprints_derivable(&self) -> Result<(), FootprintError> {
        for (receiver_type, applications) in &self.applications {
            for application in applications {
                let arguments: Vec<(String, EffectiveId)> = application
                    .declaration
                    .parameters()
                    .iter()
                    .filter_map(|(name, value_type)| {
                        declared_target(value_type).map(|target| (name.clone(), target))
                    })
                    .collect();
                derive_footprint(
                    self.package.graph(),
                    &self.model,
                    *receiver_type,
                    application.declaring,
                    &application.declaration,
                    &arguments,
                )?;
            }
        }
        Ok(())
    }

    /// FR-156 "Visibility": the locations the state constraint `name` reads.
    /// The constraint is evaluated for every object of its context type, so
    /// its reads are on any object.
    ///
    /// # Errors
    ///
    /// [`FootprintError::UnknownConstraint`] when the package declares no
    /// such clause, and [`FootprintError::Refused`] when the clause reads
    /// through a form QSpec FR-383 gives no footprint rule.
    pub fn constraint_reads(&self, name: &str) -> Result<BTreeSet<Location>, FootprintError> {
        let graph = self.package.graph();
        let clause = graph
            .state_clauses()
            .iter()
            .find(|clause| clause.name() == name)
            .ok_or(FootprintError::UnknownConstraint)?;
        let symbolic = derive_constraint_reads(graph, &self.model, clause)?;
        Ok(symbolic
            .iter()
            .filter_map(|location| lower(location, &|_| None))
            .collect())
    }

    /// ADR-027 FT-3: the model locations a protocol expression `body`, typed
    /// over the binders `parameters` in slot order, reads: for each field
    /// read, the field of every object of the subject's universe whose type
    /// declares it (the field on any object when none does), and its
    /// membership reads.
    ///
    /// # Errors
    ///
    /// [`FootprintError::Refused`] when the expression reads through a form
    /// QSpec FR-383 gives no footprint rule.
    pub(crate) fn slot_reads(
        &self,
        name: &str,
        parameters: &[(String, ValueType)],
        body: &qsl_semantics::check::CheckedBody,
    ) -> Result<BTreeSet<Location>, FootprintError> {
        let symbolic = derive_slot_reads(
            self.package.graph(),
            &self.model,
            name,
            parameters,
            body.root(),
        )?;
        Ok(symbolic
            .iter()
            .filter_map(|location| lower(location, &|_| None))
            .flat_map(|location| match location {
                Location::AnyField { field } => match self.universe_declaring(&field) {
                    Some(objects) => objects
                        .into_iter()
                        .map(|object| Location::Field {
                            object,
                            field: field.clone(),
                        })
                        .collect(),
                    None => vec![Location::AnyField { field }],
                },
                other => vec![other],
            })
            .collect())
    }

    /// FR-156 "Atom footprints": the symbolic locations the `holds` atom
    /// whose checked condition is `condition` reads, with the clause's `over`
    /// parameter `over` as the receiver. A [`ModelLetters`] over a checked
    /// package answers its `reads` with this.
    ///
    /// [`ModelLetters`]: crate::model_subject::ModelLetters
    ///
    /// # Errors
    ///
    /// [`FootprintRefusal`] when the condition reads through a form QSpec
    /// FR-383 gives no footprint rule.
    pub fn atom_reads(
        &self,
        over: &(String, ValueType),
        condition: CheckedNode<'_>,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        derive_atom_reads(self.package.graph(), &self.model, over, condition)
    }

    /// FR-156 "Atom footprints": the locations an atom reads for `instance`
    /// at `state`, from the atom's symbolic reads, which name the instance's
    /// object as the receiver. An instance whose object `state` does not
    /// hold reads the field on any object and the population's membership of
    /// any object: a dependence kept, never an independence denied.
    pub fn atom_locations(
        &self,
        reads: &BTreeSet<SymbolicLocation>,
        instance: &ModelInstance,
        state: &ModelSimState,
    ) -> BTreeSet<Location> {
        let object = state
            .state
            .populations
            .get(&instance.population)
            .and_then(|objects| objects.get(&instance.key))
            .and_then(|object| {
                self.model
                    .object_reference(&object.type_identity, &instance.key)
                    .ok()
            });
        reads
            .iter()
            .filter_map(|location| match (location, &object) {
                (
                    SymbolicLocation::Field {
                        object: FootprintObject::Receiver,
                        field,
                    },
                    None,
                ) => Some(Location::AnyField {
                    field: field.clone(),
                }),
                (
                    SymbolicLocation::Membership {
                        population,
                        object: FootprintObject::Receiver,
                    },
                    None,
                ) => Some(Location::AnyMembership {
                    population: population.clone(),
                }),
                _ => lower(location, &|bound| match bound {
                    FootprintObject::Receiver => object.clone(),
                    FootprintObject::Param(_) => None,
                }),
            })
            .collect()
    }

    /// Expand `state` for a partial-order run (FR-155 "Read enforcement"):
    /// as [`Self::successors_within`] with nothing withheld, a read outside
    /// a step's footprint stopping the expansion as an internal failure.
    ///
    /// # Errors
    ///
    /// An [`ExpansionStop`] as `successors` returns it, or
    /// `runtime_invariant`/`established-invariant-broken` when a step reads
    /// outside its footprint or a transition has none.
    pub fn successors_enforced(
        &self,
        state: &ModelSimState,
    ) -> Result<Expansion<ModelTransition, ModelSimState, super::ModelFinding>, ExpansionStop> {
        self.successors_within(state, &|_| BTreeSet::new())
            .map_err(|stopped| match stopped {
                WithinStop::Expansion(stop) => stop,
                WithinStop::Footprint(_) | WithinStop::Violation(_) => stop(BROKEN),
            })
    }

    /// Expand `state` as `successors` does, holding every step's `pre` and
    /// `post` clauses to its derived footprint less the locations `withhold`
    /// names for it (FR-155 "Read enforcement"). The engine withholds
    /// nothing; withholding a location shows what a step reads beyond a
    /// narrower footprint, and a footprint can only shrink.
    ///
    /// # Errors
    ///
    /// [`WithinStop::Violation`] when a clause reads a location outside the
    /// footprint, [`WithinStop::Footprint`] when a transition has none, and
    /// [`WithinStop::Expansion`] when the expansion stops as `successors`
    /// stops it.
    pub fn successors_within(
        &self,
        state: &ModelSimState,
        withhold: &dyn Fn(&ModelTransition) -> BTreeSet<Location>,
    ) -> Result<Expansion<ModelTransition, ModelSimState, super::ModelFinding>, WithinStop> {
        let enforcement = Enforcement {
            withhold,
            fault: RefCell::new(None),
        };
        match super::walk::expand_state(self, state, Some(&enforcement)) {
            Ok(expansion) => Ok(expansion),
            Err(stopped) => Err(match enforcement.fault.into_inner() {
                Some(Fault::Footprint(error)) => WithinStop::Footprint(error),
                Some(Fault::Violation(violation)) => WithinStop::Violation(violation),
                None => WithinStop::Expansion(stopped),
            }),
        }
    }
}
