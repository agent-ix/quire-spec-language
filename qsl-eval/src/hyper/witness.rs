// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-177 (ADR-023 HC-2; ADR-020 AX-2; ADR-011 §6.1 layer 5): the witness-set
//! product of a forall-exists safety hyperproperty (HP-3), which `qsl-analyze`'s
//! EN-1 explores and the layer-6 checkers (FR-163, FR-183) recompute with the
//! same code (ADR-018 LA-2).
//!
//! A product state is the tuple of the universal components' model states with
//! the set `X` of the (existential tuple, safety automaton state) pairs still
//! consistent with the universal run so far ([`WitnessSet`]). The safety
//! automaton is the automaton of the body itself with every state of empty
//! language removed ([`Monitor::viable_successors`]); no Büchi complementation
//! is needed. The initial states are each initial universal tuple with `X_0`,
//! every (initial existential tuple, automaton state reached by reading joint
//! position 0). A successor is a universal joint step on which `μ_U` holds;
//! its `X'` holds every (existential post-tuple, automaton successor) reached
//! from a member of `X` by an existential joint step on which `μ_E` holds
//! against that universal step, the automaton reading the joint post-position.
//! An empty `X` is absorbing. `X` is stored as the sequence of its members
//! sorted by their identity, so equal sets coalesce.
//!
//! Each member remembers how its existential tuple was reached ([`Partner`]),
//! so a counterexample that ends at an undefined evaluation can name one path
//! per existential variable. The partner is not part of a member's identity:
//! the state a product keeps for a key is the one first discovered, whose
//! members' partners are the paths to it.
//!
//! A letter with an atom that has no value at a member's joint position makes
//! the state it is read into [`Property::Undefined`], carrying the member's
//! partner; no step follows it. The expansion records why it stops in
//! [`Halt`], as [`HyperProduct`](crate::hyper::HyperProduct) does, a witness
//! set that would pass `max_witness_set` (FR-184) included.

use std::cell::{Ref, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use qsl_foundation::digest::DigestRecord;
use qsl_foundation::InternalFault;
use qsl_semantics::check::{CheckedFormula, FormulaNodeId, TemporalProfile};
use quire_exact::{Cancel, ScalarLimits};

use crate::automaton::{
    translate_body, AutomatonError, Letter, Monitor, PropertyAutomaton, StateId,
};
use crate::hyper::joint::{cartesian, Joint, Matched, Options, Reading};
use crate::hyper::product::{
    ComponentStep, Halt, HyperFinding, HyperKey, HyperState, HyperTransition, Property,
};
use crate::hyper::seam::{HyperLetters, ParameterBinding, StepMatch};
use crate::simulation::model::{ModelSimState, ModelSystem};
use crate::simulation::{Expansion, ExpansionStop, TransitionSystem};
use crate::temporal::UndefinedCause;

/// How an existential tuple was reached: the existential half of one joint
/// behaviour.
#[derive(Debug, Eq, PartialEq)]
pub enum Partner {
    /// An initial tuple.
    Initial {
        /// The index of each existential component's initial state among its
        /// subject's initial states, in quantifier order.
        indices: Vec<u32>,
        /// The digest of each existential component's state.
        digests: Vec<DigestRecord>,
    },
    /// A tuple reached from another by an existential joint step.
    Step {
        /// The tuple the step left.
        before: Rc<Partner>,
        /// The existential components' steps, in quantifier order.
        step: Vec<ComponentStep>,
        /// The digest of each existential component's post-state.
        posts: Vec<DigestRecord>,
    },
}

impl Partner {
    /// The digest of each existential component's state, in quantifier order.
    pub fn digests(&self) -> &[DigestRecord] {
        match self {
            Self::Initial { digests, .. } => digests,
            Self::Step { posts, .. } => posts,
        }
    }

    /// The initial indices the path starts from and its steps in order, each
    /// with the digests of the existential post-states.
    pub fn path(&self) -> (&[u32], Vec<PartnerStep<'_>>) {
        let mut steps = Vec::new();
        let mut cursor = self;
        loop {
            match cursor {
                Self::Initial { indices, .. } => {
                    steps.reverse();
                    return (indices, steps);
                }
                Self::Step {
                    before,
                    step,
                    posts,
                } => {
                    steps.push((step.as_slice(), posts.as_slice()));
                    cursor = before;
                }
            }
        }
    }
}

/// One step of a [`Partner`]'s path: the existential components' steps and
/// the digest of each one's post-state.
pub type PartnerStep<'a> = (&'a [ComponentStep], &'a [DigestRecord]);

/// One member of a witness set: an existential tuple of model states and the
/// safety automaton state it is in.
#[derive(Clone, Debug)]
pub struct Member {
    states: Vec<ModelSimState>,
    automaton: StateId,
    partner: Rc<Partner>,
}

impl Member {
    /// The existential components' model states, in quantifier order.
    pub fn states(&self) -> &[ModelSimState] {
        &self.states
    }

    /// The safety automaton state.
    pub fn automaton(&self) -> StateId {
        self.automaton
    }

    /// How the existential tuple was reached.
    pub fn partner(&self) -> &Rc<Partner> {
        &self.partner
    }

    fn identity(&self) -> (&[DigestRecord], usize) {
        (self.partner.digests(), self.automaton.index())
    }
}

/// A witness set `X`: its members sorted by identity.
#[derive(Clone, Debug)]
pub struct WitnessSet {
    members: Vec<Member>,
}

impl WitnessSet {
    /// The members, sorted.
    pub fn members(&self) -> &[Member] {
        &self.members
    }

    /// Whether no member is left.
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }
}

impl PartialEq for WitnessSet {
    fn eq(&self, other: &Self) -> bool {
        self.members.len() == other.members.len()
            && self
                .members
                .iter()
                .zip(&other.members)
                .all(|(left, right)| left.identity() == right.identity())
    }
}

impl Eq for WitnessSet {}

/// What [`WitnessProduct::new`] reads.
pub struct WitnessInputs<'r, 's> {
    /// The `ModelSystem` of each trace variable's subject, in quantifier
    /// order: the universal variables', then the existential ones'.
    pub systems: Vec<&'r ModelSystem<'s>>,
    /// How many of the leading variables are universal.
    pub universal: usize,
    /// The clause instance: each object parameter's key.
    pub instance: &'r [ParameterBinding],
    /// The body's formula, which must be in the safety fragment.
    pub body: &'r CheckedFormula,
    /// The profile the body was checked under.
    pub profile: TemporalProfile,
    /// The seam to the body's atoms.
    pub letters: &'r dyn HyperLetters,
    /// The seam to `μ_U`; `None` when the clause has no universal conjunct.
    pub universal_match: Option<&'r dyn StepMatch>,
    /// The seam to `μ_E`; `None` when the clause has no existential conjunct.
    pub existential_match: Option<&'r dyn StepMatch>,
    /// FR-126's `max_automaton_states`.
    pub max_automaton_states: u64,
    /// FR-184's `max_witness_set`.
    pub max_witness_set: u64,
    /// The budget of each atom or `μ` evaluation.
    pub meter: ScalarLimits,
    /// The run's cancellation handle.
    pub cancel: &'r Cancel,
}

/// The members a successor's `X'` has collected, by identity.
type Collected = BTreeMap<(Vec<DigestRecord>, usize), Member>;

/// The first atom of a joint post-position's letter that has no value, why,
/// and the partner whose position it was read at.
type Unread = (FormulaNodeId, UndefinedCause, Rc<Partner>);

/// The witness-set product of a hyper clause's subjects as a
/// [`TransitionSystem`] (ADR-023 HC-2).
pub struct WitnessProduct<'r, 's> {
    joint: Joint<'r, 's>,
    universal: usize,
    monitor: RefCell<Monitor<PropertyAutomaton>>,
    max_witness_set: u64,
    initial: Vec<HyperState>,
}

impl<'r, 's> WitnessProduct<'r, 's> {
    /// Translate the body to its safety automaton and bind the product to its
    /// inputs. The initial states are read by [`Self::start`].
    ///
    /// # Errors
    ///
    /// [`AutomatonError`] when the body has no safety automaton, or
    /// `max_automaton_states` admits no state.
    pub fn new(inputs: WitnessInputs<'r, 's>) -> Result<Self, AutomatonError> {
        let automaton = translate_body(inputs.body, inputs.profile, inputs.max_automaton_states)?;
        let atoms = automaton.atoms().to_vec();
        let monitor = Monitor::bad_prefix(automaton, inputs.max_automaton_states)?;
        Ok(Self {
            joint: Joint::new(
                inputs.systems,
                inputs.instance,
                inputs.letters,
                inputs.universal_match,
                inputs.existential_match,
                atoms,
                inputs.meter,
                inputs.cancel,
            ),
            universal: inputs.universal,
            monitor: RefCell::new(monitor),
            max_witness_set: inputs.max_witness_set,
            initial: Vec::new(),
        })
    }

    /// Read the initial product states: every tuple of initial universal
    /// states with `X_0`, the existential tuples the safety automaton accepts
    /// at joint position 0.
    ///
    /// # Errors
    ///
    /// The stop the exploration engine takes; [`Self::take_halt`] says why.
    pub fn start(&mut self) -> Result<(), ExpansionStop> {
        let (universal, existential) = self.joint.systems.split_at(self.universal);
        let universal: Vec<Vec<ModelSimState>> =
            universal.iter().map(|system| system.initial()).collect();
        let existential: Vec<Vec<(u32, ModelSimState)>> = existential
            .iter()
            .map(|system| {
                system
                    .initial()
                    .into_iter()
                    .enumerate()
                    .map(|(index, state)| (u32::try_from(index).unwrap_or(u32::MAX), state))
                    .collect()
            })
            .collect();
        let mut initial = Vec::new();
        for components in cartesian(&universal) {
            let mut members = Collected::new();
            let mut undefined = None;
            for tuple in cartesian(&existential) {
                let (indices, states): (Vec<u32>, Vec<ModelSimState>) = tuple.into_iter().unzip();
                let mut joint = components.clone();
                joint.extend(states.iter().cloned());
                let digests = self.joint.digests(&joint)?;
                let partner = Rc::new(Partner::Initial {
                    indices,
                    digests: digests[self.universal..].to_vec(),
                });
                let letter = match self.joint.reading(&joint)? {
                    Reading::Defined(letter) => letter,
                    Reading::Undefined { atom, cause } => {
                        undefined.get_or_insert((atom, cause, partner));
                        continue;
                    }
                };
                let Some(start) = self.monitor_viable_initial()? else {
                    continue;
                };
                for automaton in self.viable_successors(start, letter)? {
                    self.admit(
                        &mut members,
                        Member {
                            states: states.clone(),
                            automaton,
                            partner: partner.clone(),
                        },
                    )?;
                }
            }
            let property = match undefined {
                Some((atom, cause, partner)) => Property::Undefined {
                    atom,
                    cause,
                    partner: Some(partner),
                },
                None => Property::Witnesses(WitnessSet {
                    members: members.into_values().collect(),
                }),
            };
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

    /// The `ModelSystem` of each trace variable, in quantifier order.
    pub fn systems(&self) -> &[&'r ModelSystem<'s>] {
        &self.joint.systems
    }

    fn monitor_viable_initial(&self) -> Result<Option<StateId>, ExpansionStop> {
        self.monitor
            .borrow_mut()
            .viable_initial()
            .map_err(|error| self.automaton_stop(error))
    }

    fn viable_successors(
        &self,
        from: StateId,
        letter: Letter,
    ) -> Result<Vec<StateId>, ExpansionStop> {
        self.monitor
            .borrow_mut()
            .viable_successors(from, letter)
            .map_err(|error| self.automaton_stop(error))
    }

    fn automaton_stop(&self, error: AutomatonError) -> ExpansionStop {
        self.joint.record(match error {
            AutomatonError::StateLimit { limit } => Halt::Automaton(limit),
            _ => Halt::Fault(InternalFault::new(
                "witness-product",
                "a-reached-automaton-state-expands",
            )),
        })
    }

    /// Add `member` to `collected` unless its identity is already there,
    /// stopping the run when the set would pass `max_witness_set` (FR-184).
    fn admit(&self, collected: &mut Collected, member: Member) -> Result<(), ExpansionStop> {
        let identity = (member.partner.digests().to_vec(), member.automaton.index());
        if collected.contains_key(&identity) {
            return Ok(());
        }
        let count = u64::try_from(collected.len())
            .ok()
            .and_then(|held| held.checked_add(1))
            .ok_or_else(|| {
                self.joint.record(Halt::Fault(InternalFault::new(
                    "witness-product",
                    "a-witness-set-counts",
                )))
            })?;
        if count > self.max_witness_set {
            return Err(self.joint.record(Halt::WitnessSet(count)));
        }
        collected.insert(identity, member);
        Ok(())
    }

    /// `X'` for the universal joint step `step` with post-states `posts`: the
    /// members of `members` carried over every existential joint step on which
    /// `μ_E` holds, with the findings the evaluation made on the way.
    fn carry(
        &self,
        members: &WitnessSet,
        step: &HyperTransition,
        posts: &[ModelSimState],
        findings: &mut Vec<HyperFinding>,
    ) -> Result<(Collected, Option<Unread>), ExpansionStop> {
        let mut next = Collected::new();
        let mut undefined = None;
        for member in members.members() {
            let mut options: Vec<Options> = Vec::with_capacity(member.states.len());
            let mut known = true;
            for (offset, model) in member.states.iter().enumerate() {
                let stepping = self.joint.stepping(self.universal + offset, model)?;
                known &= stepping.contracts.is_empty();
                options.push(stepping.options);
            }
            if !known {
                findings.push(HyperFinding::MemberContract);
                continue;
            }
            for (existential, eposts) in self.joint.joint_steps(&options)? {
                let mut taken = step.0.clone();
                taken.extend(existential.0.iter().cloned());
                let joint_step = HyperTransition(taken);
                let mut joint_posts = posts.to_vec();
                joint_posts.extend(eposts.iter().cloned());
                match self.joint.matched_existential(&joint_step)? {
                    Matched::Holds => {}
                    Matched::Fails => continue,
                    Matched::Undefined { conjunct, cause } => {
                        findings.push(HyperFinding::ExistentialMatchUndefined {
                            posts: self.joint.digests(&joint_posts)?,
                            step: joint_step,
                            conjunct,
                            cause,
                            partner: member.partner.clone(),
                        });
                        continue;
                    }
                    Matched::Refused => {
                        findings.push(HyperFinding::MatchRefused);
                        continue;
                    }
                }
                let digests = self.joint.digests(&joint_posts)?;
                let partner = Rc::new(Partner::Step {
                    before: member.partner.clone(),
                    step: existential.0,
                    posts: digests[self.universal..].to_vec(),
                });
                let letter = match self.joint.reading(&joint_posts)? {
                    Reading::Defined(letter) => letter,
                    Reading::Undefined { atom, cause } => {
                        undefined.get_or_insert((atom, cause, partner));
                        continue;
                    }
                };
                for automaton in self.viable_successors(member.automaton, letter)? {
                    self.admit(
                        &mut next,
                        Member {
                            states: eposts.clone(),
                            automaton,
                            partner: partner.clone(),
                        },
                    )?;
                }
            }
        }
        Ok((next, undefined))
    }
}

impl TransitionSystem for WitnessProduct<'_, '_> {
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
        let members = match &state.property {
            // The first phase ends at this state; nothing follows it.
            Property::Undefined { .. } => {
                return Ok(Expansion {
                    successors: Vec::new(),
                    findings: Vec::new(),
                })
            }
            Property::Witnesses(members) => members,
            Property::After(_) | Property::Dead => {
                return Err(self.joint.record(Halt::Fault(InternalFault::new(
                    "witness-product",
                    "a-witness-state-has-a-witness-set",
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
        let mut successors = Vec::new();
        for (step, posts) in self.joint.joint_steps(&options)? {
            match self.joint.matched_universal(&step)? {
                Matched::Holds => {}
                Matched::Fails => continue,
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
            let (next, undefined) = self.carry(members, &step, &posts, &mut findings)?;
            successors.push((
                step.clone(),
                HyperState {
                    components: posts.clone(),
                    property: Property::Witnesses(WitnessSet {
                        members: next.into_values().collect(),
                    }),
                },
            ));
            if let Some((atom, cause, partner)) = undefined {
                successors.push((
                    step,
                    HyperState {
                        components: posts,
                        property: Property::Undefined {
                            atom,
                            cause,
                            partner: Some(partner),
                        },
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
