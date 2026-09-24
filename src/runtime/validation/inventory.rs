// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-007: bounded exact selection and complete population-index construction.

use super::super::ModelBinding;
use super::budget::{Result, Stage};
use super::{
    snapshot_key, Address, ObservationSelection, PopulationIndex, RuntimePathSegment,
    RuntimeReference, Validator,
};
use crate::syntax::ClauseKind;
use qsl_foundation::{Code, SourceIdentity};
use quire_contract_ir as ir;
use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};

impl<F: FnMut() -> bool> Validator<'_, '_, F> {
    fn reference(&self, address: Address) -> RuntimeReference {
        match address {
            Address::Snapshot(index) => {
                RuntimeReference::Snapshot(self.input.snapshots[index].reference())
            }
            Address::Invocation(index) => {
                RuntimeReference::Invocation(self.input.invocations[index].reference())
            }
        }
    }

    pub(super) fn inspect_inventory(&mut self) -> Result<()> {
        self.budget.ceiling(
            self.input
                .snapshots
                .len()
                .checked_add(self.input.invocations.len()),
            self.budget.limits.artifacts,
            "validation inventory count limit",
        )?;
        // Complete aggregate byte admission precedes every derived input index.
        for snapshot in &self.input.snapshots {
            self.budget.artifact(snapshot.bytes().len())?;
        }
        for invocation in &self.input.invocations {
            self.budget.artifact(invocation.bytes().len())?;
        }
        for address in (0..self.input.snapshots.len())
            .map(Address::Snapshot)
            .chain((0..self.input.invocations.len()).map(Address::Invocation))
        {
            self.budget.visit()?;
            let reference = self.reference(address);
            let identity = reference.identity();
            self.inventory
                .entry(labels(identity))
                .or_default()
                .push(address);
        }
        for entries in self.inventory.values() {
            self.budget.poll()?;
            if entries.len() > 1 {
                // Each offered conflicting artifact has its own exact provenance.
                // This yields the same multiset regardless of inventory order.
                for address in entries {
                    self.budget.visit()?;
                    self.budget.location.artifact = self.reference(*address);
                    self.budget.issue(
                        Stage::Identity,
                        Code::InvalidRuntimeInput,
                        "duplicate native input source labels in inventory",
                    )?;
                }
            }
        }
        Ok(())
    }

    fn resolve(&mut self, expected: RuntimeReference) -> Result<Option<Address>> {
        self.budget.location.artifact = expected.clone();
        self.budget.location.path.clear();
        self.budget.visit()?;
        let key = labels(expected.identity());
        let Some(entries) = self.inventory.get(&key) else {
            self.budget.issue(
                Stage::Observation,
                Code::UnavailableObservation,
                "selected native input is unavailable",
            )?;
            return Ok(None);
        };
        if entries.len() != 1 {
            return Ok(None);
        }
        let address = entries[0];
        let actual = self.reference(address);
        if !matches!(
            (&expected, &actual),
            (RuntimeReference::Snapshot(_), RuntimeReference::Snapshot(_))
                | (
                    RuntimeReference::Invocation(_),
                    RuntimeReference::Invocation(_)
                )
        ) {
            self.budget.issue(
                Stage::Observation,
                Code::WrongSnapshot,
                "selected identity has the wrong artifact role",
            )?;
            return Ok(None);
        }
        if actual.digest() != expected.digest() {
            self.budget.issue(
                Stage::Binding,
                Code::StaleDependency,
                "selected input digest differs from supplied complete bytes",
            )?;
            return Ok(None);
        }
        Ok(Some(address))
    }

    fn resolve_snapshot(
        &mut self,
        expected: &super::super::SnapshotRef,
        observation: ir::StateObservation,
    ) -> Result<Option<usize>> {
        self.budget.location.observation = Some(observation);
        let Some(Address::Snapshot(index)) =
            self.resolve(RuntimeReference::Snapshot(expected.clone()))?
        else {
            return Ok(None);
        };
        if self.input.snapshots[index].draft().observation != observation {
            self.budget.issue(
                Stage::Observation,
                Code::WrongSnapshot,
                "snapshot observation does not match its selected role",
            )?;
        }
        Ok(Some(index))
    }

    pub(super) fn select(&mut self, clause: usize) -> Result<()> {
        match (
            &self.selection.observation,
            self.checked.linked().unit().clauses()[clause].kind,
        ) {
            (ObservationSelection::Current { snapshot, .. }, ClauseKind::Invariant) => {
                self.selected.current =
                    self.resolve_snapshot(snapshot, ir::StateObservation::Current)?;
            }
            (
                ObservationSelection::Invocation { invocation },
                ClauseKind::Precondition | ClauseKind::Postcondition,
            ) => {
                self.budget.location.observation = None;
                if let Some(Address::Invocation(index)) =
                    self.resolve(RuntimeReference::Invocation(invocation.clone()))?
                {
                    self.selected.invocation = Some(index);
                    let input = self.input;
                    let invocation = input.invocations[index].draft();
                    self.selected.pre =
                        self.resolve_snapshot(&invocation.pre, ir::StateObservation::Pre)?;
                    self.selected.post =
                        self.resolve_snapshot(&invocation.post, ir::StateObservation::Post)?;
                }
            }
            (
                ObservationSelection::Current { .. },
                ClauseKind::Precondition | ClauseKind::Postcondition,
            )
            | (ObservationSelection::Invocation { .. }, ClauseKind::Invariant) => {
                self.budget.issue(
                    Stage::Observation,
                    Code::WrongSnapshot,
                    "runtime selection form does not match the authored clause role",
                )?;
            }
        }
        Ok(())
    }

    pub(super) fn models(
        &mut self,
        bindings: &[ModelBinding],
    ) -> Result<BTreeSet<ir::RequirementRef>> {
        let mut counts = BTreeMap::new();
        let mut admitted = BTreeSet::new();
        for binding in bindings {
            self.budget.visit()?;
            self.budget.location.path = vec![RuntimePathSegment::Model(binding.model.clone())];
            let count = counts.entry(&binding.model).or_insert(0_usize);
            *count += 1; // Constructor-bounded metadata occurrences.
            if *count > 1 {
                self.budget.issue(
                    Stage::Binding,
                    Code::InvalidRuntimeInput,
                    "duplicate input model binding",
                )?;
            }
            let digest = self
                .catalog(&binding.model)
                .map(|catalog| catalog.model.digest());
            match digest {
                None => self.budget.issue(
                    Stage::Binding,
                    Code::InvalidModelBinding,
                    "input names a model outside the checked package",
                )?,
                Some(digest) if digest != binding.digest => self.budget.issue(
                    Stage::Binding,
                    Code::StaleDependency,
                    "input model binding selects stale native model bytes",
                )?,
                Some(_) => {
                    admitted.insert(binding.model.clone());
                }
            }
        }
        admitted.retain(|owner| counts.get(owner) == Some(&1));
        self.budget.location.path.clear();
        Ok(admitted)
    }

    pub(super) fn index_populations(&mut self) -> Result<()> {
        // Count all selected occurrences, including duplicate populations and
        // the pre/post occurrences of a surviving object, before allocating maps.
        for (observation, snapshot) in self.selected.snapshots() {
            self.at_snapshot(snapshot, observation);
            for population in &self.input.snapshots[snapshot].draft().populations {
                self.budget.visit()?;
                self.budget.objects(population.objects.len())?;
            }
        }
        for (observation, snapshot) in self.selected.snapshots() {
            self.at_snapshot(snapshot, observation);
            let input = self.input;
            let offered = &input.snapshots[snapshot];
            let models = self.models(&offered.draft().models)?;
            let mut populations = BTreeMap::new();
            for (position, population) in offered.draft().populations.iter().enumerate() {
                self.budget.visit()?;
                let key = snapshot_key(offered, position);
                self.budget.location.path = key.path();
                let role = self
                    .catalog(&population.model)
                    .and_then(|catalog| catalog.objects.get(&population.record))
                    .copied();
                if !models.contains(&population.model) {
                    self.budget.issue(
                        Stage::Binding,
                        Code::InvalidModelBinding,
                        "population lacks an exact unambiguous input model binding",
                    )?;
                }
                let valid_role = role.is_some_and(|role| role.universe == population.universe);
                if !valid_role {
                    self.budget.issue(
                        Stage::Population,
                        Code::InvalidRuntimeInput,
                        "population type or universe is not an admitted object role",
                    )?;
                }
                let mut index = PopulationIndex {
                    position,
                    objects: BTreeMap::new(),
                    complete: population.complete,
                    unambiguous: valid_role && models.contains(&population.model),
                };
                for (object_index, object) in population.objects.iter().enumerate() {
                    self.budget.visit()?;
                    self.budget
                        .location
                        .path
                        .push(RuntimePathSegment::Object(object.key.clone()));
                    match index.objects.entry(object.key.clone()) {
                        Entry::Vacant(entry) => {
                            entry.insert(Some(object_index));
                        }
                        Entry::Occupied(mut entry) => {
                            entry.insert(None);
                            self.budget.issue(
                                Stage::Population,
                                Code::InvalidRuntimeInput,
                                "duplicate object key within population",
                            )?;
                        }
                    }
                    self.budget.location.path.pop();
                }
                match populations.entry(key) {
                    Entry::Vacant(entry) => {
                        entry.insert(index);
                    }
                    Entry::Occupied(mut entry) => {
                        // Conflicting entries cannot establish authority; retain
                        // every observed incompleteness flag regardless of order.
                        entry.get_mut().complete &= index.complete;
                        entry.get_mut().unambiguous = false;
                        self.budget.issue(
                            Stage::Population,
                            Code::InvalidRuntimeInput,
                            "duplicate population identity",
                        )?;
                    }
                }
            }
            self.bound_models.insert(snapshot, models);
            self.indexes.insert(snapshot, populations);
            let mut states = BTreeMap::new();
            for root in &offered.draft().values {
                self.budget.visit()?;
                match states.entry(root.declaration.clone()) {
                    Entry::Vacant(entry) => {
                        entry.insert(Some(root.value));
                    }
                    Entry::Occupied(mut entry) => {
                        entry.insert(None);
                    }
                }
            }
            self.states.insert(snapshot, states);
        }
        Ok(())
    }
}

/// A runtime artifact's four source labels (FR-018), in label order.
pub(super) type Labels = (String, String, String, String);

/// The inventory key of `identity`: all four labels, so artifacts that
/// differ in any label never share an entry.
fn labels(identity: &SourceIdentity) -> Labels {
    (
        identity.authority.clone(),
        identity.identity.clone(),
        identity.revision_namespace.clone(),
        identity.revision.clone(),
    )
}
