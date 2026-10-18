// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-155 "Instantiation" and "Read enforcement": the footprint of a model
//! transition, and the check that the `pre` and `post` clauses of a step read
//! nothing outside it.
//!
//! The symbolic footprint of an operation is derived once, from its checked
//! clauses and frame, by `qsl_semantics::model::footprint`. Instantiating it
//! replaces the receiver and each reference parameter with the object the
//! transition binds. Enforcement runs a step's clauses with a read log
//! attached to their observations and compares the log with the footprint.

use std::cell::RefCell;
use std::collections::BTreeSet;

use qsl_semantics::check::Observation;
use qsl_semantics::model::footprint::{
    FootprintObject, FootprintRefusal, SymbolicFootprint, SymbolicLocation,
};
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::object_environment::{ObservedRead, ReadLog};
use qsl_semantics::model::observation::AdmittedObservations;
use qsl_semantics::model::state::StateModel;
use quire_canonical::Limits as CanonicalLimits;
use quire_exact::{ObjectReference, Value, ValueType};
use quire_semantic_value::model_transition::ModelTransition;

use super::domain::Domain;
use super::encode::{reference_enc, value_enc};
use super::walk::{rows_of, stop};
use super::{identity_limits, Application, ModelSimState, ModelSystem, BROKEN};
use crate::simulation::expansion::{Expansion, ExpansionStop};
use crate::simulation::footprint::{Footprint, Location};

/// Why a transition has no footprint.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum FootprintError {
    /// The operation's clauses read through a form QSpec FR-383 gives no
    /// footprint rule.
    #[error(transparent)]
    Refused(#[from] FootprintRefusal),
    /// The transition is no application of the state's objects.
    #[error("the transition names no operation and receiver of the state")]
    UnknownTransition,
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

/// The footprints an enforcing expansion checks each step against.
pub(super) struct Enforcement<'e> {
    narrow: &'e dyn Fn(&ModelTransition, Footprint) -> Footprint,
    fault: RefCell<Option<Fault>>,
}

impl Enforcement<'_> {
    /// The footprint `transition` is enforced against: the derived footprint
    /// of `application`, instantiated and passed through the caller's
    /// restriction.
    pub(super) fn footprint_of(
        &self,
        application: &Application,
        receiver: &ObjectReference,
        arguments: &[(String, Value)],
        transition: &ModelTransition,
    ) -> Result<Footprint, ExpansionStop> {
        match &application.footprint {
            Ok(symbolic) => Ok((self.narrow)(
                transition,
                instantiate(symbolic, receiver, arguments),
            )),
            Err(refusal) => {
                self.fail(Fault::Footprint(FootprintError::Refused(refusal.clone())));
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
    arguments: &[(String, Value)],
) -> Footprint {
    let bind = |object: &FootprintObject| match object {
        FootprintObject::Receiver => Some(receiver.clone()),
        FootprintObject::Param(name) => arguments
            .iter()
            .find(|(parameter, _)| parameter == name)
            .and_then(|(_, value)| referenced(value))
            .cloned(),
    };
    let locations = |symbolic: &BTreeSet<SymbolicLocation>| -> BTreeSet<Location> {
        symbolic
            .iter()
            .filter_map(|location| match location {
                SymbolicLocation::Field { object, field } => {
                    bind(object).map(|object| Location::Field {
                        object,
                        field: field.clone(),
                    })
                }
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
            })
            .collect()
    };
    Footprint {
        reads: locations(&symbolic.reads),
        writes: locations(&symbolic.writes),
        enabling: locations(&symbolic.enabling),
    }
}

/// The canonical text of `reference`, as a transition names its receiver.
fn reference_text(reference: &ObjectReference) -> Option<String> {
    reference_enc(reference)
        .canonical_text(CanonicalLimits::new(identity_limits().input_bytes))
        .ok()
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
    /// The footprint of `transition` at `state` (FR-155 "Instantiation"): the
    /// operation's symbolic footprint with the receiver replaced by the
    /// transition's receiver and each reference parameter by its argument.
    /// The state supplies the objects the transition's canonical texts name.
    /// A footprint depends on the checked package and the transition
    /// identity alone.
    ///
    /// # Errors
    ///
    /// [`FootprintError::Refused`] when the operation's clauses read through a
    /// form with no footprint rule, and [`FootprintError::UnknownTransition`]
    /// when `state` holds no object and application the transition names.
    pub fn footprint(
        &self,
        state: &ModelSimState,
        transition: &ModelTransition,
    ) -> Result<Footprint, FootprintError> {
        let unknown = || FootprintError::UnknownTransition;
        let rows = rows_of(self, &state.state).map_err(|_| unknown())?;
        let row = rows
            .iter()
            .find(|row| reference_text(&row.reference).as_deref() == transition.receiver())
            .ok_or_else(unknown)?;
        let application = self
            .applications
            .get(&row.type_id())
            .and_then(|applications| {
                applications
                    .iter()
                    .find(|application| application.text == transition.operation())
            })
            .ok_or_else(unknown)?;
        let symbolic = application.footprint.as_ref().map_err(Clone::clone)?;

        let references: Vec<ObjectReference> =
            rows.iter().map(|row| row.reference.clone()).collect();
        let mut arguments = Vec::new();
        for ((name, value_type), text) in application
            .declaration
            .parameters()
            .iter()
            .zip(transition.arguments())
        {
            if !binds_object(value_type) {
                continue;
            }
            let domain = Domain::of(value_type, &references, self.types).ok_or_else(unknown)?;
            let value = (0..domain.len())
                .filter_map(|index| domain.nth(index))
                .find(|value| {
                    value_enc(value)
                        .canonical_text(CanonicalLimits::new(identity_limits().input_bytes))
                        .is_ok_and(|candidate| candidate == *text)
                })
                .ok_or_else(unknown)?;
            arguments.push((name.clone(), value));
        }
        Ok(instantiate(symbolic, &row.reference, &arguments))
    }

    /// Expand `state` as `successors` does, holding every step's `pre` and
    /// `post` clauses to the footprint `restrict` returns for it (FR-155
    /// "Read enforcement"). `restrict` receives each transition with its
    /// derived footprint; the engine passes it through unchanged, and a
    /// narrower footprint shows what a step reads beyond it.
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
        restrict: &dyn Fn(&ModelTransition, Footprint) -> Footprint,
    ) -> Result<Expansion<ModelTransition, ModelSimState, super::ModelFinding>, WithinStop> {
        let enforcement = Enforcement {
            narrow: restrict,
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
