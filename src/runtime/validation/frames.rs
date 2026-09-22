// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-007: recorded population differences and immutable model effect frames.

use super::super::{FieldBinding, ObjectIdentity, QualifiedName, ValueId, ValueNode};
use super::budget::{Result, Stage};
use super::values::{locus, require_population};
use super::{PopulationIndexes, PopulationKey, RuntimePathSegment, Validator};
use crate::checking::FrameIndex;
use crate::linking::{DeclarationKey, DeclarationLocation};
use qsl_foundation::Code;
use quire_contract_ir as ir;
use std::collections::{BTreeMap, BTreeSet};

type FieldValues<'a> = BTreeMap<&'a ir::SymbolName, Option<ValueId>>;

impl<F: FnMut() -> bool> Validator<'_, '_, F> {
    fn field_map<'a>(&mut self, fields: &'a [FieldBinding]) -> Result<FieldValues<'a>> {
        let mut values = BTreeMap::new();
        for field in fields {
            self.budget.visit()?;
            match values.entry(&field.name) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(Some(field.value));
                }
                std::collections::btree_map::Entry::Occupied(mut entry) => {
                    entry.insert(None);
                }
            }
        }
        Ok(values)
    }

    fn storage_equal(
        &mut self,
        left: (&[ValueNode], ValueId),
        right: (&[ValueNode], ValueId),
    ) -> Result<Option<bool>> {
        self.budget.visit()?;
        let a = usize::try_from(left.1.index())
            .ok()
            .and_then(|index| left.0.get(index));
        let b = usize::try_from(right.1.index())
            .ok()
            .and_then(|index| right.0.get(index));
        let (Some(a), Some(b)) = (a, b) else {
            return Ok(None);
        };
        let equal = match (a, b) {
            (ValueNode::Boolean { value: a }, ValueNode::Boolean { value: b }) => a == b,
            (ValueNode::Integer { value: a }, ValueNode::Integer { value: b }) => a == b,
            (ValueNode::Text { value: a }, ValueNode::Text { value: b }) => {
                self.budget.text_equal(a, b)?
            }
            (
                ValueNode::Enum {
                    declaration: a,
                    variant: av,
                },
                ValueNode::Enum {
                    declaration: b,
                    variant: bv,
                },
            ) => a == b && av == bv,
            (ValueNode::Absent, ValueNode::Absent) => true,
            (ValueNode::Present { value: a }, ValueNode::Present { value: b }) => {
                self.budget.visit()?;
                return self.storage_equal((left.0, *a), (right.0, *b));
            }
            (ValueNode::Sequence { values: a }, ValueNode::Sequence { values: b }) => {
                if a.len() != b.len() {
                    return Ok(Some(false));
                }
                let mut ambiguous = false;
                for (a, b) in a.iter().zip(b) {
                    self.budget.visit()?;
                    match self.storage_equal((left.0, *a), (right.0, *b))? {
                        Some(true) => {}
                        Some(false) => return Ok(Some(false)),
                        None => ambiguous = true,
                    }
                }
                if ambiguous {
                    return Ok(None);
                }
                true
            }
            (
                ValueNode::Record {
                    declaration: a,
                    fields: af,
                },
                ValueNode::Record {
                    declaration: b,
                    fields: bf,
                },
            ) => {
                if a != b {
                    return Ok(Some(false));
                }
                let (a, b) = (self.field_map(af)?, self.field_map(bf)?);
                if a.len() != b.len() {
                    return Ok(Some(false));
                }
                let mut ambiguous = false;
                for (name, a) in a {
                    self.budget.visit()?;
                    let Some(b) = b.get(name) else {
                        return Ok(Some(false));
                    };
                    let (Some(a), Some(b)) = (a, b) else {
                        ambiguous = true;
                        continue;
                    };
                    match self.storage_equal((left.0, a), (right.0, *b))? {
                        Some(true) => {}
                        Some(false) => return Ok(Some(false)),
                        None => ambiguous = true,
                    }
                }
                if ambiguous {
                    return Ok(None);
                }
                true
            }
            (ValueNode::Object { identity: a }, ValueNode::Object { identity: b })
            | (ValueNode::Reference { identity: a }, ValueNode::Reference { identity: b }) => {
                a.model == b.model
                    && a.record == b.record
                    && a.universe == b.universe
                    && self.budget.text_equal(&a.key, &b.key)?
            }
            _ => false,
        };
        Ok(Some(equal))
    }

    fn declared_deltas(
        &mut self,
        created: &[ObjectIdentity],
        deleted: &[ObjectIdentity],
        location: &DeclarationLocation,
    ) -> Result<(BTreeSet<ObjectIdentity>, BTreeSet<ObjectIdentity>)> {
        let mut creates = BTreeSet::new();
        let mut deletes = BTreeSet::new();
        for (declared, identities) in [(created, &mut creates), (deleted, &mut deletes)] {
            for identity in declared {
                self.budget.visit()?;
                self.budget.location.path = PopulationKey::of(identity).path();
                self.budget
                    .location
                    .path
                    .push(RuntimePathSegment::Object(identity.key.clone()));
                if !identities.insert(identity.clone()) {
                    self.related(
                        Stage::Delta,
                        Code::PopulationDeltaMismatch,
                        "duplicate declared population delta identity",
                        location,
                    )?;
                }
            }
        }
        for identity in creates.intersection(&deletes) {
            self.budget.visit()?;
            self.budget.location.path = PopulationKey::of(identity).path();
            self.budget
                .location
                .path
                .push(RuntimePathSegment::Object(identity.key.clone()));
            self.related(
                Stage::Delta,
                Code::PopulationDeltaMismatch,
                "created and deleted identity sets intersect",
                location,
            )?;
        }
        Ok((creates, deletes))
    }

    pub(super) fn inspect_frames(
        &mut self,
        clause_index: usize,
        indexes: &PopulationIndexes,
    ) -> Result<()> {
        let Some(invocation_index) = self.selected.invocation else {
            return Ok(());
        };
        let input = self.input;
        let invocation = input.invocations[invocation_index].draft();
        let requirements = self.checked.clauses()[clause_index].runtime_requirements();
        let Some(operation) = requirements.operation else {
            return Ok(());
        }; // Binding stage records the defect.
        let model = requirements.model;
        let Some(frame) = self
            .catalog(model.environment().owner())
            .and_then(|catalog| catalog.frames.get(&(&operation.context, &operation.name)))
        else {
            return self.budget.issue(
                Stage::Binding,
                Code::InvalidModelBinding,
                "checked operation lacks its admitted frame index",
            );
        };
        let location = locus(
            model,
            DeclarationKey::Operation {
                context: operation.context.clone(),
                name: operation.name.clone(),
            },
            &operation.source,
        );
        self.at_invocation(invocation_index);
        let (declared_created, declared_deleted) =
            self.declared_deltas(&invocation.created, &invocation.deleted, &location)?;
        let (Some(pre), Some(post)) = (self.selected.pre, self.selected.post) else {
            return Ok(());
        };
        let mut populations = BTreeSet::new();
        for index in [pre, post] {
            for key in indexes
                .get(&index)
                .into_iter()
                .flat_map(|populations| populations.keys())
            {
                self.budget.visit()?;
                populations.insert(key.clone());
            }
        }
        let mut created = BTreeSet::new();
        let mut deleted = BTreeSet::new();
        let mut complete = true;
        for key in populations {
            self.at_snapshot(pre, ir::StateObservation::Pre);
            self.budget.location.path = key.path();
            let pre_complete =
                require_population(&mut self.budget, &self.bound_models, indexes, pre, &key)?;
            self.at_snapshot(post, ir::StateObservation::Post);
            self.budget.location.path = key.path();
            let post_complete =
                require_population(&mut self.budget, &self.bound_models, indexes, post, &key)?;
            let population_complete = pre_complete && post_complete;
            complete &= population_complete;
            // Available unique objects can establish field changes even when a
            // population is incomplete. Missing objects establish deltas only
            // when both observations are complete.
            let before = indexes
                .get(&pre)
                .and_then(|populations| populations.get(&key))
                .filter(|index| index.unambiguous)
                .map(|index| (index.position, &index.objects));
            let after = indexes
                .get(&post)
                .and_then(|populations| populations.get(&key))
                .filter(|index| index.unambiguous)
                .map(|index| (index.position, &index.objects));
            let (Some((before_position, before)), Some((after_position, after))) = (before, after)
            else {
                complete = false;
                continue;
            };
            for name in before.keys() {
                self.budget.visit()?;
                if population_complete && !after.contains_key(name) {
                    let identity = ObjectIdentity {
                        model: key.model.clone(),
                        record: key.record.clone(),
                        universe: key.universe.clone(),
                        key: name.clone(),
                    };
                    self.at_snapshot(pre, ir::StateObservation::Pre);
                    self.budget.location.path = key.path();
                    self.budget
                        .location
                        .path
                        .push(RuntimePathSegment::Object(name.clone()));
                    if identity.model != *frame.owner || !frame.deleted.contains(&identity.record) {
                        self.related(
                            Stage::Frame,
                            Code::FrameViolation,
                            "object deletion is outside the selected model frame",
                            &location,
                        )?;
                    }
                    deleted.insert(identity);
                }
            }
            for (name, after_index) in after {
                self.budget.visit()?;
                self.at_snapshot(post, ir::StateObservation::Post);
                self.budget.location.path = key.path();
                self.budget
                    .location
                    .path
                    .push(RuntimePathSegment::Object(name.clone()));
                if let Some(before_index) = before.get(name) {
                    let (Some(before_index), Some(after_index)) = (before_index, after_index)
                    else {
                        continue;
                    };
                    let before_object = &input.snapshots[pre].draft().populations[before_position]
                        .objects[*before_index];
                    let after_object = &input.snapshots[post].draft().populations[after_position]
                        .objects[*after_index];
                    self.object_frame(
                        (pre, &before_object.fields),
                        (post, &after_object.fields),
                        &key,
                        frame,
                        &location,
                    )?;
                } else if population_complete {
                    let identity = ObjectIdentity {
                        model: key.model.clone(),
                        record: key.record.clone(),
                        universe: key.universe.clone(),
                        key: name.clone(),
                    };
                    if identity.model != *frame.owner || !frame.created.contains(&identity.record) {
                        self.related(
                            Stage::Frame,
                            Code::FrameViolation,
                            "object creation is outside the selected model frame",
                            &location,
                        )?;
                    }
                    created.insert(identity);
                }
            }
        }
        self.at_invocation(invocation_index);
        if complete && (created != declared_created || deleted != declared_deleted) {
            self.related(
                Stage::Delta,
                Code::PopulationDeltaMismatch,
                "declared population deltas differ from complete pre/post populations",
                &location,
            )?;
        }
        self.state_frame(pre, post, &location)
    }

    fn object_frame(
        &mut self,
        pre: (usize, &[FieldBinding]),
        post: (usize, &[FieldBinding]),
        key: &PopulationKey,
        frame: &FrameIndex<'_>,
        location: &DeclarationLocation,
    ) -> Result<()> {
        let (before, after) = (self.field_map(pre.1)?, self.field_map(post.1)?);
        for (name, before_value) in before {
            self.budget.visit()?;
            let (Some(before_value), Some(Some(after_value))) = (before_value, after.get(name))
            else {
                continue;
            }; // Exact-field validation already diagnoses missing data as invalid.
            self.budget
                .location
                .path
                .push(RuntimePathSegment::Field(name.clone()));
            let input = self.input;
            let equal = self.storage_equal(
                (&input.snapshots[pre.0].draft().arena, before_value),
                (&input.snapshots[post.0].draft().arena, *after_value),
            )?;
            let allowed = key.model == *frame.owner && frame.fields.contains(&(&key.record, name));
            if equal == Some(false) && !allowed {
                self.related(
                    Stage::Frame,
                    Code::FrameViolation,
                    "object field changed outside the selected model frame",
                    location,
                )?;
            }
            self.budget.location.path.pop();
        }
        Ok(())
    }

    fn state_map(&mut self, snapshot: usize) -> Result<BTreeMap<QualifiedName, Option<ValueId>>> {
        let mut values = BTreeMap::new();
        for root in &self.input.snapshots[snapshot].draft().values {
            self.budget.visit()?;
            match values.entry(root.declaration.clone()) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(Some(root.value));
                }
                std::collections::btree_map::Entry::Occupied(mut entry) => {
                    entry.insert(None);
                }
            }
        }
        Ok(values)
    }

    fn state_frame(
        &mut self,
        pre: usize,
        post: usize,
        location: &DeclarationLocation,
    ) -> Result<()> {
        let before = self.state_map(pre)?;
        let after = self.state_map(post)?;
        let mut roots = BTreeSet::new();
        for name in before
            .keys()
            .chain(after.keys())
            .chain(self.required_states.iter().map(|(_, name)| name))
        {
            self.budget.visit()?;
            roots.insert(name.clone());
        }
        for name in roots {
            self.budget.visit()?;
            for (snapshot, observation, values) in [
                (pre, ir::StateObservation::Pre, &before),
                (post, ir::StateObservation::Post, &after),
            ] {
                self.budget.poll()?;
                if !values.contains_key(&name) {
                    self.at_snapshot(snapshot, observation);
                    self.budget.location.path = vec![RuntimePathSegment::State(name.clone())];
                    self.related(
                        Stage::Observation,
                        Code::UnavailableObservation,
                        "State root counterpart is unavailable for frame comparison",
                        location,
                    )?;
                }
            }
            let (Some(Some(a)), Some(Some(b))) = (before.get(&name), after.get(&name)) else {
                continue;
            };
            self.at_snapshot(post, ir::StateObservation::Post);
            self.budget.location.path = vec![RuntimePathSegment::State(name)];
            let input = self.input;
            if self.storage_equal(
                (&input.snapshots[pre].draft().arena, *a),
                (&input.snapshots[post].draft().arena, *b),
            )? == Some(false)
            {
                self.related(
                    Stage::Frame,
                    Code::FrameViolation,
                    "State root changed without a model root-write permission",
                    location,
                )?;
            }
        }
        Ok(())
    }
}
