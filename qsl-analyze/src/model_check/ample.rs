// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-021 POR-5 to POR-10 (FR-156, FR-157): which transitions are visible,
//! the stubborn-set candidate each enabled transition seeds, and the choice
//! of an ample set at one product state.
//!
//! The choice is a function of the subject, the item, the discovered set
//! and the model state: [`Por::ample`] reads the model state, its enabled
//! transitions and which of them reach only undiscovered product states,
//! and nothing else. The engines that call it own the discovered set.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use qsl_eval::model_subject::{ModelInstance, ModelLetters};
use qsl_eval::simulation::model::{ModelSimState, ModelSystem};
use qsl_eval::simulation::{any_affects, independent, ExpansionStop, Footprint, Location};
use qsl_foundation::CatalogCode;
use qsl_semantics::check::footprint::SymbolicLocation;
use qsl_semantics::check::FormulaNodeId;
use quire_semantic_value::fairness::{FairnessConstraint, FairnessGranularity};
use quire_semantic_value::model_transition::ModelTransition;

use super::check::ModelCheckRefusal;
use super::liveness::LivenessRefusal;

/// `runtime_invariant`/`established-invariant-broken`.
const BROKEN: CatalogCode = CatalogCode::new("runtime_invariant", "established-invariant-broken");

/// The atoms of a clause and the seam that reads their footprints.
pub struct Atoms<'r> {
    /// The seam that reads each atom's footprint.
    pub letters: &'r dyn ModelLetters,
    /// The atoms, as the clause's automaton orders them.
    pub nodes: &'r [FormulaNodeId],
    /// Whether the clause's activation has a `when` condition, whose
    /// footprint the atoms' include.
    pub guarded: bool,
}

/// One fairness class of the item's fairness set (FR-157 "Fairness
/// classes"): the identities of one operation for `whole`, each of them
/// alone for `each`.
struct Class {
    operation: String,
    granularity: FairnessGranularity,
}

/// What a partial-order run needs of the item and the request, derived once.
pub struct Por<'r, 'a> {
    system: &'r ModelSystem<'a>,
    /// What the item's atoms read, with `over` as the receiver.
    atom_reads: BTreeSet<SymbolicLocation>,
    /// The instances the item's atoms are read for.
    instances: &'r [ModelInstance],
    /// What the state constraint reads.
    constraint_reads: BTreeSet<Location>,
    classes: Vec<Class>,
    footprints: RefCell<BTreeMap<ModelTransition, Rc<Footprint>>>,
}

impl<'r, 'a> Por<'r, 'a> {
    /// The inputs of ample-set choice for an item reading `atoms` for
    /// `instances` under `fairness` and the state constraint `constraint`.
    /// The deadlock-freedom item has no atoms (POR-5).
    ///
    /// # Errors
    ///
    /// [`ModelCheckRefusal::Footprint`] when an operation, an atom or the
    /// constraint reads through a form with no footprint rule, and
    /// [`LivenessRefusal::UnknownOperation`] for a fairness constraint that
    /// names an operation the subject does not declare.
    pub fn new(
        system: &'r ModelSystem<'a>,
        atoms: Option<Atoms<'_>>,
        instances: &'r [ModelInstance],
        fairness: &[FairnessConstraint],
        constraint: Option<&str>,
    ) -> Result<Self, ModelCheckRefusal> {
        system
            .check_footprints_derivable()
            .map_err(ModelCheckRefusal::Footprint)?;
        let mut atom_reads = BTreeSet::new();
        if let Some(atoms) = &atoms {
            for node in atoms.nodes {
                atom_reads.extend(
                    atoms
                        .letters
                        .reads(*node)
                        .map_err(|refusal| ModelCheckRefusal::Footprint(refusal.into()))?,
                );
            }
            if atoms.guarded {
                atom_reads.extend(
                    atoms
                        .letters
                        .activation_reads()
                        .map_err(|refusal| ModelCheckRefusal::Footprint(refusal.into()))?,
                );
            }
        }
        let constraint_reads = match constraint {
            Some(name) => system
                .constraint_reads(name)
                .map_err(ModelCheckRefusal::Footprint)?,
            None => BTreeSet::new(),
        };
        let mut classes = Vec::new();
        for constraint in fairness {
            let operation = constraint.operation();
            let text = system
                .operation_text(operation.declaring_type(), operation.name())
                .ok_or_else(|| LivenessRefusal::UnknownOperation {
                    name: operation.name().to_owned(),
                })?;
            classes.push(Class {
                operation: text.to_owned(),
                granularity: constraint.granularity(),
            });
        }
        Ok(Self {
            system,
            atom_reads,
            instances,
            constraint_reads,
            classes,
            footprints: RefCell::new(BTreeMap::new()),
        })
    }

    /// The footprint of `transition`, derived once (FR-162 `footprint`).
    fn footprint(&self, transition: &ModelTransition) -> Result<Rc<Footprint>, ExpansionStop> {
        if let Some(known) = self.footprints.borrow().get(transition) {
            return Ok(Rc::clone(known));
        }
        let footprint = Rc::new(
            qsl_eval::simulation::TransitionSystem::footprint(self.system, transition)
                .ok_or_else(|| self.broken())??,
        );
        self.footprints
            .borrow_mut()
            .insert(transition.clone(), Rc::clone(&footprint));
        Ok(footprint)
    }

    /// The stop of a broken invariant: a hook the system offers answered
    /// nothing, or a transition names no identity of the state (FR-162).
    fn broken(&self) -> ExpansionStop {
        ExpansionStop {
            cause: BROKEN,
            limit: None,
        }
    }

    /// FR-156 "Visibility" and FR-157 "Fairness visibility" at `state`:
    /// whether each of `all` is visible. A transition is visible when its
    /// writes meet a location an atom or the constraint reads, and, under a
    /// non-empty fairness set, when its writes meet the enabling footprint
    /// of a member of a class it does not belong to.
    fn visibility(
        &self,
        state: &ModelSimState,
        all: &[ModelTransition],
        footprints: &[Rc<Footprint>],
    ) -> Vec<bool> {
        let mut read: BTreeSet<Location> = self.constraint_reads.clone();
        for instance in self.instances {
            read.extend(
                self.system
                    .atom_locations(&self.atom_reads, instance, state),
            );
        }
        let mut visible: Vec<bool> = footprints
            .iter()
            .map(|footprint| any_affects(&footprint.writes, &read))
            .collect();
        for class in &self.classes {
            for (member, member_footprint) in all.iter().zip(footprints) {
                if member.operation() != class.operation {
                    continue;
                }
                for (index, (other, other_footprint)) in all.iter().zip(footprints).enumerate() {
                    let outside = match class.granularity {
                        FairnessGranularity::Whole => other.operation() != class.operation,
                        FairnessGranularity::Each => other != member,
                    };
                    if outside && any_affects(&other_footprint.writes, &member_footprint.enabling) {
                        visible[index] = true;
                    }
                }
            }
        }
        visible
    }

    /// FR-156 "Visibility" and FR-157 "Fairness visibility": whether
    /// `transition` is visible at `state`. A test hook.
    ///
    /// # Errors
    ///
    /// An [`ExpansionStop`] when the state's identities cannot be listed, a
    /// transition has no footprint, or `transition` is no identity of
    /// `state`.
    #[cfg(feature = "test-support")]
    pub fn visible(
        &self,
        state: &ModelSimState,
        transition: &ModelTransition,
    ) -> Result<bool, ExpansionStop> {
        let (all, footprints) = self.identities(state)?;
        let at = all
            .iter()
            .position(|held| held == transition)
            .ok_or_else(|| self.broken())?;
        Ok(self.visibility(state, &all, &footprints)[at])
    }

    /// FR-156 "Candidates": the identities of the candidate seeded by the
    /// enabled `seed` at `state` (`enabled` ascending), in canonical order.
    ///
    /// # Errors
    ///
    /// As [`Self::visible`].
    #[cfg(feature = "test-support")]
    pub fn candidate_seeded_by(
        &self,
        state: &ModelSimState,
        enabled: &[ModelTransition],
        seed: &ModelTransition,
    ) -> Result<Vec<ModelTransition>, ExpansionStop> {
        let (all, footprints) = self.identities(state)?;
        let at = all
            .iter()
            .position(|held| held == seed)
            .ok_or_else(|| self.broken())?;
        let is_enabled = Self::enabled_flags(&all, enabled);
        let members = Self::candidate(at, &is_enabled, &footprints);
        Ok(all
            .into_iter()
            .zip(members)
            .filter_map(|(transition, member)| member.then_some(transition))
            .collect())
    }

    /// Every identity of `state` with its footprint.
    fn identities(
        &self,
        state: &ModelSimState,
    ) -> Result<(Vec<ModelTransition>, Vec<Rc<Footprint>>), ExpansionStop> {
        let all = self.system.transition_identities(state)?;
        let footprints = all
            .iter()
            .map(|transition| self.footprint(transition))
            .collect::<Result<Vec<_>, _>>()?;
        Ok((all, footprints))
    }

    fn enabled_flags(all: &[ModelTransition], enabled: &[ModelTransition]) -> Vec<bool> {
        all.iter()
            .map(|transition| enabled.binary_search(transition).is_ok())
            .collect()
    }

    /// FR-156 "Candidates": the identities, as indices of `all`, of the
    /// candidate seeded by `seed`. Each enabled member brings every identity
    /// dependent on it, each disabled member every identity whose writes
    /// meet its enabling footprint.
    fn candidate(seed: usize, enabled: &[bool], footprints: &[Rc<Footprint>]) -> Vec<bool> {
        let mut members = vec![false; footprints.len()];
        members[seed] = true;
        let mut pending = vec![seed];
        while let Some(member) = pending.pop() {
            for (other, other_footprint) in footprints.iter().enumerate() {
                if members[other] {
                    continue;
                }
                let brought = if enabled[member] {
                    !independent(&footprints[member], other_footprint)
                } else {
                    any_affects(&other_footprint.writes, &footprints[member].enabling)
                };
                if brought {
                    members[other] = true;
                    pending.push(other);
                }
            }
        }
        members
    }

    /// FR-156 "Choice": `A(s)` for the model state `state` whose enabled
    /// transitions are `enabled`, ascending and without repeats. `fresh`
    /// says whether every product successor through a transition is a
    /// product state not yet discovered (C3). The ample set is the enabled
    /// part of the candidate with the fewest enabled members that satisfies
    /// C2 and C3, ties broken by the seed's canonical order, or `enabled`
    /// when none does.
    ///
    /// # Errors
    ///
    /// An [`ExpansionStop`] when the state's identities cannot be listed or
    /// a transition has no footprint.
    pub fn ample(
        &self,
        state: &ModelSimState,
        enabled: &[ModelTransition],
        fresh: &dyn Fn(&ModelTransition) -> bool,
    ) -> Result<Vec<ModelTransition>, ExpansionStop> {
        // C0, and a singleton is its own ample set.
        if enabled.len() <= 1 {
            return Ok(enabled.to_vec());
        }
        let (all, footprints) = self.identities(state)?;
        let visible = self.visibility(state, &all, &footprints);
        let is_enabled = Self::enabled_flags(&all, enabled);
        let mut best: Option<Vec<usize>> = None;
        for seed in 0..all.len() {
            if !is_enabled[seed] {
                continue;
            }
            let members = Self::candidate(seed, &is_enabled, &footprints);
            let part: Vec<usize> = (0..all.len())
                .filter(|index| members[*index] && is_enabled[*index])
                .collect();
            if part.len() >= enabled.len()
                || best.as_ref().is_some_and(|held| held.len() <= part.len())
            {
                continue;
            }
            if part
                .iter()
                .all(|index| !visible[*index] && fresh(&all[*index]))
            {
                best = Some(part);
            }
        }
        Ok(match best {
            Some(part) => part.into_iter().map(|index| all[index].clone()).collect(),
            None => enabled.to_vec(),
        })
    }
}
