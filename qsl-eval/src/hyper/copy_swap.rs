// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-174 (ADR-023 CS-1 to CS-3): the canonicaliser that halves a
//! self-composition product whose clause is symmetric in its copies.
//!
//! S3 finds the classes of universal variables that can be swapped without
//! changing the clause (the [`CopyGroup`]). The product builds the body's
//! automaton from its normal form, in which an `and` or an `or` is the same
//! node however its operands were written, so exchanging two variables
//! permutes the automaton's states: it renames the atoms each state's
//! formulas read. A swap that moves one of the automaton's acceptance sets to
//! another is not an automorphism of its acceptance condition, and the
//! product then offers no reduction. The
//! canonical form of a product state is the member of its orbit under the
//! copy group whose key is least: the components' keys in quantifier order,
//! then the automaton state's index, as [`HyperKey`](super::HyperKey) encodes
//! them. The group is the product of the symmetric groups of the classes, and
//! the orbit is reached by applying one exchange of two consecutive variables
//! of a class at a time.
//!
//! A [`SymmetryGroup`] names the group a run reduces by, as ADR-021 EI-3's
//! key symmetry does: its classes are classes of trace variables, held by
//! the pseudo-population [`copy_population`], and a [`Permutation`] of it
//! names the variable each component's value moved to.

use std::collections::BTreeSet;

use qsl_foundation::{IdentityLimits, InternalFault};
use qsl_semantics::check::{CopyGenerator, CopyGroup, FormulaNodeId};
use qsl_semantics::model::key::DeclarationKey;

use crate::automaton::AutomatonError;
use crate::hyper::product::{Halt, HyperKey, HyperProduct, HyperState, Property};
use crate::simulation::{
    EncodingRefusal, ExpansionStop, Permutation, SymmetryClass, SymmetryGroup,
};

/// The population whose "keys" are a clause's trace variables.
pub fn copy_population() -> DeclarationKey {
    DeclarationKey {
        package: "qsl/copy-swap".to_owned(),
        node: "ix://qsl/copy-swap/trace-variables".to_owned(),
    }
}

/// The symmetry group of `copy`: one class of trace variables for each class
/// of swappable variables.
pub fn copy_symmetry(copy: &CopyGroup) -> SymmetryGroup {
    SymmetryGroup {
        classes: copy
            .classes
            .iter()
            .map(|class| {
                let mut keys = class.clone();
                keys.sort();
                SymmetryClass {
                    population: copy_population(),
                    keys,
                }
            })
            .collect(),
    }
}

/// What [`HyperProduct::new`] reads to canonicalise under the copy group.
pub struct CopySwapInputs<'r> {
    /// The clause's trace variables, in quantifier order.
    pub variables: Vec<&'r str>,
    /// The copy group the product reduces by: the clause's (FR-174 Outputs),
    /// or the classes of it the caller reduces by.
    pub group: &'r CopyGroup,
}

/// The copy group a product canonicalises under.
pub(super) struct CopySwap {
    variables: Vec<String>,
    generators: Vec<CopyGenerator>,
}

impl CopySwap {
    pub(super) fn new(inputs: &CopySwapInputs<'_>) -> Self {
        Self {
            variables: inputs.variables.iter().map(|v| (*v).to_owned()).collect(),
            generators: inputs.group.generators.clone(),
        }
    }
}

/// A member of an orbit: the state, its key, and where its components came
/// from. Component `k` of the state is component `positions[k]` of the first
/// member.
struct Member {
    state: HyperState,
    key: Vec<u8>,
    positions: Vec<usize>,
}

impl Member {
    /// This member after `generator` exchanges its two variables.
    fn after(&self, generator: &CopyGenerator, state: HyperState, key: Vec<u8>) -> Self {
        let mut positions = self.positions.clone();
        positions.swap(generator.first, generator.second);
        Self {
            state,
            key,
            positions,
        }
    }
}

impl HyperProduct<'_, '_> {
    /// The canonical form of `state` under the copy group `group` names, and
    /// the permutation of `group` that maps `state` to it (FR-174
    /// "Canonicalisation"): the member of the orbit with the least key.
    ///
    /// # Errors
    ///
    /// The stop the exploration engine takes; [`Self::take_halt`] says why.
    pub(super) fn canonicalise(
        &self,
        swap: &CopySwap,
        state: &HyperState,
    ) -> Result<(HyperState, Permutation), ExpansionStop> {
        let mut orbit = vec![Member {
            key: self.key_bytes(state)?,
            state: state.clone(),
            positions: (0..state.components.len()).collect(),
        }];
        let mut seen: BTreeSet<Vec<u8>> = BTreeSet::from([orbit[0].key.clone()]);
        let mut next = 0;
        while let Some(member) = orbit.get(next) {
            let mut found = Vec::new();
            for (index, generator) in swap.generators.iter().enumerate() {
                let exchanged = self.exchange(member, index, generator)?;
                if seen.insert(exchanged.key.clone()) {
                    found.push(exchanged);
                }
            }
            orbit.extend(found);
            next += 1;
        }
        let least = orbit
            .iter()
            .min_by(|left, right| left.key.cmp(&right.key))
            .ok_or_else(|| self.fault("an-orbit-has-a-member"))?;
        let moved = least
            .positions
            .iter()
            .enumerate()
            .map(|(slot, from)| (swap.variables[*from].clone(), swap.variables[slot].clone()));
        let permutation = Permutation::of(&copy_population(), moved);
        Ok((least.state.clone(), permutation))
    }

    /// `member` with generator `index` applied: the two variables'
    /// components exchanged and the automaton state renamed to match.
    fn exchange(
        &self,
        member: &Member,
        index: usize,
        generator: &CopyGenerator,
    ) -> Result<Member, ExpansionStop> {
        let mut components = member.state.components.clone();
        components.swap(generator.first, generator.second);
        let property = match &member.state.property {
            Property::After(at) => {
                let exchanged = self.automaton.borrow_mut().exchange(*at, index);
                Property::After(exchanged.map_err(|error| self.automaton_stop(&error))?)
            }
            Property::Dead => Property::Dead,
            Property::Undefined {
                atom,
                cause,
                partner,
            } => Property::Undefined {
                atom: self.exchanged_atom(*atom, generator)?,
                cause: cause.clone(),
                partner: partner.clone(),
            },
            Property::Witnesses(_) => {
                return Err(self.fault("a-self-composition-state-has-no-witness-set"))
            }
        };
        let state = HyperState {
            components,
            property,
        };
        let key = self.key_bytes(&state)?;
        Ok(member.after(generator, state, key))
    }

    /// The atom `generator` takes `atom` to.
    fn exchanged_atom(
        &self,
        atom: FormulaNodeId,
        generator: &CopyGenerator,
    ) -> Result<FormulaNodeId, ExpansionStop> {
        let automaton = self.automaton.borrow();
        let atoms = automaton.atoms();
        atoms
            .iter()
            .position(|known| *known == atom)
            .and_then(|index| generator.atoms.get(index))
            .and_then(|image| atoms.get(*image))
            .copied()
            .ok_or_else(|| self.fault("an-undefined-atom-is-an-atom-of-the-body"))
    }

    /// The atom of the body that reads, in the state whose component `k` is
    /// the stored state's component `positions[k]`, what `atom` reads in the
    /// stored state: the atom to name in a counterexample concretised by
    /// `positions` (FR-174 "Counterexamples and verdicts"). `None` when
    /// `positions` is no element of the copy group.
    pub fn concrete_atom(&self, atom: FormulaNodeId, positions: &[usize]) -> Option<FormulaNodeId> {
        let Some(swap) = &self.copy_swap else {
            return Some(atom);
        };
        let automaton = self.automaton.borrow();
        let atoms = automaton.atoms();
        let stored = atoms.iter().position(|known| *known == atom)?;
        let mut elements = vec![(
            (0..positions.len()).collect::<Vec<_>>(),
            (0..atoms.len()).collect::<Vec<_>>(),
        )];
        let mut seen: BTreeSet<Vec<usize>> = BTreeSet::from([elements[0].0.clone()]);
        let mut next = 0;
        while let Some((held, letters)) = elements.get(next).cloned() {
            if held == positions {
                // Atom `x` of the concrete letters reads atom `letters[x]` of
                // the stored ones.
                let concrete = letters.iter().position(|read| *read == stored)?;
                return atoms.get(concrete).copied();
            }
            for generator in &swap.generators {
                let mut moved = held.clone();
                moved.swap(generator.first, generator.second);
                if seen.insert(moved.clone()) {
                    let read = generator.atoms.iter().map(|a| letters[*a]).collect();
                    elements.push((moved, read));
                }
            }
            next += 1;
        }
        None
    }

    /// The canonical encoding of `state`'s key: what orders an orbit.
    fn key_bytes(&self, state: &HyperState) -> Result<Vec<u8>, ExpansionStop> {
        let identity = IdentityLimits::default();
        quire_canonical::to_vec(
            &HyperKey::of(&self.joint.systems, state),
            quire_canonical::Limits::new(identity.input_bytes),
        )
        .map_err(|error| {
            self.joint
                .record(Halt::Encoding(EncodingRefusal::of(&error, identity)))
        })
    }

    fn automaton_stop(&self, error: &AutomatonError) -> ExpansionStop {
        match error {
            AutomatonError::StateLimit { limit } => self.joint.record(Halt::Automaton(*limit)),
            _ => self.fault("a-reached-automaton-state-exchanges"),
        }
    }

    fn fault(&self, invariant: &'static str) -> ExpansionStop {
        self.joint.record(Halt::Fault(InternalFault::new(
            "hyper-copy-swap",
            invariant,
        )))
    }
}
