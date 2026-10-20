// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-176, FR-177 (ADR-023 HC-1, HC-2): what the self-composition product and
//! the witness-set product share: the components' systems, the cache of joint
//! letters, the seams to the clause evaluator, the joint steps in canonical
//! order and the record of why an exploration stopped.

use std::cell::RefCell;
use std::collections::BTreeMap;

use qsl_foundation::digest::DigestRecord;
use qsl_foundation::{CatalogCode, IdentityLimits, InternalFault};
use qsl_semantics::check::FormulaNodeId;
use quire_exact::{Cancel, Meter, ScalarLimits};
use quire_semantic_value::model_transition::ModelTransition;

use crate::automaton::{Atom, Letter};
use crate::hyper::product::{ComponentStep, Halt, HyperTransition};
use crate::hyper::seam::{HyperLetters, MatchReading, ParameterBinding, StepMatch};
use crate::simulation::model::{ModelFinding, ModelSimState, ModelSystem};
use crate::simulation::{EncodingRefusal, ExpansionStop, TransitionSystem};
use crate::temporal::{AtomReading, UndefinedCause};

/// `resource_exhausted`/`insufficient-next-charge`, the code of an expansion
/// that stopped; [`Halt`] records which budget it was.
const STOPPED: CatalogCode = CatalogCode::new("resource_exhausted", "insufficient-next-charge");

/// One joint position's letter, or the first atom that has no value there.
#[derive(Clone, Debug)]
pub(super) enum Reading {
    Defined(Letter),
    Undefined {
        atom: FormulaNodeId,
        cause: UndefinedCause,
    },
}

/// What `μ` made of one joint step.
pub(super) enum Matched {
    Holds,
    Fails,
    Undefined {
        conjunct: usize,
        cause: UndefinedCause,
    },
    Refused,
}

/// A component's step options out of its expansion: each enabled transition
/// and its post-state, or the stutter step at a terminal component.
pub(super) type Options = Vec<(ComponentStep, ModelSimState)>;

/// One component's expansion, as a product reads it.
pub(super) struct Stepping {
    /// The component's step options.
    pub(super) options: Options,
    /// The transition identities its model state enables (ADR-018 FA-2),
    /// ascending and without repeats.
    pub(super) enabled: Vec<ModelTransition>,
    /// The contract conjunctions its expansion evaluated undefined or
    /// refused.
    pub(super) contracts: Vec<ModelFinding>,
}

/// The components' systems and the seams a product reads a joint position
/// through.
pub(super) struct Joint<'r, 's> {
    pub(super) systems: Vec<&'r ModelSystem<'s>>,
    instance: &'r [ParameterBinding],
    letters: &'r dyn HyperLetters,
    universal: Option<&'r dyn StepMatch>,
    existential: Option<&'r dyn StepMatch>,
    atoms: Vec<FormulaNodeId>,
    readings: RefCell<BTreeMap<Vec<DigestRecord>, Reading>>,
    meter: ScalarLimits,
    cancel: &'r Cancel,
    halt: RefCell<Option<Halt>>,
}

impl<'r, 's> Joint<'r, 's> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        systems: Vec<&'r ModelSystem<'s>>,
        instance: &'r [ParameterBinding],
        letters: &'r dyn HyperLetters,
        universal: Option<&'r dyn StepMatch>,
        existential: Option<&'r dyn StepMatch>,
        atoms: Vec<FormulaNodeId>,
        meter: ScalarLimits,
        cancel: &'r Cancel,
    ) -> Self {
        Self {
            systems,
            instance,
            letters,
            universal,
            existential,
            atoms,
            readings: RefCell::new(BTreeMap::new()),
            meter,
            cancel,
            halt: RefCell::new(None),
        }
    }

    /// Why the exploration stopped, once; `None` when it did not stop here.
    pub(super) fn take_halt(&self) -> Option<Halt> {
        self.halt.borrow_mut().take()
    }

    /// Record why the run stops, keeping the first reason, and return the stop
    /// the exploration engine takes.
    pub(super) fn record(&self, halt: Halt) -> ExpansionStop {
        let mut slot = self.halt.borrow_mut();
        if slot.is_none() {
            *slot = Some(halt);
        }
        ExpansionStop {
            cause: STOPPED,
            limit: None,
        }
    }

    pub(super) fn poll(&self) -> Result<(), ExpansionStop> {
        if self.cancel.poll() {
            if let Some(source) = self.cancel.cause() {
                return Err(self.record(Halt::Cancelled(source)));
            }
        }
        Ok(())
    }

    /// The FR-101 state-key digest of each component.
    pub(super) fn digests(
        &self,
        components: &[ModelSimState],
    ) -> Result<Vec<DigestRecord>, ExpansionStop> {
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
    pub(super) fn reading(&self, components: &[ModelSimState]) -> Result<Reading, ExpansionStop> {
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

    /// `μ_U` over the universal components' step `step`.
    pub(super) fn matched_universal(
        &self,
        step: &HyperTransition,
    ) -> Result<Matched, ExpansionStop> {
        let Some(matcher) = self.universal else {
            return Ok(Matched::Holds);
        };
        self.poll()?;
        let labels = step.labels();
        let mut meter = Meter::new(self.meter);
        Ok(read(matcher.universal(self.instance, &labels, &mut meter)))
    }

    /// `μ_E` over the joint step `step` of every component.
    pub(super) fn matched_existential(
        &self,
        step: &HyperTransition,
    ) -> Result<Matched, ExpansionStop> {
        let Some(matcher) = self.existential else {
            return Ok(Matched::Holds);
        };
        self.poll()?;
        let labels = step.labels();
        let mut meter = Meter::new(self.meter);
        Ok(read(matcher.existential(
            self.instance,
            &labels,
            &mut meter,
        )))
    }

    /// The expansion of `component` at `model`, with the stutter step at a
    /// terminal state (ADR-023 HM-5).
    pub(super) fn stepping(
        &self,
        component: usize,
        model: &ModelSimState,
    ) -> Result<Stepping, ExpansionStop> {
        let system = self.systems.get(component).ok_or_else(|| {
            self.record(Halt::Fault(InternalFault::new(
                "hyper-product",
                "a-component-has-a-system",
            )))
        })?;
        let expansion = system.successors(model).inspect_err(|&stop| {
            self.record(Halt::Expansion { component, stop });
        })?;
        let contracts = expansion
            .findings
            .into_iter()
            .filter(|finding| {
                finding.undefined_contract().is_some() || finding.is_refused_contract()
            })
            .collect();
        let mut enabled: Vec<ModelTransition> = expansion
            .successors
            .iter()
            .map(|(transition, _)| transition.clone())
            .collect();
        enabled.sort();
        enabled.dedup();
        let options = if expansion.successors.is_empty() {
            vec![(ComponentStep::Stutter, model.clone())]
        } else {
            expansion
                .successors
                .into_iter()
                .map(|(transition, post)| (ComponentStep::Model(transition), post))
                .collect()
        };
        Ok(Stepping {
            options,
            enabled,
            contracts,
        })
    }

    /// The joint steps over `options`, one option of each component, in the
    /// canonical order of their identities, each with the components'
    /// post-states.
    pub(super) fn joint_steps(
        &self,
        options: &[Options],
    ) -> Result<Vec<(HyperTransition, Vec<ModelSimState>)>, ExpansionStop> {
        let mut candidates: Vec<(Vec<u8>, HyperTransition, Vec<ModelSimState>)> = Vec::new();
        for choice in cartesian(options) {
            let (steps, posts): (Vec<ComponentStep>, Vec<ModelSimState>) =
                choice.into_iter().unzip();
            let step = HyperTransition(steps);
            let order = self.order(&step)?;
            candidates.push((order, step, posts));
        }
        candidates.sort_by(|left, right| left.0.cmp(&right.0));
        Ok(candidates
            .into_iter()
            .map(|(_, step, posts)| (step, posts))
            .collect())
    }

    /// The canonical encoding that orders a joint step among its siblings.
    fn order(&self, step: &HyperTransition) -> Result<Vec<u8>, ExpansionStop> {
        let identity = IdentityLimits::default();
        quire_canonical::to_vec(step, quire_canonical::Limits::new(identity.input_bytes))
            .map_err(|error| self.record(Halt::Encoding(EncodingRefusal::of(&error, identity))))
    }
}

fn read(reading: Result<MatchReading, quire_exact::Incomplete>) -> Matched {
    match reading {
        Ok(MatchReading::Holds) => Matched::Holds,
        Ok(MatchReading::Fails) => Matched::Fails,
        Ok(MatchReading::Undefined { conjunct, cause }) => Matched::Undefined { conjunct, cause },
        Ok(MatchReading::Refused) | Err(_) => Matched::Refused,
    }
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
