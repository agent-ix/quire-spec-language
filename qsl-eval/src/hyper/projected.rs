// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-178 (ADR-023 PA-2 to PA-5; ADR-011 §6.1 layer 5): the projected product
//! of a projection-aligned universal hyperproperty (HP-6), which
//! `qsl-analyze`'s EN-1 explores with the same breadth-first engine as the
//! self-composition product (ADR-018 LA-2).
//!
//! A product state is the tuple of the components' model states with the
//! state of the body's deterministic safety monitor ([`Monitor`], ADR-018
//! SM-6) after it read the joint projected prefix, keyed as
//! [`HyperProduct`](crate::hyper::HyperProduct) keys a state. The initial
//! states are the same tuples of initial states, the monitor reading joint
//! position 0.
//!
//! A step is **skipped** when its operation is in the clause's `align skip`
//! list and **visible** otherwise (PA-2). There are two kinds of move:
//!
//! - A **skip move**: one component takes an enabled skipped step, every other
//!   component stays ([`ComponentStep::Stay`]) and so does the monitor. No
//!   atom and no `match` conjunct reads it.
//! - A **visible move**: every component takes an enabled visible step
//!   together when `μ_U` holds on the joint step, the monitor reading the joint
//!   post-position.
//!
//! A component that is terminal, or all of whose steps are skipped, has no
//! visible step, so no visible move follows and its path takes part in the
//! prefixes up to its last visible step (PA-3); there is no stutter step here.
//!
//! The states that end the first phase are states of their own, so the
//! engine's canonical breadth-first order ranks them (ADR-023 HV-8): a monitor
//! in its rejecting state ([`Property::Rejected`]), a joint position whose
//! letter has an atom with no value ([`Property::Undefined`]) and a visible
//! move on which `μ_U` has no value ([`Property::MatchUndefined`]). No move
//! follows any of them. A move on which `μ_U` is refused or incomplete is no
//! successor: it is recorded as [`HyperFinding::MatchRefused`].
//!
//! The expansion records why it stops in [`Halt`], as the other products do.

use std::cell::{Ref, RefCell};
use std::collections::BTreeSet;

use qsl_semantics::check::{CheckedFormula, OperationKey, TemporalProfile};
use quire_exact::{Cancel, ScalarLimits};
use quire_semantic_value::catalog::InternalFault;
use quire_semantic_value::model_transition::ModelTransition;

use crate::automaton::{translate_body, AutomatonError, Monitor, PropertyAutomaton, StateId};
use crate::hyper::joint::{cartesian, Joint, Matched, Options, Reading, Seams};
use crate::hyper::product::{
    ComponentStep, Halt, HyperFinding, HyperKey, HyperState, HyperTransition, Property,
};
use crate::hyper::seam::{HyperLetters, ParameterBinding, StepMatch};
use crate::simulation::model::{ModelSimState, ModelSystem};
use crate::simulation::{Expansion, ExpansionStop, TransitionSystem};

/// What [`ProjectedProduct::new`] reads.
pub struct ProjectedInputs<'r, 's> {
    /// The `ModelSystem` of each trace variable's subject, in quantifier
    /// order; two variables of one alias name the same system.
    pub systems: Vec<&'r ModelSystem<'s>>,
    /// The clause instance: each object parameter's key.
    pub instance: &'r [ParameterBinding],
    /// The body's formula, which must be in the safety fragment.
    pub body: &'r CheckedFormula,
    /// The profile the body was checked under.
    pub profile: TemporalProfile,
    /// The operations whose steps are skipped (ADR-023 PA-1): each is skipped
    /// in every trace of the alias whose model declares it.
    pub align_skip: &'r [OperationKey],
    /// The seam to the body's atoms.
    pub letters: &'r dyn HyperLetters,
    /// The seam to `μ_U`; `None` when the clause has no universal conjunct.
    pub matcher: Option<&'r dyn StepMatch>,
    /// FR-126's `max_automaton_states`.
    pub max_automaton_states: u64,
    /// The budget of each atom or `μ_U` evaluation.
    pub meter: ScalarLimits,
    /// The run's cancellation handle.
    pub cancel: &'r Cancel,
}

/// The projected product of a hyper clause's subjects as a
/// [`TransitionSystem`] (ADR-023 PA-4).
pub struct ProjectedProduct<'r, 's> {
    joint: Joint<'r, 's>,
    monitor: RefCell<Monitor<PropertyAutomaton>>,
    /// The texts of the skipped operations of each component's subject.
    skips: Vec<BTreeSet<String>>,
    initial: Vec<HyperState>,
}

impl<'r, 's> ProjectedProduct<'r, 's> {
    /// Translate the body to its safety automaton and bind the product to its
    /// inputs. The initial states are read by [`Self::start`].
    ///
    /// # Errors
    ///
    /// [`AutomatonError`] when the body has no safety automaton, or
    /// `max_automaton_states` admits no state.
    pub fn new(inputs: ProjectedInputs<'r, 's>) -> Result<Self, AutomatonError> {
        let automaton = translate_body(inputs.body, inputs.profile, inputs.max_automaton_states)?;
        let atoms = automaton.atoms().to_vec();
        let monitor = Monitor::bad_prefix(automaton, inputs.max_automaton_states)?;
        let skips = inputs
            .systems
            .iter()
            .map(|system| {
                inputs
                    .align_skip
                    .iter()
                    .filter_map(|key| system.operation_text_of(key))
                    .map(str::to_owned)
                    .collect()
            })
            .collect();
        Ok(Self {
            joint: Joint::new(
                inputs.systems,
                inputs.instance,
                Seams {
                    letters: inputs.letters,
                    universal: inputs.matcher,
                    existential: None,
                },
                atoms,
                inputs.meter,
                inputs.cancel,
            ),
            monitor: RefCell::new(monitor),
            skips,
            initial: Vec::new(),
        })
    }

    /// Read the initial product states: every tuple of initial states, each
    /// component from its own subject's list, with the monitor reading joint
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
            let property = self.read(None, &reading)?;
            initial.push(HyperState {
                components,
                property,
            });
        }
        self.initial = initial;
        Ok(())
    }

    /// Why the exploration stopped, once; `None` when it did not stop here.
    pub fn take_halt(&self) -> Option<Halt> {
        self.joint.take_halt()
    }

    /// The safety automaton: the states it has reached.
    pub fn automaton(&self) -> Ref<'_, PropertyAutomaton> {
        Ref::map(self.monitor.borrow(), Monitor::automaton)
    }

    /// Whether `transition`, a step of `component`, is skipped.
    pub fn skipped(&self, component: usize, transition: &ModelTransition) -> bool {
        self.skips
            .get(component)
            .is_some_and(|skips| skips.contains(transition.operation()))
    }

    /// The property state a joint projected position reads into, from the
    /// monitor state `from` (the initial one for position 0).
    fn read(&self, from: Option<StateId>, reading: &Reading) -> Result<Property, ExpansionStop> {
        let letter = match reading {
            Reading::Undefined { atom, cause } => {
                return Ok(Property::Undefined {
                    atom: *atom,
                    cause: cause.clone(),
                    partner: None,
                })
            }
            Reading::Defined(letter) => *letter,
        };
        let mut monitor = self.monitor.borrow_mut();
        let from = match from {
            Some(from) => from,
            None => monitor.initial().map_err(|error| self.stop(error))?,
        };
        let next = monitor
            .step(from, letter)
            .map_err(|error| self.stop(error))?;
        Ok(
            if monitor
                .is_rejecting(next)
                .map_err(|error| self.stop(error))?
            {
                Property::Rejected
            } else {
                Property::After(next)
            },
        )
    }

    fn stop(&self, error: AutomatonError) -> ExpansionStop {
        self.joint.record(match error {
            AutomatonError::StateLimit { limit } => Halt::Automaton(limit),
            _ => Halt::Fault(InternalFault::new(
                "projected-product",
                "a-reached-monitor-state-steps",
            )),
        })
    }
}

impl TransitionSystem for ProjectedProduct<'_, '_> {
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

    fn successors(
        &self,
        state: &HyperState,
    ) -> Result<Expansion<HyperTransition, HyperState, HyperFinding>, ExpansionStop> {
        self.joint.poll()?;
        let monitor = match &state.property {
            // The first phase ends at this state; nothing follows it.
            Property::Undefined { .. } | Property::Rejected | Property::MatchUndefined { .. } => {
                return Ok(Expansion {
                    successors: Vec::new(),
                    findings: Vec::new(),
                })
            }
            Property::After(at) => *at,
            Property::Dead | Property::Witnesses(_) => {
                return Err(self.joint.record(Halt::Fault(InternalFault::new(
                    "projected-product",
                    "a-projected-state-has-a-monitor-state",
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
            // A terminal component's stutter step is no step of a projected
            // path (ADR-023 PA-3).
            options.push(
                stepping
                    .options
                    .into_iter()
                    .filter(|(step, _)| matches!(step, ComponentStep::Model(_)))
                    .collect(),
            );
        }
        let mut successors = Vec::new();
        // The skip moves: one component steps, the others and the monitor stay.
        for (component, moves) in options.iter().enumerate() {
            for (step, post) in moves {
                let ComponentStep::Model(transition) = step else {
                    continue;
                };
                if !self.skipped(component, transition) {
                    continue;
                }
                let mut components = state.components.clone();
                components[component] = post.clone();
                let mut joint = vec![ComponentStep::Stay; components.len()];
                joint[component] = step.clone();
                successors.push((
                    HyperTransition(joint),
                    HyperState {
                        components,
                        property: state.property.clone(),
                    },
                ));
            }
        }
        // The visible moves: every component takes a visible step together.
        let visible: Vec<Options> = options
            .iter()
            .enumerate()
            .map(|(component, moves)| {
                moves
                    .iter()
                    .filter(|(step, _)| {
                        matches!(step, ComponentStep::Model(transition)
                            if !self.skipped(component, transition))
                    })
                    .cloned()
                    .collect()
            })
            .collect();
        for (step, posts) in self.joint.joint_steps(&visible)? {
            let property = match self.joint.matched_universal(&step)? {
                Matched::Fails => continue,
                Matched::Holds => {
                    let reading = self.joint.reading(&posts)?;
                    self.read(Some(monitor), &reading)?
                }
                Matched::Undefined { conjunct, cause } => {
                    Property::MatchUndefined { conjunct, cause }
                }
                Matched::Refused => {
                    findings.push(HyperFinding::MatchRefused);
                    continue;
                }
            };
            successors.push((
                step,
                HyperState {
                    components: posts,
                    property,
                },
            ));
        }
        Ok(Expansion {
            successors,
            findings,
        })
    }
}
