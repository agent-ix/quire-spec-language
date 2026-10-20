// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-176 (ADR-023 HC-1): the self-composition product as a FR-101
//! [`TransitionSystem`].
//!
//! A product state is the tuple of the components' model states, one per
//! trace variable in quantifier order, with the state of the generalized
//! Büchi automaton of the negated body after it read the joint position. Its
//! key is the components' FR-101 state keys in quantifier order and the
//! automaton state's index. The initial states are every tuple of initial
//! states, each component from its own subject's list, with the automaton
//! reading joint position 0: the same subject twice gives every ordered pair,
//! a state's pair with itself included.
//!
//! A successor is a tuple of component steps, each an FR-120 transition
//! enabled at its component or, at a terminal component, the stutter step
//! (ADR-023 HM-5), on which `μ_U` holds. A joint step's identity is the tuple
//! of the components' identities ([`HyperTransition`]); each joint post-state
//! and each automaton successor on its letter is a successor. A joint step on
//! which `μ_U` is undefined is no successor: it is recorded as a finding for
//! the engine, which ends the run there (ADR-023 HV-8).
//!
//! A position's letter is read when the product state for it is created,
//! before the automaton steps on it (ADR-018 UE-4). A letter with an atom that
//! has no value makes the state [`Property::Undefined`]; no step follows it. A
//! position at which the automaton has no move makes the state
//! [`Property::Dead`], which keeps reading letters and never accepts, so a
//! letter or a contract that is undefined later on the same joint behaviour is
//! still found (ADR-018 UE-1).
//!
//! The expansion records why it stops in [`Halt`], which [`HyperProduct::take_halt`]
//! hands to the caller, as the `qsl-analyze` liveness product does: a
//! `TransitionSystem` stop carries a catalog code only.

use std::cell::{Ref, RefCell};
use std::collections::BTreeMap;

use qsl_foundation::digest::DigestRecord;
use qsl_foundation::{CatalogCode, IdentityLimits, InternalFault};
use qsl_semantics::check::{CheckedFormula, FormulaNodeId, TemporalProfile};
use quire_canonical::{Encode, Sink, Writer};
use quire_exact::{Cancel, CancelCause, Meter, ScalarLimits};
use quire_semantic_value::model_transition::ModelTransition;

use crate::automaton::{
    translate_negation, Atom, AutomatonError, Letter, PropertyAutomaton, StateId,
};
use crate::hyper::seam::{HyperLetters, MatchReading, ParameterBinding, StepLabel, StepMatch};
use crate::simulation::model::{ModelFinding, ModelKey, ModelSimState, ModelSystem};
use crate::simulation::{EncodingRefusal, Expansion, ExpansionStop, TransitionSystem};
use crate::temporal::{AtomReading, UndefinedCause};

/// `resource_exhausted`/`insufficient-next-charge`, the code of an expansion
/// that stopped; [`Halt`] records which budget it was.
const STOPPED: CatalogCode = CatalogCode::new("resource_exhausted", "insufficient-next-charge");

/// The property component of a product state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Property {
    /// The automaton state after reading the joint position.
    After(StateId),
    /// The automaton had no move on the joint position's letter, on any run:
    /// no violation passes through here, but later positions are still read.
    Dead,
    /// An atom of the joint position's letter has no value.
    Undefined {
        /// The first such atom, in the automaton's atom order.
        atom: FormulaNodeId,
        /// Why it has no value.
        cause: UndefinedCause,
    },
}

/// A product state: the components' model states and the property state.
#[derive(Clone, Debug)]
pub struct HyperState {
    /// One model state per trace variable, in quantifier order.
    pub components: Vec<ModelSimState>,
    /// Where the automaton is after reading this joint position.
    pub property: Property,
}

/// One component's part of a joint step.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ComponentStep {
    /// An FR-120 transition enabled at the component.
    Model(ModelTransition),
    /// The stutter step of a terminal component (ADR-018 SM-4), with no model
    /// operation.
    Stutter,
}

impl Encode for ComponentStep {
    fn encode_into<S: Sink + ?Sized>(
        &self,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        writer.begin_array()?;
        match self {
            Self::Model(identity) => {
                writer.string("model")?;
                identity.encode_into(writer)?;
            }
            Self::Stutter => writer.string("terminal-stutter")?,
        }
        writer.end_array()
    }
}

/// A joint step's identity: the tuple of the components' step identities in
/// quantifier order (ADR-023 HC-1).
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct HyperTransition(pub Vec<ComponentStep>);

impl HyperTransition {
    /// The labels `match` conjuncts read for this step.
    pub fn labels(&self) -> Vec<StepLabel<'_>> {
        self.0
            .iter()
            .map(|step| match step {
                ComponentStep::Model(transition) => StepLabel::Operation(transition),
                ComponentStep::Stutter => StepLabel::Stutter,
            })
            .collect()
    }
}

impl Encode for HyperTransition {
    fn encode_into<S: Sink + ?Sized>(
        &self,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        writer.begin_array()?;
        for step in &self.0 {
            step.encode_into(writer)?;
        }
        writer.end_array()
    }
}

/// A product state's canonical key: the components' state keys in quantifier
/// order and the property state.
pub struct HyperKey {
    components: Vec<ModelKey>,
    property: PropertyKey,
}

enum PropertyKey {
    After(usize),
    Dead,
    Undefined,
}

impl Encode for HyperKey {
    fn encode_into<S: Sink + ?Sized>(
        &self,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        writer.begin_array()?;
        writer.begin_array()?;
        for component in &self.components {
            component.encode_into(writer)?;
        }
        writer.end_array()?;
        writer.begin_array()?;
        match self.property {
            PropertyKey::After(index) => {
                writer.string("after")?;
                writer.integer(i128::try_from(index).unwrap_or(i128::MAX))?;
            }
            PropertyKey::Dead => writer.string("dead")?,
            PropertyKey::Undefined => writer.string("undefined")?,
        }
        writer.end_array()?;
        writer.end_array()
    }
}

/// What an expansion found beside its successors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HyperFinding {
    /// A contract conjunction the component's expansion evaluated undefined
    /// or refused.
    Contract {
        /// The component, by its index in quantifier order.
        component: usize,
        /// The component's finding.
        finding: ModelFinding,
    },
    /// The transition identities the component's model state enables
    /// (ADR-018 FA-2): read from the component's own successor relation,
    /// whatever `μ_U` leaves of the joint steps.
    Enabled {
        /// The component, by its index in quantifier order.
        component: usize,
        /// Its enabled identities, ascending and without repeats.
        transitions: Vec<ModelTransition>,
    },
    /// `μ_U` has no value on a joint step (ADR-023 HV-8). The step is no
    /// successor.
    MatchUndefined {
        /// The joint step.
        step: HyperTransition,
        /// The digest of each component's post-state, in quantifier order.
        posts: Vec<DigestRecord>,
        /// The index of the conjunct that has no value.
        conjunct: usize,
        /// Why it has no value.
        cause: UndefinedCause,
    },
    /// `μ_U` was refused or incomplete on a joint step: the successor
    /// relation is not exactly known.
    MatchRefused,
}

/// Why a product's exploration ended before its frontier emptied.
#[derive(Debug)]
pub enum Halt {
    /// A component's expansion stopped.
    Expansion {
        /// The component, by its index in quantifier order.
        component: usize,
        /// Why its expansion stopped.
        stop: ExpansionStop,
    },
    /// An atom's evaluation exhausted the meter.
    Meter,
    /// The caller cancelled the run.
    Cancelled(CancelCause),
    /// The automaton reached `max_automaton_states`.
    Automaton(u64),
    /// A state or transition identity has no canonical encoding.
    Encoding(EncodingRefusal),
    /// An invariant of the product broke.
    Fault(InternalFault),
}

/// What [`HyperProduct::new`] reads.
pub struct HyperInputs<'r, 's> {
    /// The `ModelSystem` of each trace variable's subject, in quantifier
    /// order; two variables of one alias name the same system.
    pub systems: Vec<&'r ModelSystem<'s>>,
    /// The clause instance: each object parameter's key.
    pub instance: &'r [ParameterBinding],
    /// The negated body's formula and its profile.
    pub body: &'r CheckedFormula,
    /// The profile the body was checked under.
    pub profile: TemporalProfile,
    /// The seam to the body's atoms.
    pub letters: &'r dyn HyperLetters,
    /// The seam to `μ_U`; `None` when the clause has no universal conjunct.
    pub matcher: Option<&'r dyn StepMatch>,
    /// FR-184's `max_automaton_states`.
    pub max_automaton_states: u64,
    /// The budget of each atom or `μ_U` evaluation.
    pub meter: ScalarLimits,
    /// The run's cancellation handle.
    pub cancel: &'r Cancel,
}

/// One joint position's letter, or the first atom that has no value there.
#[derive(Clone, Debug)]
enum Reading {
    Defined(Letter),
    Undefined {
        atom: FormulaNodeId,
        cause: UndefinedCause,
    },
}

/// Where the automaton is before a joint position is read.
#[derive(Clone, Copy)]
enum Before {
    Start,
    At(StateId),
    Dead,
}

/// The self-composition of a hyper clause's subjects as a
/// [`TransitionSystem`] (ADR-023 HC-1).
pub struct HyperProduct<'r, 's> {
    systems: Vec<&'r ModelSystem<'s>>,
    instance: &'r [ParameterBinding],
    letters: &'r dyn HyperLetters,
    matcher: Option<&'r dyn StepMatch>,
    atoms: Vec<FormulaNodeId>,
    automaton: RefCell<PropertyAutomaton>,
    start: StateId,
    readings: RefCell<BTreeMap<Vec<DigestRecord>, Reading>>,
    initial: Vec<HyperState>,
    meter: ScalarLimits,
    cancel: &'r Cancel,
    halt: RefCell<Option<Halt>>,
}

impl<'r, 's> HyperProduct<'r, 's> {
    /// Translate the negated body and bind the product to its inputs. The
    /// initial states are read by [`Self::start`].
    ///
    /// # Errors
    ///
    /// [`AutomatonError`] when the body has no property automaton, or
    /// `max_automaton_states` admits no state.
    pub fn new(inputs: HyperInputs<'r, 's>) -> Result<Self, AutomatonError> {
        let mut automaton =
            translate_negation(inputs.body, inputs.profile, inputs.max_automaton_states)?;
        let start = automaton.initial()?;
        Ok(Self {
            systems: inputs.systems,
            instance: inputs.instance,
            letters: inputs.letters,
            matcher: inputs.matcher,
            atoms: automaton.atoms().to_vec(),
            automaton: RefCell::new(automaton),
            start,
            readings: RefCell::new(BTreeMap::new()),
            initial: Vec::new(),
            meter: inputs.meter,
            cancel: inputs.cancel,
            halt: RefCell::new(None),
        })
    }

    /// Read the initial product states: every tuple of initial states, each
    /// component from its own subject's list, with the automaton reading joint
    /// position 0.
    ///
    /// # Errors
    ///
    /// The stop the exploration engine takes; [`Self::take_halt`] says why.
    pub fn start(&mut self) -> Result<(), ExpansionStop> {
        let lists: Vec<Vec<ModelSimState>> =
            self.systems.iter().map(|system| system.initial()).collect();
        let mut initial = Vec::new();
        for components in cartesian(&lists) {
            let reading = self.reading(&components)?;
            for property in self.step(Before::Start, &reading)? {
                initial.push(HyperState {
                    components: components.clone(),
                    property,
                });
            }
        }
        self.initial = initial;
        Ok(())
    }

    /// Why the exploration stopped, once; `None` when it did not stop here.
    pub fn take_halt(&self) -> Option<Halt> {
        self.halt.borrow_mut().take()
    }

    /// The automaton of the negated body: its acceptance sets and the states
    /// it has reached.
    pub fn automaton(&self) -> Ref<'_, PropertyAutomaton> {
        self.automaton.borrow()
    }

    /// The `ModelSystem` of each trace variable, in quantifier order.
    pub fn systems(&self) -> &[&'r ModelSystem<'s>] {
        &self.systems
    }

    /// Record why the run stops, keeping the first reason, and return the stop
    /// the exploration engine takes.
    fn record(&self, halt: Halt) -> ExpansionStop {
        let mut slot = self.halt.borrow_mut();
        if slot.is_none() {
            *slot = Some(halt);
        }
        ExpansionStop {
            cause: STOPPED,
            limit: None,
        }
    }

    fn poll(&self) -> Result<(), ExpansionStop> {
        if self.cancel.poll() {
            if let Some(source) = self.cancel.cause() {
                return Err(self.record(Halt::Cancelled(source)));
            }
        }
        Ok(())
    }

    /// The FR-101 state-key digest of each component.
    fn digests(&self, components: &[ModelSimState]) -> Result<Vec<DigestRecord>, ExpansionStop> {
        self.systems
            .iter()
            .zip(components)
            .map(|(system, state)| {
                system
                    .state_key(state)
                    .map(|(_, digest)| digest)
                    .map_err(|refusal| self.record(Halt::Encoding(refusal)))
            })
            .collect()
    }

    /// The letter of the joint position `components`, read once per tuple of
    /// model states.
    fn reading(&self, components: &[ModelSimState]) -> Result<Reading, ExpansionStop> {
        let digests = self.digests(components)?;
        if let Some(known) = self.readings.borrow().get(&digests) {
            return Ok(known.clone());
        }
        let states: Vec<&ModelSimState> = components.iter().collect();
        let mut letter = Letter::default();
        let mut reading = None;
        for (index, atom) in self.atoms.iter().enumerate() {
            self.poll()?;
            let mut meter = Meter::new(self.meter);
            let value = self
                .letters
                .atom(*atom, self.instance, &states, &mut meter)
                .map_err(|_| self.record(Halt::Meter))?;
            match value {
                AtomReading::True => {
                    let atom = Atom::new(index).map_err(|_| {
                        self.record(Halt::Fault(InternalFault::new(
                            "hyper-product",
                            "atom-in-range",
                        )))
                    })?;
                    letter = letter.with(atom);
                }
                AtomReading::False => {}
                AtomReading::Undefined(cause) => {
                    reading = Some(Reading::Undefined { atom: *atom, cause });
                    break;
                }
            }
        }
        let reading = reading.unwrap_or(Reading::Defined(letter));
        self.readings.borrow_mut().insert(digests, reading.clone());
        Ok(reading)
    }

    /// The property states a joint position reads into, from `before`.
    fn step(&self, before: Before, reading: &Reading) -> Result<Vec<Property>, ExpansionStop> {
        let letter = match reading {
            Reading::Undefined { atom, cause } => {
                return Ok(vec![Property::Undefined {
                    atom: *atom,
                    cause: cause.clone(),
                }])
            }
            Reading::Defined(letter) => *letter,
        };
        let from = match before {
            Before::Dead => return Ok(vec![Property::Dead]),
            Before::Start => self.start,
            Before::At(state) => state,
        };
        let targets = self
            .automaton
            .borrow_mut()
            .successors(from, letter)
            .map_err(|error| match error {
                AutomatonError::StateLimit { limit } => self.record(Halt::Automaton(limit)),
                _ => self.record(Halt::Fault(InternalFault::new(
                    "hyper-product",
                    "a-reached-automaton-state-expands",
                ))),
            })?;
        Ok(if targets.is_empty() {
            vec![Property::Dead]
        } else {
            targets.into_iter().map(Property::After).collect()
        })
    }

    /// `μ_U` over `step`, or a finding when it has no value or was refused.
    fn matched(&self, step: &HyperTransition) -> Result<Matched, ExpansionStop> {
        let Some(matcher) = self.matcher else {
            return Ok(Matched::Holds);
        };
        self.poll()?;
        let labels = step.labels();
        let mut meter = Meter::new(self.meter);
        Ok(
            match matcher.universal(self.instance, &labels, &mut meter) {
                Ok(MatchReading::Holds) => Matched::Holds,
                Ok(MatchReading::Fails) => Matched::Fails,
                Ok(MatchReading::Undefined { conjunct, cause }) => {
                    Matched::Undefined { conjunct, cause }
                }
                Ok(MatchReading::Refused) | Err(_) => Matched::Refused,
            },
        )
    }
}

/// What `μ_U` made of one joint step.
enum Matched {
    Holds,
    Fails,
    Undefined {
        conjunct: usize,
        cause: UndefinedCause,
    },
    Refused,
}

/// Every tuple that takes one element of each list, in the order of the
/// lists: the last list varies fastest. No list gives no tuple but the empty
/// one; an empty list gives none.
pub fn cartesian<T: Clone>(lists: &[Vec<T>]) -> Vec<Vec<T>> {
    lists.iter().fold(vec![Vec::new()], |tuples, list| {
        tuples
            .into_iter()
            .flat_map(|tuple| {
                list.iter().map(move |element| {
                    let mut next = tuple.clone();
                    next.push(element.clone());
                    next
                })
            })
            .collect()
    })
}

/// A component's step options out of its expansion: each enabled transition
/// and its post-state, or the stutter step at a terminal component.
type Options = Vec<(ComponentStep, ModelSimState)>;

impl TransitionSystem for HyperProduct<'_, '_> {
    type State = HyperState;
    type TransitionId = HyperTransition;
    type Key = HyperKey;
    type Finding = HyperFinding;

    fn initial(&self) -> Vec<HyperState> {
        self.initial.clone()
    }

    fn key(&self, state: &HyperState) -> HyperKey {
        HyperKey {
            components: self
                .systems
                .iter()
                .zip(&state.components)
                .map(|(system, component)| system.key(component))
                .collect(),
            property: match &state.property {
                Property::After(at) => PropertyKey::After(at.index()),
                Property::Dead => PropertyKey::Dead,
                Property::Undefined { .. } => PropertyKey::Undefined,
            },
        }
    }

    fn successors(
        &self,
        state: &HyperState,
    ) -> Result<Expansion<HyperTransition, HyperState, HyperFinding>, ExpansionStop> {
        self.poll()?;
        let before = match &state.property {
            // The first phase ends at this state; nothing follows it.
            Property::Undefined { .. } => {
                return Ok(Expansion {
                    successors: Vec::new(),
                    findings: Vec::new(),
                })
            }
            Property::Dead => Before::Dead,
            Property::After(at) => Before::At(*at),
        };
        let mut findings = Vec::new();
        let mut options: Vec<Options> = Vec::with_capacity(self.systems.len());
        for (component, (system, model)) in self.systems.iter().zip(&state.components).enumerate() {
            let expansion = system.successors(model).inspect_err(|&stop| {
                self.record(Halt::Expansion { component, stop });
            })?;
            findings.extend(
                expansion
                    .findings
                    .into_iter()
                    .filter(|finding| {
                        finding.undefined_contract().is_some() || finding.is_refused_contract()
                    })
                    .map(|finding| HyperFinding::Contract { component, finding }),
            );
            let mut enabled: Vec<ModelTransition> = expansion
                .successors
                .iter()
                .map(|(transition, _)| transition.clone())
                .collect();
            enabled.sort();
            enabled.dedup();
            findings.push(HyperFinding::Enabled {
                component,
                transitions: enabled,
            });
            options.push(if expansion.successors.is_empty() {
                vec![(ComponentStep::Stutter, model.clone())]
            } else {
                expansion
                    .successors
                    .into_iter()
                    .map(|(transition, post)| (ComponentStep::Model(transition), post))
                    .collect()
            });
        }
        // The joint steps in the canonical order of their identities, so the
        // first undefined `μ_U` is the first in canonical order (FR-176-AC-4).
        let mut candidates: Vec<(Vec<u8>, HyperTransition, Vec<ModelSimState>)> = Vec::new();
        for choice in cartesian(&options) {
            let (steps, posts): (Vec<ComponentStep>, Vec<ModelSimState>) =
                choice.into_iter().unzip();
            let step = HyperTransition(steps);
            let identity = IdentityLimits::default();
            let order =
                quire_canonical::to_vec(&step, quire_canonical::Limits::new(identity.input_bytes))
                    .map_err(|error| {
                        self.record(Halt::Encoding(EncodingRefusal::of(&error, identity)))
                    })?;
            candidates.push((order, step, posts));
        }
        candidates.sort_by(|left, right| left.0.cmp(&right.0));
        let mut successors = Vec::new();
        for (_, step, posts) in candidates {
            match self.matched(&step)? {
                Matched::Fails => continue,
                Matched::Holds => {}
                Matched::Undefined { conjunct, cause } => {
                    findings.push(HyperFinding::MatchUndefined {
                        posts: self.digests(&posts)?,
                        step,
                        conjunct,
                        cause,
                    });
                    continue;
                }
                Matched::Refused => {
                    findings.push(HyperFinding::MatchRefused);
                    continue;
                }
            }
            let reading = self.reading(&posts)?;
            for property in self.step(before, &reading)? {
                successors.push((
                    step.clone(),
                    HyperState {
                        components: posts.clone(),
                        property,
                    },
                ));
            }
        }
        Ok(Expansion {
            successors,
            findings,
        })
    }
}
