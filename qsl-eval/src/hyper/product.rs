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
use std::rc::Rc;

use qsl_foundation::digest::DigestRecord;
use qsl_foundation::InternalFault;
use qsl_semantics::check::{CheckedFormula, FormulaNodeId, TemporalProfile};
use quire_canonical::{Encode, Sink, Writer};
use quire_exact::{Cancel, CancelCause, ScalarLimits};
use quire_semantic_value::model_transition::ModelTransition;

use crate::automaton::{
    translate_negation, AtomSymmetry, AutomatonError, PropertyAutomaton, StateId,
};
use crate::hyper::copy_swap::{CopySwap, CopySwapInputs};
use crate::hyper::joint::{cartesian, Joint, Matched, Options, Reading, Seams};
use crate::hyper::seam::{HyperLetters, ParameterBinding, StepLabel, StepMatch};
use crate::hyper::witness::{Partner, WitnessSet};
use crate::simulation::model::{ModelFinding, ModelKey, ModelSimState, ModelSystem};
use crate::simulation::{
    EncodingRefusal, Expansion, ExpansionStop, OfferedReductions, Permutation, SymmetryGroup,
    TransitionSystem,
};
use crate::temporal::UndefinedCause;

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
        /// For an HP-3 product, the existential tuple whose joint position
        /// the letter was read at: how it was reached. `None` for HP-2.
        partner: Option<Rc<Partner>>,
    },
    /// HP-3: the witness set `X` of the universal tuple (ADR-023 HC-2).
    Witnesses(WitnessSet),
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
    Witnesses(Vec<(Vec<ModelKey>, usize)>),
}

impl HyperKey {
    /// The key of `state`, whose components are over `systems` in quantifier
    /// order.
    pub(super) fn of(systems: &[&ModelSystem<'_>], state: &HyperState) -> Self {
        Self {
            components: systems
                .iter()
                .zip(&state.components)
                .map(|(system, component)| system.key(component))
                .collect(),
            property: match &state.property {
                Property::After(at) => PropertyKey::After(at.index()),
                Property::Dead => PropertyKey::Dead,
                Property::Undefined { .. } => PropertyKey::Undefined,
                Property::Witnesses(set) => PropertyKey::Witnesses(
                    set.members()
                        .iter()
                        .map(|member| {
                            let offset = state.components.len();
                            let keys = member
                                .states()
                                .iter()
                                .enumerate()
                                .filter_map(|(index, held)| {
                                    systems.get(offset + index).map(|system| system.key(held))
                                })
                                .collect();
                            (keys, member.automaton().index())
                        })
                        .collect(),
                ),
            },
        }
    }
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
        match &self.property {
            PropertyKey::After(index) => {
                writer.string("after")?;
                writer.integer(i128::try_from(*index).unwrap_or(i128::MAX))?;
            }
            PropertyKey::Dead => writer.string("dead")?,
            PropertyKey::Undefined => writer.string("undefined")?,
            PropertyKey::Witnesses(members) => {
                writer.string("witnesses")?;
                for (components, automaton) in members {
                    writer.begin_array()?;
                    for component in components {
                        component.encode_into(writer)?;
                    }
                    writer.integer(i128::try_from(*automaton).unwrap_or(i128::MAX))?;
                    writer.end_array()?;
                }
            }
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
    /// `μ_U` or `μ_E` was refused or incomplete on a joint step: the
    /// successor relation is not exactly known.
    MatchRefused,
    /// HP-3: `μ_E` has no value on a joint step of a witness set's member
    /// against a universal step (ADR-023 HV-8). The step is no successor.
    ExistentialMatchUndefined {
        /// The joint step, every component's.
        step: HyperTransition,
        /// The digest of each component's post-state, in quantifier order.
        posts: Vec<DigestRecord>,
        /// The index of the conjunct among the existential conjuncts.
        conjunct: usize,
        /// Why it has no value.
        cause: UndefinedCause,
        /// How the member's existential tuple was reached.
        partner: Rc<Partner>,
    },
    /// HP-3: a contract conjunction an existential member's expansion
    /// evaluated undefined or refused. FR-177 states no verdict for it, so the
    /// successor relation of the member is not exactly known.
    MemberContract,
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
    /// HP-3: a witness set would hold more members than `max_witness_set`
    /// (FR-184); the count is the size it would have reached.
    WitnessSet(u64),
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
    /// The clause's copy group when the product canonicalises under it
    /// (FR-174): the body's automaton is then built in normal form.
    pub copy_swap: Option<CopySwapInputs<'r>>,
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
    pub(super) joint: Joint<'r, 's>,
    pub(super) automaton: RefCell<PropertyAutomaton>,
    pub(super) copy_swap: Option<CopySwap>,
    start: StateId,
    initial: Vec<HyperState>,
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
        let generators: Vec<Vec<usize>> = inputs
            .copy_swap
            .iter()
            .flat_map(|swap| swap.group.generators.iter().map(|g| g.atoms.clone()))
            .collect();
        let symmetry = inputs.copy_swap.as_ref().map(|_| AtomSymmetry {
            generators: &generators,
        });
        let mut automaton = translate_negation(
            inputs.body,
            inputs.profile,
            inputs.max_automaton_states,
            symmetry.as_ref(),
        )?;
        let start = automaton.initial()?;
        let symmetric = automaton.is_symmetric();
        Ok(Self {
            joint: Joint::new(
                inputs.systems,
                inputs.instance,
                Seams {
                    letters: inputs.letters,
                    universal: inputs.matcher,
                    existential: None,
                },
                automaton.atoms().to_vec(),
                inputs.meter,
                inputs.cancel,
            ),
            automaton: RefCell::new(automaton),
            copy_swap: inputs
                .copy_swap
                .as_ref()
                .filter(|_| symmetric)
                .map(CopySwap::new),
            start,
            initial: Vec::new(),
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
        let lists: Vec<Vec<ModelSimState>> = self
            .joint
            .systems
            .iter()
            .map(|system| system.initial())
            .collect();
        let mut initial = Vec::new();
        for components in cartesian(&lists) {
            let reading = self.joint.reading(&components)?;
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
        self.joint.take_halt()
    }

    /// The automaton of the negated body: its acceptance sets and the states
    /// it has reached.
    pub fn automaton(&self) -> Ref<'_, PropertyAutomaton> {
        self.automaton.borrow()
    }

    /// The `ModelSystem` of each trace variable, in quantifier order.
    pub fn systems(&self) -> &[&'r ModelSystem<'s>] {
        &self.joint.systems
    }

    /// The property states a joint position reads into, from `before`.
    fn step(&self, before: Before, reading: &Reading) -> Result<Vec<Property>, ExpansionStop> {
        let letter = match reading {
            Reading::Undefined { atom, cause } => {
                return Ok(vec![Property::Undefined {
                    atom: *atom,
                    cause: cause.clone(),
                    partner: None,
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
                AutomatonError::StateLimit { limit } => self.joint.record(Halt::Automaton(limit)),
                _ => self.joint.record(Halt::Fault(InternalFault::new(
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
}

impl TransitionSystem for HyperProduct<'_, '_> {
    type State = HyperState;
    type TransitionId = HyperTransition;
    type Key = HyperKey;
    type Finding = HyperFinding;

    fn initial(&self) -> Vec<HyperState> {
        self.initial.clone()
    }

    fn key(&self, state: &HyperState) -> HyperKey {
        HyperKey::of(&self.joint.systems, state)
    }

    fn offered_reductions(&self) -> OfferedReductions {
        OfferedReductions {
            symmetry: self.copy_swap.is_some(),
            ..OfferedReductions::NONE
        }
    }

    fn canonical(
        &self,
        state: &HyperState,
        _group: &SymmetryGroup,
    ) -> Option<Result<(HyperState, Permutation), ExpansionStop>> {
        let swap = self.copy_swap.as_ref()?;
        Some(self.canonicalise(swap, state))
    }

    fn successors(
        &self,
        state: &HyperState,
    ) -> Result<Expansion<HyperTransition, HyperState, HyperFinding>, ExpansionStop> {
        self.joint.poll()?;
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
            Property::Witnesses(_) => {
                return Err(self.joint.record(Halt::Fault(InternalFault::new(
                    "hyper-product",
                    "a-self-composition-state-has-no-witness-set",
                ))))
            }
        };
        let mut findings = Vec::new();
        let mut options: Vec<Options> = Vec::with_capacity(state.components.len());
        for (component, model) in state.components.iter().enumerate() {
            let stepping = self.joint.stepping(component, model)?;
            findings.extend(
                stepping
                    .contracts
                    .into_iter()
                    .map(|finding| HyperFinding::Contract { component, finding }),
            );
            findings.push(HyperFinding::Enabled {
                component,
                transitions: stepping.enabled,
            });
            options.push(stepping.options);
        }
        // The joint steps in the canonical order of their identities, so the
        // first undefined `μ_U` is the first in canonical order (FR-176-AC-4).
        let mut successors = Vec::new();
        for (step, posts) in self.joint.joint_steps(&options)? {
            match self.joint.matched_universal(&step)? {
                Matched::Fails => continue,
                Matched::Holds => {}
                Matched::Undefined { conjunct, cause } => {
                    findings.push(HyperFinding::MatchUndefined {
                        posts: self.joint.digests(&posts)?,
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
            let reading = self.joint.reading(&posts)?;
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
