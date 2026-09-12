// SPDX-License-Identifier: AGPL-3.0-only
//! FR-046/047/049: exact evaluation over one admitted compiled value graph.
#![allow(
    clippy::result_large_err,
    reason = "closed typed failures retain complete identities without heap allocation"
)]
#![allow(
    clippy::unnecessary_lazy_evaluations,
    reason = "uniform closures keep typed refusal construction adjacent to failed lookups"
)]

use std::cmp::Ordering;

use quire_contract_ir as ir;

use super::input::*;
use super::work::{Dimension, Exhaustion, Work};
use super::{EvaluationOutcome, EvaluationReport, Limits};
use crate::checking::{Catalog, NativeType};
use crate::native_model::{ScalarKind, ScalarSite, Unit};
use crate::protocol_artifact::{
    wire as w, AdmittedPackage, ExactInteger, ExactRational, ProtocolNumber,
};

enum Stop {
    Incomplete(MissingInput),
    Refused(Refusal),
    Exhausted(Exhaustion),
}

type Result<T> = std::result::Result<T, Stop>;

/// Evaluate one exact declaration-local value from an admitted artifact.
pub fn evaluate(
    package: &AdmittedPackage,
    request: EvaluationRequest,
    view: &StateView,
    limits: Limits,
) -> EvaluationReport {
    let mut evaluator = Evaluator {
        package,
        request,
        view,
        work: Work::new(limits),
        binders: Vec::new(),
        populations: Vec::new(),
        locals: Vec::new(),
        call_depth: 0,
    };
    let outcome = match evaluator.run() {
        Ok(value) => EvaluationOutcome::Completed(value),
        Err(Stop::Incomplete(value)) => EvaluationOutcome::Incomplete(value),
        Err(Stop::Refused(value)) => EvaluationOutcome::Refused(value),
        Err(Stop::Exhausted(value)) => EvaluationOutcome::Exhausted(value),
    };
    EvaluationReport {
        limits: evaluator.work.limits,
        usage: evaluator.work.usage,
        outcome,
    }
}

struct Evaluator<'a> {
    package: &'a AdmittedPackage,
    request: EvaluationRequest,
    view: &'a StateView,
    work: Work,
    binders: Vec<((u32, u32), &'a InputSlot)>,
    populations: Vec<PopulationIndex<'a>>,
    locals: Vec<((u32, u32), Value)>,
    call_depth: usize,
}

struct PopulationIndex<'a> {
    requirement: w::Handle,
    population: &'a PopulationInput,
    anchor: w::Handle,
    object_type: w::ExportRef,
    universe: w::ExportRef,
    offset: usize,
    objects: Vec<&'a ObjectInput>,
}

impl<'a> PopulationIndex<'a> {
    fn nominal_domain_matches(&self, key: &ObjectKey) -> bool {
        key.model == self.object_type.model
            && key.object_type == self.object_type
            && key.universe == self.universe
    }

    fn domain_matches(&self, key: &ObjectKey) -> bool {
        self.nominal_domain_matches(key)
            && key.observation.anchor == self.anchor
            && key.observation.snapshot.0
                == self
                    .population
                    .authority
                    .assessment_selection
                    .snapshot_identity
            && key.observation.window.as_ref().map(|value| &value.0)
                == self
                    .population
                    .authority
                    .assessment_selection
                    .window_identity
                    .as_ref()
    }

    fn is_complete(&self) -> bool {
        self.population
            .authority
            .assessment_selection
            .membership_complete
            && self.population.membership.is_ok()
            && self.population.closure.is_ok()
    }

    fn object(&self, key: &ObjectKey) -> Option<&'a ObjectInput> {
        self.object_index(key)
            .ok()
            .and_then(|index| self.objects.get(index).copied())
    }

    fn object_index(&self, key: &ObjectKey) -> std::result::Result<usize, usize> {
        self.objects
            .binary_search_by(|candidate| object_key_cmp(&candidate.key, key))
    }
}

impl<'a> Evaluator<'a> {
    fn run(&mut self) -> Result<Value> {
        let declaration = self
            .package
            .package()
            .declarations
            .get(self.request.declaration as usize)
            .ok_or_else(|| Stop::Refused(Refusal::RequestDeclaration(self.request.declaration)))?;
        if self.request.value.declaration != self.request.declaration {
            return Err(Stop::Refused(Refusal::Owner(self.request.value.clone())));
        }
        if declaration
            .values
            .get(self.request.value.index as usize)
            .is_none()
        {
            return Err(Stop::Refused(Refusal::RequestValue(
                self.request.value.clone(),
            )));
        }
        self.validate_inputs()?;
        let value = self.expression(&self.request.value.clone(), 1)?;
        self.work
            .charge(Dimension::RetainedOutput, 1)
            .map_err(Stop::Exhausted)?;
        // Completed values contain no unavailable slot. This also prevents a
        // partially materialized query result from escaping through the report.
        ensure_available(&value)?;
        Ok(value)
    }

    fn validate_inputs(&mut self) -> Result<()> {
        let required =
            required_inputs(&mut self.work, self.package.package(), &self.request.value)?;
        let mut offered_binders = boolean_table(
            &self.work,
            self.package
                .package()
                .declarations
                .iter()
                .map(|declaration| declaration.binders.len()),
        )?;
        let mut offered_populations = boolean_table(
            &self.work,
            self.package
                .package()
                .declarations
                .iter()
                .map(|declaration| declaration.bindings.len()),
        )?;
        for offered in &self.view.binders {
            self.work
                .charge(Dimension::InputAggregateEntries, 1)
                .map_err(Stop::Exhausted)?;
            if offered.binder.declaration != self.request.declaration {
                return Err(Stop::Refused(Refusal::SurplusBinding(
                    offered.binder.clone(),
                )));
            }
            let declaration =
                &self.package.package().declarations[self.request.declaration as usize];
            let binder = declaration
                .binders
                .get(offered.binder.index as usize)
                .ok_or_else(|| Stop::Refused(Refusal::SurplusBinding(offered.binder.clone())))?;
            let key = (offered.binder.declaration, offered.binder.index);
            if !selected_handle(&required.binders, &offered.binder) {
                return Err(Stop::Refused(Refusal::SurplusBinding(
                    offered.binder.clone(),
                )));
            }
            let seen = offered_binders
                .get_mut(offered.binder.declaration as usize)
                .and_then(|declaration| declaration.get_mut(offered.binder.index as usize))
                .ok_or_else(|| Stop::Refused(Refusal::SurplusBinding(offered.binder.clone())))?;
            if std::mem::replace(seen, true) {
                return Err(Stop::Refused(Refusal::DuplicateBinding(
                    offered.binder.clone(),
                )));
            }
            self.entry()?;
            self.binders.try_reserve(1).map_err(|_| {
                Stop::Exhausted(self.work.allocation(Dimension::InputAggregateEntries, 1))
            })?;
            self.binders.push((key, &offered.value));
            match (offered.requirement, &offered.authority) {
                (Some(requirement), Some(authority)) => {
                    self.authority(self.request.declaration, requirement, authority)?;
                }
                (None, None) if binder.kind == w::BinderKind::Parameter => {}
                _ => return Err(Stop::Refused(Refusal::Authority(offered.binder.clone()))),
            }
            self.slot(&offered.value, binder.value_type, 1)?;
        }
        for binder in &required.binders {
            if !offered_binders
                .get(binder.declaration as usize)
                .and_then(|declaration| declaration.get(binder.index as usize))
                .copied()
                .unwrap_or(false)
            {
                return Err(Stop::Refused(Refusal::MissingBinding(binder.clone())));
            }
        }
        self.binders.sort_unstable_by_key(|(key, _)| *key);
        for population in &self.view.populations {
            self.work
                .charge(Dimension::InputAggregateEntries, 1)
                .map_err(Stop::Exhausted)?;
            if population.requirement.declaration != population.closure_requirement.declaration {
                return Err(Stop::Refused(Refusal::SurplusBinding(
                    population.requirement.clone(),
                )));
            }
            if !selected_handle(&required.populations, &population.requirement) {
                return Err(Stop::Refused(Refusal::SurplusBinding(
                    population.requirement.clone(),
                )));
            }
            let seen = offered_populations
                .get_mut(population.requirement.declaration as usize)
                .and_then(|declaration| declaration.get_mut(population.requirement.index as usize))
                .ok_or_else(|| {
                    Stop::Refused(Refusal::SurplusBinding(population.requirement.clone()))
                })?;
            if std::mem::replace(seen, true) {
                return Err(Stop::Refused(Refusal::DuplicateBinding(
                    population.requirement.clone(),
                )));
            }
            self.authority(
                population.requirement.declaration,
                population.requirement.index,
                &population.authority,
            )?;
            let declaration = self
                .package
                .package()
                .declarations
                .get(population.requirement.declaration as usize)
                .ok_or_else(|| {
                    Stop::Refused(Refusal::SurplusBinding(population.requirement.clone()))
                })?;
            let requirement = declaration
                .bindings
                .get(population.requirement.index as usize)
                .ok_or_else(|| {
                    Stop::Refused(Refusal::SurplusBinding(population.requirement.clone()))
                })?;
            let closure = declaration
                .bindings
                .get(population.closure_requirement.index as usize)
                .ok_or_else(|| {
                    Stop::Refused(Refusal::SurplusBinding(
                        population.closure_requirement.clone(),
                    ))
                })?;
            if requirement.kind != w::BindingKind::Population
                || closure.kind != w::BindingKind::Closure
                || closure.requires != [population.requirement.index]
                || closure.model != requirement.model
                || closure.anchor != requirement.anchor
                || closure.value_type != requirement.value_type
            {
                return Err(Stop::Refused(Refusal::Authority(
                    population.closure_requirement.clone(),
                )));
            }
            let expected = requirement.value_type.0.ok_or_else(|| {
                Stop::Refused(Refusal::AdmittedInvariant(population.requirement.clone()))
            })?;
            let w::Type::Object {
                export: object_type,
            } = self
                .package
                .package()
                .types
                .get(expected as usize)
                .ok_or_else(|| {
                    Stop::Refused(Refusal::AdmittedInvariant(population.requirement.clone()))
                })?
                .clone()
            else {
                return Err(Stop::Refused(Refusal::AdmittedInvariant(
                    population.requirement.clone(),
                )));
            };
            let universe = requirement.model.0.clone().ok_or_else(|| {
                Stop::Refused(Refusal::AdmittedInvariant(population.requirement.clone()))
            })?;
            let mut objects = Vec::new();
            objects.try_reserve(population.objects.len()).map_err(|_| {
                Stop::Exhausted(
                    self.work
                        .allocation(Dimension::InputAggregateEntries, population.objects.len()),
                )
            })?;
            for object in &population.objects {
                self.work
                    .charge(Dimension::InputAggregateEntries, 1)
                    .map_err(Stop::Exhausted)?;
                if object.key.observation.anchor != requirement.anchor
                    || object.key.observation.snapshot.0
                        != population.authority.assessment_selection.snapshot_identity
                    || object.key.observation.window.as_ref().map(|value| &value.0)
                        != population
                            .authority
                            .assessment_selection
                            .window_identity
                            .as_ref()
                    || object.key.observation.record.0.is_empty()
                {
                    return Err(Stop::Refused(Refusal::Authority(
                        population.requirement.clone(),
                    )));
                }
                self.object(object, expected)?;
                objects.push(object);
            }
            objects.sort_unstable_by(|left, right| object_key_cmp(&left.key, &right.key));
            if let Some(duplicate) = objects
                .windows(2)
                .find(|pair| pair[0].key == pair[1].key)
                .map(|pair| &pair[0].key)
            {
                return Err(Stop::Refused(Refusal::DuplicateObject(duplicate.clone())));
            }
            self.entry()?;
            self.populations.try_reserve(1).map_err(|_| {
                Stop::Exhausted(self.work.allocation(Dimension::InputAggregateEntries, 1))
            })?;
            let offset = self.populations.last().map_or(Ok(0), |prior| {
                prior
                    .offset
                    .checked_add(prior.objects.len())
                    .ok_or_else(|| {
                        Stop::Exhausted(counter_overflow(
                            &self.work,
                            Dimension::InputAggregateEntries,
                        ))
                    })
            })?;
            self.populations.push(PopulationIndex {
                requirement: population.requirement.clone(),
                population,
                anchor: requirement.anchor.clone(),
                object_type,
                universe,
                offset,
                objects,
            });
        }
        for required in &required.populations {
            if !offered_populations
                .get(required.declaration as usize)
                .and_then(|declaration| declaration.get(required.index as usize))
                .copied()
                .unwrap_or(false)
            {
                return Err(Stop::Refused(Refusal::MissingBinding(required.clone())));
            }
        }
        self.validate_population_references()?;
        for population in &self.populations {
            if !population
                .population
                .authority
                .assessment_selection
                .membership_complete
            {
                return Err(Stop::Incomplete(MissingInput::Membership(
                    population.requirement.clone(),
                )));
            }
            if let Err(missing) = &population.population.membership {
                return Err(Stop::Incomplete(missing.clone()));
            }
            if let Err(missing) = &population.population.closure {
                return Err(Stop::Incomplete(missing.clone()));
            }
        }
        Ok(())
    }

    fn validate_population_references(&self) -> Result<()> {
        for (_, slot) in &self.binders {
            self.validate_slot_references(slot)?;
        }
        for population in &self.populations {
            for object in &population.objects {
                for field in &object.fields {
                    self.validate_field_references(field)?;
                }
            }
        }
        Ok(())
    }

    fn validate_field_references(&self, field: &FieldInput) -> Result<()> {
        match &field.value {
            FieldValue::Compiled(slot) => self.validate_slot_references(slot),
            FieldValue::Contextual(slot) => self.validate_contextual_slot_references(slot),
        }
    }

    fn validate_slot_references(&self, slot: &InputSlot) -> Result<()> {
        let InputSlot::Available(value) = slot else {
            return Ok(());
        };
        match value.kind() {
            ValueKind::Record(fields) => {
                for field in fields {
                    self.validate_field_references(field)?;
                }
                Ok(())
            }
            ValueKind::Option(Some(child)) => self.validate_slot_references(child),
            ValueKind::Sequence(children) => {
                for child in children {
                    self.validate_slot_references(child)?;
                }
                Ok(())
            }
            ValueKind::Reference(key) | ValueKind::Object(key) => self.validate_graph_key(key),
            ValueKind::Boolean(_)
            | ValueKind::Number(_)
            | ValueKind::Text(_)
            | ValueKind::Enum(_)
            | ValueKind::Option(None) => Ok(()),
        }
    }

    fn validate_contextual_slot_references(&self, slot: &ContextualSlot) -> Result<()> {
        let ContextualSlot::Available(value) = slot else {
            return Ok(());
        };
        match value.kind() {
            ContextualValueKind::Record(fields) => {
                for field in fields {
                    self.validate_field_references(field)?;
                }
                Ok(())
            }
            ContextualValueKind::Option(Some(child)) => {
                self.validate_contextual_slot_references(child)
            }
            ContextualValueKind::Sequence(children) => {
                for child in children {
                    self.validate_contextual_slot_references(child)?;
                }
                Ok(())
            }
            ContextualValueKind::Reference(key) | ContextualValueKind::Object(key) => {
                self.validate_graph_key(key)
            }
            ContextualValueKind::Boolean(_)
            | ContextualValueKind::Number(_)
            | ContextualValueKind::Text(_)
            | ContextualValueKind::Enum(_)
            | ContextualValueKind::Option(None) => Ok(()),
        }
    }

    fn validate_graph_key(&self, key: &ObjectKey) -> Result<()> {
        let selected = self
            .populations
            .iter()
            .find(|population| population.domain_matches(key));
        let Some(population) = selected else {
            if self
                .populations
                .iter()
                .any(|population| population.nominal_domain_matches(key))
            {
                return Err(Stop::Refused(Refusal::PopulationDomain(key.clone())));
            }
            return Ok(());
        };
        if population.is_complete() && population.object(key).is_none() {
            return Err(Stop::Refused(Refusal::Dangling(key.clone())));
        }
        Ok(())
    }

    fn authority(&mut self, owner: u32, index: u32, offered: &AuthorityEvidence) -> Result<()> {
        let handle = w::Handle {
            declaration: owner,
            index,
        };
        let requirement = self
            .package
            .package()
            .declarations
            .get(owner as usize)
            .and_then(|declaration| declaration.bindings.get(index as usize))
            .ok_or_else(|| Stop::Refused(Refusal::SurplusBinding(handle.clone())))?;
        if offered.producer_contract_revision != PRODUCER_CONTRACT_REVISION
            || offered.observation_contract_revision != OBSERVATION_CONTRACT_REVISION
            || offered.static_selection.interface_version != "1.2.0"
            || !valid_identity(&offered.static_selection.document_identity)
            || !valid_identity(&offered.static_selection.model_identity)
            || !valid_identity(&offered.static_selection.profile_identity)
            || !valid_identity(&offered.static_selection.configuration_identity)
            || !valid_identity(&offered.assessment_selection.population_identity)
            || !valid_identity(&offered.assessment_selection.snapshot_identity)
            || !valid_identity(&offered.assessment_selection.closure_identity)
            || offered
                .assessment_selection
                .window_identity
                .as_ref()
                .is_some_and(|identity| !valid_identity(identity))
            || !valid_digest(&offered.static_selection.document_digest)
            || !valid_digest(&offered.static_selection.model_digest)
            || !valid_digest(&offered.static_selection.profile_digest)
            || !valid_digest(&offered.static_selection.configuration_digest)
            || !valid_observation_digest(&offered.assessment_selection.membership_digest)
            || !valid_observation_digest(&offered.assessment_selection.snapshot_digest)
            || !valid_observation_digest(&offered.assessment_selection.closure_digest)
            || offered
                .assessment_selection
                .window_digest
                .as_ref()
                .is_some_and(|digest| !valid_observation_digest(digest))
            || offered.assessment_selection.window_identity.is_some()
                != offered.assessment_selection.window_digest.is_some()
        {
            return Err(Stop::Refused(Refusal::AuthorityRevision));
        }
        if offered.compiled != *self.package.artifact()
            || offered.observation != requirement.authority
        {
            return Err(Stop::Refused(Refusal::Authority(handle)));
        }
        let Some(adapter) = &offered.adapter else {
            return Err(Stop::Refused(Refusal::UngroundedAuthorityMapping(handle)));
        };
        if !valid_adapter(&adapter.artifact)
            || adapter.compiled != offered.compiled
            || adapter.requirement != requirement.authority
            || adapter.producer != offered.producer
            || adapter.observation != offered.observation
            || adapter.producer_contract_revision != offered.producer_contract_revision
            || adapter.observation_contract_revision != offered.observation_contract_revision
            || adapter.static_selection != offered.static_selection
            || adapter.assessment_selection != offered.assessment_selection
        {
            return Err(Stop::Refused(Refusal::UngroundedAuthorityMapping(handle)));
        }
        if let Some(model) = &requirement.model.0 {
            let expected = self
                .package
                .package()
                .models
                .get(model.model as usize)
                .and_then(|model| {
                    self.package
                        .package()
                        .dependencies
                        .get(model.artifact as usize)
                })
                .map(|dependency| &dependency.artifact)
                .ok_or_else(|| Stop::Refused(Refusal::AdmittedInvariant(handle.clone())))?;
            if expected != &offered.producer {
                return Err(Stop::Refused(Refusal::Authority(handle)));
            }
        }
        Ok(())
    }

    fn slot(&mut self, slot: &InputSlot, expected: u32, depth: usize) -> Result<()> {
        self.work
            .charge(Dimension::InputValueNodes, 1)
            .map_err(Stop::Exhausted)?;
        self.work
            .charge(Dimension::InputStructuralDepth, depth)
            .map_err(Stop::Exhausted)?;
        match slot {
            InputSlot::Unavailable(_) => Ok(()),
            InputSlot::Available(value) => self.input_value(value, expected, depth),
        }
    }

    fn input_value(&mut self, value: &Value, expected: u32, depth: usize) -> Result<()> {
        if value.value_type != expected {
            return Err(Stop::Refused(Refusal::Type {
                expected,
                actual: value.value_type,
            }));
        }
        let ty = self
            .package
            .package()
            .types
            .get(expected as usize)
            .ok_or_else(|| Stop::Refused(Refusal::ValueShape(expected)))?
            .clone();
        match (&ty, value.kind()) {
            (w::Type::Boolean {}, ValueKind::Boolean(_)) => Ok(()),
            (w::Type::Scalar { representation, .. }, ValueKind::Number(number)) => {
                self.number(expected, representation, number)
            }
            (
                w::Type::Scalar {
                    representation: w::Representation::Text { maximum_scalars },
                    ..
                },
                ValueKind::Text(text),
            ) => {
                self.work
                    .charge(Dimension::InputTextBytes, text.len())
                    .map_err(Stop::Exhausted)?;
                let maximum = wire_integer(maximum_scalars).ok_or_else(|| {
                    Stop::Refused(Refusal::AdmittedInvariant(w::Handle {
                        declaration: self.request.declaration,
                        index: self.request.value.index,
                    }))
                })?;
                if i64::try_from(text.chars().count())
                    .ok()
                    .is_some_and(|count| count <= maximum)
                {
                    Ok(())
                } else {
                    Err(Stop::Refused(Refusal::Bounds(expected)))
                }
            }
            (w::Type::Enum { export }, ValueKind::Enum(variant)) => {
                let expected_path = self.export_path(export, w::ExportKind::Enum)?;
                let actual = self.export_path(variant, w::ExportKind::Variant)?;
                if variant.model == export.model
                    && actual.len() == 2
                    && actual.first() == expected_path.first()
                {
                    Ok(())
                } else {
                    Err(Stop::Refused(Refusal::ValueShape(expected)))
                }
            }
            (w::Type::Record { export }, ValueKind::Record(fields)) => {
                self.record(export, fields, depth)
            }
            (w::Type::Option { value: child }, ValueKind::Option(option)) => {
                if let Some(child_slot) = option {
                    self.entry()?;
                    self.slot(child_slot, *child, depth + 1)?;
                }
                Ok(())
            }
            (w::Type::Sequence { element, maximum }, ValueKind::Sequence(values)) => {
                let maximum = wire_integer(maximum).unwrap_or(-1);
                if i64::try_from(values.len())
                    .ok()
                    .is_none_or(|len| len > maximum)
                {
                    return Err(Stop::Refused(Refusal::Bounds(expected)));
                }
                for child in values {
                    self.entry()?;
                    self.slot(child, *element, depth + 1)?;
                }
                Ok(())
            }
            (
                w::Type::Reference {
                    object, universe, ..
                },
                ValueKind::Reference(key),
            ) => self.key(key, export_model(object), object, universe),
            (w::Type::Object { export }, ValueKind::Object(key)) => {
                let universe = self.population_export(export)?;
                self.key(key, export.model, export, &universe)
            }
            _ => Err(Stop::Refused(Refusal::ValueShape(expected))),
        }
    }

    fn number(
        &self,
        expected: u32,
        representation: &w::Representation,
        number: &ProtocolNumber,
    ) -> Result<()> {
        let valid = match (representation, number) {
            (w::Representation::Integer { minimum, maximum }, ProtocolNumber::Integer(value)) => {
                let value = value.value();
                wire_integer(minimum).is_some_and(|min| value >= min)
                    && wire_integer(maximum).is_some_and(|max| value <= max)
            }
            (
                w::Representation::Rational {
                    numerator_minimum,
                    numerator_maximum,
                    maximum_denominator,
                },
                ProtocolNumber::Rational(value),
            ) => {
                wire_integer(numerator_minimum).is_some_and(|min| value.numerator() >= min)
                    && wire_integer(numerator_maximum).is_some_and(|max| value.numerator() <= max)
                    && wire_integer(maximum_denominator)
                        .is_some_and(|max| value.denominator() <= max)
            }
            _ => false,
        };
        if valid {
            Ok(())
        } else {
            Err(Stop::Refused(Refusal::Bounds(expected)))
        }
    }

    fn record(&mut self, export: &w::ExportRef, fields: &[FieldInput], depth: usize) -> Result<()> {
        let path = self.export_path(export, w::ExportKind::Record)?.to_vec();
        let name = path
            .first()
            .ok_or_else(|| Stop::Refused(Refusal::Field(export.clone())))?;
        self.record_fields_by_name(export.model, name, fields, depth)
    }

    fn object(&mut self, object: &ObjectInput, expected: u32) -> Result<()> {
        let w::Type::Object { export } = self
            .package
            .package()
            .types
            .get(expected as usize)
            .ok_or_else(|| Stop::Refused(Refusal::ValueShape(expected)))?
            .clone()
        else {
            return Err(Stop::Refused(Refusal::ValueShape(expected)));
        };
        let universe = self.population_export(&export)?;
        self.key(&object.key, export.model, &export, &universe)?;
        self.record_object(&export, &object.fields, 1)
    }

    fn record_object(
        &mut self,
        export: &w::ExportRef,
        fields: &[FieldInput],
        depth: usize,
    ) -> Result<()> {
        // Object and record share the original record name but are distinct export kinds.
        let path = self.export_path(export, w::ExportKind::Object)?.to_vec();
        // Resolve directly through the object record because there is no Record export for object payloads.
        self.record_fields_by_name(export.model, &path[0], fields, depth)
    }

    fn record_fields_by_name(
        &mut self,
        model_index: u32,
        name: &str,
        fields: &[FieldInput],
        depth: usize,
    ) -> Result<()> {
        let model = self
            .package
            .schema_model(model_index)
            .cloned()
            .ok_or_else(|| Stop::Refused(Refusal::ValueShape(model_index)))?;
        let catalog = Catalog::composed(&model);
        let declaration = catalog
            .records
            .iter()
            .find(|(candidate, _)| candidate.as_str() == name)
            .map(|(_, value)| *value)
            .ok_or_else(|| Stop::Refused(Refusal::ValueShape(model_index)))?;
        if fields.len() != declaration.fields().len() {
            return Err(Stop::Refused(Refusal::ValueShape(model_index)));
        }
        let mut offered = Vec::new();
        offered.try_reserve(fields.len()).map_err(|_| {
            Stop::Exhausted(
                self.work
                    .allocation(Dimension::InputAggregateEntries, fields.len()),
            )
        })?;
        for field in fields {
            let path = self.export_path(&field.field, w::ExportKind::Field)?;
            if field.field.model != model_index
                || path.first().map(String::as_str) != Some(name)
                || path.len() != 2
            {
                return Err(Stop::Refused(Refusal::Field(field.field.clone())));
            }
            offered.push((field.field.export, field));
        }
        offered.sort_unstable_by_key(|(index, _)| *index);
        if let Some(duplicate) = offered
            .windows(2)
            .find(|pair| pair[0].0 == pair[1].0)
            .map(|pair| pair[0].1)
        {
            return Err(Stop::Refused(Refusal::Field(duplicate.field.clone())));
        }
        for expected_field in declaration.fields() {
            let expected_export = self.model_export(
                model_index,
                w::ExportKind::Field,
                name,
                Some(expected_field.name().as_str()),
            )?;
            let Some((_, offered)) = offered
                .binary_search_by_key(&expected_export.export, |(index, _)| *index)
                .ok()
                .and_then(|index| offered.get(index))
            else {
                return Err(Stop::Refused(Refusal::Field(w::ExportRef {
                    model: model_index,
                    export: u32::MAX,
                })));
            };
            let site = ScalarSite::Field {
                record: declaration.name().clone(),
                field: expected_field.name().clone(),
            };
            let native = catalog
                .formal(expected_field.value_type(), &site)
                .ok_or_else(|| Stop::Refused(Refusal::Field(offered.field.clone())))?;
            self.entry()?;
            match &offered.value {
                FieldValue::Compiled(slot) => {
                    let expected = self.find_wire_type(&native)?;
                    self.slot(slot, expected, depth + 1)?;
                }
                FieldValue::Contextual(slot) => {
                    self.contextual_slot(slot, &native, model_index, &offered.field, depth + 1)?;
                }
            }
        }
        Ok(())
    }

    fn contextual_slot(
        &mut self,
        slot: &ContextualSlot,
        expected: &NativeType<'_>,
        model_index: u32,
        field: &w::ExportRef,
        depth: usize,
    ) -> Result<()> {
        self.work
            .charge(Dimension::InputValueNodes, 1)
            .map_err(Stop::Exhausted)?;
        self.work
            .charge(Dimension::InputStructuralDepth, depth)
            .map_err(Stop::Exhausted)?;
        match slot {
            ContextualSlot::Unavailable(_) => Ok(()),
            ContextualSlot::Available(value) => {
                self.contextual_value(value, expected, model_index, field, depth)
            }
        }
    }

    fn contextual_value(
        &mut self,
        value: &ContextualValue,
        expected: &NativeType<'_>,
        model_index: u32,
        field: &w::ExportRef,
        depth: usize,
    ) -> Result<()> {
        let invalid = || Stop::Refused(Refusal::ContextualValue(field.clone()));
        match (expected, value.kind()) {
            (NativeType::Boolean, ContextualValueKind::Boolean(_)) => Ok(()),
            (
                NativeType::Scalar {
                    role,
                    representation,
                    ..
                },
                ContextualValueKind::Number(number),
            ) => {
                let valid = match (representation, &role.kind, number) {
                    (
                        ir::ValueType::Integer { value: domain },
                        ScalarKind::Integer { .. },
                        ProtocolNumber::Integer(actual),
                    ) => actual.value() >= domain.minimum() && actual.value() <= domain.maximum(),
                    (
                        ir::ValueType::Rational { value: domain },
                        ScalarKind::Rational { .. },
                        ProtocolNumber::Rational(actual),
                    ) => {
                        actual.numerator() >= domain.numerator_minimum()
                            && actual.numerator() <= domain.numerator_maximum()
                            && u64::try_from(actual.denominator())
                                .ok()
                                .is_some_and(|denominator| {
                                    denominator <= domain.maximum_denominator()
                                })
                    }
                    _ => false,
                };
                if valid {
                    Ok(())
                } else {
                    Err(invalid())
                }
            }
            (NativeType::Scalar { role, .. }, ContextualValueKind::Text(text))
                if matches!(role.kind, ScalarKind::Text { .. }) =>
            {
                self.work
                    .charge(Dimension::InputTextBytes, text.len())
                    .map_err(Stop::Exhausted)?;
                let ScalarKind::Text { max_scalars } = role.kind else {
                    return Err(invalid());
                };
                if u32::try_from(text.chars().count())
                    .ok()
                    .is_some_and(|count| count <= max_scalars)
                {
                    Ok(())
                } else {
                    Err(invalid())
                }
            }
            (NativeType::Enumeration { declaration, .. }, ContextualValueKind::Enum(variant)) => {
                let path = self.export_path(variant, w::ExportKind::Variant)?;
                if variant.model == model_index
                    && path.first().map(String::as_str) == Some(declaration.name().as_str())
                    && declaration.variants().iter().any(|candidate| {
                        path.get(1).map(String::as_str) == Some(candidate.name().as_str())
                    })
                {
                    Ok(())
                } else {
                    Err(invalid())
                }
            }
            (NativeType::Record { declaration, .. }, ContextualValueKind::Record(fields)) => {
                self.record_fields_by_name(model_index, declaration.name().as_str(), fields, depth)
            }
            (NativeType::Option(child), ContextualValueKind::Option(option)) => {
                if let Some(child_slot) = option {
                    self.entry()?;
                    self.contextual_slot(child_slot, child, model_index, field, depth + 1)?;
                }
                Ok(())
            }
            (NativeType::Sequence { element, maximum }, ContextualValueKind::Sequence(values)) => {
                if values.len() > *maximum as usize {
                    return Err(invalid());
                }
                for child in values {
                    self.entry()?;
                    self.contextual_slot(child, element, model_index, field, depth + 1)?;
                }
                Ok(())
            }
            (NativeType::Object { role, .. }, ContextualValueKind::Object(key)) => {
                let object = self.model_export(
                    model_index,
                    w::ExportKind::Object,
                    role.record.as_str(),
                    None,
                )?;
                let universe = self.model_export(
                    model_index,
                    w::ExportKind::Population,
                    role.record.as_str(),
                    Some(role.universe.as_str()),
                )?;
                self.key(key, model_index, &object, &universe)
            }
            (NativeType::Reference { role, .. }, ContextualValueKind::Reference(key)) => {
                let object = self.model_export(
                    model_index,
                    w::ExportKind::Object,
                    role.record.as_str(),
                    None,
                )?;
                let universe = self.model_export(
                    model_index,
                    w::ExportKind::Population,
                    role.record.as_str(),
                    Some(role.universe.as_str()),
                )?;
                self.key(key, model_index, &object, &universe)
            }
            _ => Err(invalid()),
        }
    }

    fn entry(&mut self) -> Result<()> {
        self.work
            .charge(Dimension::InputAggregateEntries, 1)
            .map_err(Stop::Exhausted)
    }

    fn key(
        &self,
        key: &ObjectKey,
        model: u32,
        object: &w::ExportRef,
        universe: &w::ExportRef,
    ) -> Result<()> {
        if key.observation.anchor.declaration != self.request.declaration
            || !valid_identity(&key.observation.snapshot.0)
            || key
                .observation
                .window
                .as_ref()
                .is_some_and(|identity| !valid_identity(&identity.0))
            || !valid_identity(&key.observation.record.0)
            || !valid_identity(&key.identifier)
            || key.model != model
            || &key.object_type != object
            || &key.universe != universe
        {
            Err(Stop::Refused(Refusal::ValueShape(self.request.value.index)))
        } else {
            Ok(())
        }
    }

    fn population_export(&self, object: &w::ExportRef) -> Result<w::ExportRef> {
        let model = self
            .package
            .package()
            .models
            .get(object.model as usize)
            .ok_or_else(|| Stop::Refused(Refusal::Field(object.clone())))?;
        let object_path = self.export_path(object, w::ExportKind::Object)?;
        let at = model
            .exports
            .iter()
            .position(|export| {
                export.kind == w::ExportKind::Population
                    && export.path.first() == object_path.first()
            })
            .ok_or_else(|| Stop::Refused(Refusal::Field(object.clone())))?;
        Ok(w::ExportRef {
            model: object.model,
            export: u32::try_from(at).map_err(|_| Stop::Refused(Refusal::Field(object.clone())))?,
        })
    }

    fn model_export(
        &self,
        model_index: u32,
        kind: w::ExportKind,
        owner: &str,
        member: Option<&str>,
    ) -> Result<w::ExportRef> {
        let model = self
            .package
            .package()
            .models
            .get(model_index as usize)
            .ok_or_else(|| {
                Stop::Refused(Refusal::Field(w::ExportRef {
                    model: model_index,
                    export: u32::MAX,
                }))
            })?;
        let export = model
            .exports
            .iter()
            .position(|export| {
                export.kind == kind
                    && export.path.first().map(String::as_str) == Some(owner)
                    && match member {
                        Some(member) => export.path.get(1).map(String::as_str) == Some(member),
                        None => export.path.len() == 1,
                    }
            })
            .and_then(|index| u32::try_from(index).ok())
            .ok_or_else(|| {
                Stop::Refused(Refusal::Field(w::ExportRef {
                    model: model_index,
                    export: u32::MAX,
                }))
            })?;
        Ok(w::ExportRef {
            model: model_index,
            export,
        })
    }

    fn export_path(&self, export: &w::ExportRef, kind: w::ExportKind) -> Result<&[String]> {
        let value = self
            .package
            .package()
            .models
            .get(export.model as usize)
            .and_then(|model| model.exports.get(export.export as usize))
            .ok_or_else(|| Stop::Refused(Refusal::Field(export.clone())))?;
        if value.kind != kind {
            return Err(Stop::Refused(Refusal::Field(export.clone())));
        }
        Ok(&value.path)
    }

    fn find_wire_type(&self, native: &NativeType<'_>) -> Result<u32> {
        self.package
            .package()
            .types
            .iter()
            .enumerate()
            .find(|(index, _)| {
                self.wire_matches_native(u32::try_from(*index).unwrap_or(u32::MAX), native)
            })
            .and_then(|(index, _)| u32::try_from(index).ok())
            .ok_or_else(|| Stop::Refused(Refusal::ValueShape(u32::MAX)))
    }

    fn wire_matches_native(&self, index: u32, native: &NativeType<'_>) -> bool {
        let Some(ty) = self.package.package().types.get(index as usize) else {
            return false;
        };
        match (ty, native) {
            (w::Type::Boolean {}, NativeType::Boolean) => true,
            (
                w::Type::Scalar {
                    export,
                    unit,
                    representation,
                },
                NativeType::Scalar { role, .. },
            ) => {
                self.export_path(export, w::ExportKind::Scalar)
                    .ok()
                    .and_then(|path| path.first())
                    .is_some_and(|name| name == role.name.as_str())
                    && unit.0.as_deref()
                        == match &role.kind {
                            ScalarKind::Integer { unit } | ScalarKind::Rational { unit } => {
                                match unit {
                                    Unit::Dimensionless => None,
                                    Unit::Named(name) => Some(name.as_str()),
                                }
                            }
                            ScalarKind::Text { .. } => None,
                        }
                    && matches!(
                        (representation, &role.kind),
                        (
                            w::Representation::Integer { .. },
                            ScalarKind::Integer { .. }
                        ) | (
                            w::Representation::Rational { .. },
                            ScalarKind::Rational { .. }
                        ) | (w::Representation::Text { .. }, ScalarKind::Text { .. })
                    )
            }
            (w::Type::Enum { export }, NativeType::Enumeration { declaration, .. }) => self
                .export_path(export, w::ExportKind::Enum)
                .ok()
                .and_then(|path| path.first())
                .is_some_and(|name| name == declaration.name().as_str()),
            (w::Type::Record { export }, NativeType::Record { declaration, .. }) => self
                .export_path(export, w::ExportKind::Record)
                .ok()
                .and_then(|path| path.first())
                .is_some_and(|name| name == declaration.name().as_str()),
            (w::Type::Object { export }, NativeType::Object { role, .. }) => self
                .export_path(export, w::ExportKind::Object)
                .ok()
                .and_then(|path| path.first())
                .is_some_and(|name| name == role.record.as_str()),
            (w::Type::Reference { export, .. }, NativeType::Reference { role, .. }) => self
                .export_path(export, w::ExportKind::Reference)
                .ok()
                .and_then(|path| path.first())
                .is_some_and(|name| name == role.reference.as_str()),
            (w::Type::Option { value }, NativeType::Option(child)) => {
                self.wire_matches_native(*value, child)
            }
            (
                w::Type::Sequence { element, maximum },
                NativeType::Sequence {
                    element: child,
                    maximum: bound,
                },
            ) => {
                wire_integer(maximum) == Some(i64::from(*bound))
                    && self.wire_matches_native(*element, child)
            }
            _ => false,
        }
    }

    fn expression(&mut self, handle: &w::Handle, depth: usize) -> Result<Value> {
        if handle.declaration as usize >= self.package.package().declarations.len() {
            return Err(Stop::Refused(Refusal::Owner(handle.clone())));
        }
        self.work
            .charge(Dimension::ExpressionWork, 1)
            .map_err(Stop::Exhausted)?;
        self.work
            .charge(Dimension::ActiveExpressionDepth, depth)
            .map_err(Stop::Exhausted)?;
        let declaration = &self.package.package().declarations[handle.declaration as usize];
        let node = declaration
            .values
            .get(handle.index as usize)
            .ok_or_else(|| Stop::Refused(Refusal::AdmittedInvariant(handle.clone())))?
            .clone();
        self.work.locus = Some(node.locus.clone());
        let kind = match node.operation {
            w::ValueOperation::Boolean { value } => ValueKind::Boolean(value),
            w::ValueOperation::Number { value } => ValueKind::Number(
                value
                    .checked()
                    .map_err(|_| Stop::Refused(Refusal::AdmittedInvariant(handle.clone())))?,
            ),
            w::ValueOperation::Text { value } => ValueKind::Text(value),
            w::ValueOperation::Enum { variant } => ValueKind::Enum(variant),
            w::ValueOperation::Read { binder } => return self.read(&binder),
            w::ValueOperation::Group { value } => return self.expression(&value, depth + 1),
            w::ValueOperation::Field { base, field } => {
                let base = self.expression(&base, depth + 1)?;
                return self.field(base, &field);
            }
            w::ValueOperation::Unary { operator, value } => {
                return self.unary(node.value_type, operator, &value, depth)
            }
            w::ValueOperation::Binary {
                operator,
                left,
                right,
            } => return self.binary(node.value_type, operator, &left, &right, depth),
            w::ValueOperation::If {
                condition,
                then_value,
                else_value,
            } => {
                let condition = self.expression(&condition, depth + 1)?;
                if boolean(&condition)? {
                    return self.expression(&then_value, depth + 1);
                } else {
                    return self.expression(&else_value, depth + 1);
                }
            }
            w::ValueOperation::Let {
                binder,
                initializer,
                body,
            } => {
                let value = self.expression(&initializer, depth + 1)?;
                let base = self.locals.len();
                self.bind_local((binder.declaration, binder.index), value)?;
                let result = self.expression(&body, depth + 1);
                self.locals.truncate(base);
                return result;
            }
            w::ValueOperation::Pre { value, .. } => return self.expression(&value, depth + 1),
            w::ValueOperation::Call {
                predicate,
                arguments,
            } => return self.call(predicate, &arguments, depth),
            w::ValueOperation::Size { collection, .. } => {
                let collection = self.expression(&collection, depth + 1)?;
                let values = sequence(&collection)?;
                ValueKind::Number(ProtocolNumber::Integer(ExactInteger::new(
                    i64::try_from(values.len())
                        .map_err(|_| Stop::Refused(Refusal::Bounds(node.value_type)))?,
                )))
            }
            w::ValueOperation::Contains { collection, member } => {
                let collection = self.expression(&collection, depth + 1)?;
                let values = sequence(&collection)?;
                let member = self.expression(&member, depth + 1)?;
                let mut found = false;
                for slot in values {
                    self.sequence_charge()?;
                    let value = available(slot.clone())?;
                    if self.equal(&value, &member)? {
                        found = true;
                        break;
                    }
                }
                ValueKind::Boolean(found)
            }
            w::ValueOperation::Query {
                operator,
                binder,
                collection,
                body,
                ..
            } => {
                return self.query(
                    node.value_type,
                    operator,
                    &binder,
                    &collection,
                    &body,
                    depth,
                )
            }
            w::ValueOperation::Parent { .. } => {
                return Err(Stop::Refused(Refusal::AdmittedInvariant(handle.clone())))
            }
            w::ValueOperation::Reaches {
                start,
                target,
                edge,
                ..
            } => {
                let start = self.expression(&start, depth + 1)?;
                let target = self.expression(&target, depth + 1)?;
                ValueKind::Boolean(self.reaches(
                    object_key(&start)?,
                    object_key(&target)?,
                    &edge,
                )?)
            }
        };
        Ok(Value::new(node.value_type, kind))
    }

    fn read(&self, binder: &w::Handle) -> Result<Value> {
        if let Some((_, value)) = self
            .locals
            .iter()
            .rev()
            .find(|(candidate, _)| *candidate == (binder.declaration, binder.index))
        {
            return Ok(value.clone());
        }
        let slot = self
            .binders
            .binary_search_by_key(&(binder.declaration, binder.index), |(candidate, _)| {
                *candidate
            })
            .ok()
            .and_then(|index| self.binders.get(index))
            .map(|(_, slot)| *slot)
            .ok_or_else(|| Stop::Refused(Refusal::MissingBinding(binder.clone())))?;
        available(slot.clone())
    }

    fn field(&mut self, base: Value, field: &w::ExportRef) -> Result<Value> {
        match base.kind() {
            ValueKind::Record(fields) => fields
                .iter()
                .find(|candidate| candidate.field == *field)
                .map(compiled_field)
                .transpose()?
                .ok_or_else(|| Stop::Refused(Refusal::Field(field.clone()))),
            ValueKind::Object(key) => {
                let object = self.find_object(key)?;
                object
                    .fields
                    .iter()
                    .find(|candidate| candidate.field == *field)
                    .map(compiled_field)
                    .transpose()?
                    .ok_or_else(|| Stop::Refused(Refusal::Field(field.clone())))
            }
            _ => Err(Stop::Refused(Refusal::Field(field.clone()))),
        }
    }

    fn unary(
        &mut self,
        result: u32,
        operator: w::Unary,
        value: &w::Handle,
        depth: usize,
    ) -> Result<Value> {
        let value = self.expression(value, depth + 1)?;
        let kind = match operator {
            w::Unary::Not => ValueKind::Boolean(!boolean(&value)?),
            w::Unary::Negate => ValueKind::Number(negate(number_value(&value)?)?),
            w::Unary::Present => {
                ValueKind::Boolean(matches!(value.kind(), ValueKind::Option(Some(_))))
            }
            w::Unary::Value => match value.kind() {
                ValueKind::Option(Some(slot)) => return available((**slot).clone()),
                _ => return Err(Stop::Refused(Refusal::ValueShape(result))),
            },
            w::Unary::Deref => {
                let key = reference_key(&value)?.clone();
                self.find_object(&key)?;
                ValueKind::Object(key)
            }
        };
        self.checked(Value::new(result, kind))
    }

    fn binary(
        &mut self,
        result: u32,
        operator: w::Binary,
        left: &w::Handle,
        right: &w::Handle,
        depth: usize,
    ) -> Result<Value> {
        let left = self.expression(left, depth + 1)?;
        if matches!(
            operator,
            w::Binary::Implies | w::Binary::Or | w::Binary::And
        ) {
            let a = boolean(&left)?;
            if (operator == w::Binary::Implies && !a)
                || (operator == w::Binary::Or && a)
                || (operator == w::Binary::And && !a)
            {
                return Ok(Value::new(
                    result,
                    ValueKind::Boolean(operator != w::Binary::And),
                ));
            }
        }
        let right = self.expression(right, depth + 1)?;
        let kind = match operator {
            w::Binary::Implies | w::Binary::Or | w::Binary::And => {
                ValueKind::Boolean(boolean(&right)?)
            }
            w::Binary::Equal | w::Binary::NotEqual => {
                ValueKind::Boolean(self.equal(&left, &right)? == (operator == w::Binary::Equal))
            }
            w::Binary::Less
            | w::Binary::LessEqual
            | w::Binary::Greater
            | w::Binary::GreaterEqual => {
                let order = compare(number_value(&left)?, number_value(&right)?)?;
                ValueKind::Boolean(match operator {
                    w::Binary::Less => order.is_lt(),
                    w::Binary::LessEqual => order.is_le(),
                    w::Binary::Greater => order.is_gt(),
                    _ => order.is_ge(),
                })
            }
            w::Binary::Add
            | w::Binary::Subtract
            | w::Binary::Multiply
            | w::Binary::RationalDivide => ValueKind::Number(arithmetic(
                operator,
                number_value(&left)?,
                number_value(&right)?,
            )?),
        };
        self.checked(Value::new(result, kind))
    }

    fn checked(&mut self, value: Value) -> Result<Value> {
        let ty = self
            .package
            .package()
            .types
            .get(value.value_type as usize)
            .ok_or_else(|| Stop::Refused(Refusal::ValueShape(value.value_type)))?;
        match (ty, value.kind()) {
            (w::Type::Scalar { representation, .. }, ValueKind::Number(number)) => {
                self.number(value.value_type, representation, number)?;
            }
            (w::Type::Sequence { maximum, .. }, ValueKind::Sequence(values))
                if i64::try_from(values.len()).ok().is_some_and(|length| {
                    wire_integer(maximum).is_some_and(|bound| length <= bound)
                }) => {}
            (w::Type::Boolean {}, ValueKind::Boolean(_))
            | (w::Type::Enum { .. }, ValueKind::Enum(_))
            | (w::Type::Object { .. }, ValueKind::Object(_))
            | (w::Type::Reference { .. }, ValueKind::Reference(_)) => {}
            _ => return Err(Stop::Refused(Refusal::ValueShape(value.value_type))),
        }
        Ok(value)
    }

    fn equal(&mut self, left: &Value, right: &Value) -> Result<bool> {
        self.work
            .charge(Dimension::ValueComparison, 1)
            .map_err(Stop::Exhausted)?;
        if left.value_type != right.value_type {
            return Err(Stop::Refused(Refusal::Type {
                expected: left.value_type,
                actual: right.value_type,
            }));
        }
        match (left.kind(), right.kind()) {
            (ValueKind::Boolean(a), ValueKind::Boolean(b)) => Ok(a == b),
            (ValueKind::Number(a), ValueKind::Number(b)) => Ok(a == b),
            (ValueKind::Text(a), ValueKind::Text(b)) => Ok(a == b),
            (ValueKind::Enum(a), ValueKind::Enum(b)) => Ok(a == b),
            (ValueKind::Reference(a), ValueKind::Reference(b))
            | (ValueKind::Object(a), ValueKind::Object(b)) => Ok(identity_equal(a, b)),
            _ => Err(Stop::Refused(Refusal::ValueShape(left.value_type))),
        }
    }

    fn call(&mut self, predicate: u32, arguments: &[w::Handle], depth: usize) -> Result<Value> {
        let declaration = self
            .package
            .package()
            .declarations
            .get(predicate as usize)
            .ok_or_else(|| Stop::Refused(Refusal::RequestDeclaration(predicate)))?
            .clone();
        let w::Body::Predicate {
            parameters, root, ..
        } = declaration.body
        else {
            return Err(Stop::Refused(Refusal::RequestDeclaration(predicate)));
        };
        let next = self.call_depth.checked_add(1).ok_or_else(|| {
            Stop::Exhausted(counter_overflow(&self.work, Dimension::PredicateCallDepth))
        })?;
        self.work
            .charge(Dimension::PredicateCallDepth, next)
            .map_err(Stop::Exhausted)?;
        self.call_depth = next;
        let base = self.locals.len();
        let result = (|| {
            for (argument, parameter) in arguments.iter().zip(&parameters) {
                let value = self.expression(argument, depth + 1)?;
                self.bind_local((parameter.declaration, parameter.index), value)?;
            }
            self.expression(&root, depth + 1)
        })();
        self.locals.truncate(base);
        self.call_depth -= 1;
        result
    }

    fn query(
        &mut self,
        result_type: u32,
        operator: w::Query,
        binder: &w::Handle,
        collection: &w::Handle,
        body: &w::Handle,
        depth: usize,
    ) -> Result<Value> {
        let collection = self.expression(collection, depth + 1)?;
        let values = sequence(&collection)?;
        let mut output = Vec::new();
        let mut count = 0_i64;
        let mut sum = if operator == w::Query::Sum {
            Some(zero_for(
                self.package
                    .package()
                    .types
                    .get(result_type as usize)
                    .ok_or_else(|| Stop::Refused(Refusal::ValueShape(result_type)))?,
            )?)
        } else {
            None
        };
        for slot in values {
            self.sequence_charge()?;
            let item = available(slot.clone())?;
            let base = self.locals.len();
            self.bind_local((binder.declaration, binder.index), item.clone())?;
            let projected = self.expression(body, depth + 1);
            self.locals.truncate(base);
            let projected = projected?;
            match operator {
                w::Query::ForAll => {
                    if !boolean(&projected)? {
                        return Ok(Value::new(result_type, ValueKind::Boolean(false)));
                    }
                }
                w::Query::Exists => {
                    if boolean(&projected)? {
                        return Ok(Value::new(result_type, ValueKind::Boolean(true)));
                    }
                }
                w::Query::Filter => {
                    if boolean(&projected)? {
                        self.retain(&mut output, slot.clone())?;
                    }
                }
                w::Query::Map => self.retain(&mut output, InputSlot::Available(projected))?,
                w::Query::Count => {
                    if boolean(&projected)? {
                        count = count
                            .checked_add(1)
                            .ok_or_else(|| Stop::Refused(Refusal::Bounds(result_type)))?;
                    }
                }
                w::Query::Sum => {
                    let next = arithmetic(
                        w::Binary::Add,
                        sum.as_ref().ok_or_else(|| {
                            Stop::Refused(Refusal::AdmittedInvariant(self.request.value.clone()))
                        })?,
                        number_value(&projected)?,
                    )?;
                    self.number_for_type(result_type, &next)?;
                    sum = Some(next);
                }
            }
        }
        let kind = match operator {
            w::Query::ForAll => ValueKind::Boolean(true),
            w::Query::Exists => ValueKind::Boolean(false),
            w::Query::Filter | w::Query::Map => ValueKind::Sequence(output),
            w::Query::Count => ValueKind::Number(ProtocolNumber::Integer(ExactInteger::new(count))),
            w::Query::Sum => ValueKind::Number(sum.ok_or_else(|| {
                Stop::Refused(Refusal::AdmittedInvariant(self.request.value.clone()))
            })?),
        };
        self.checked(Value::new(result_type, kind))
    }

    fn sequence_charge(&mut self) -> Result<()> {
        self.work
            .charge(Dimension::SequenceWork, 1)
            .map_err(Stop::Exhausted)
    }
    fn retain(&mut self, output: &mut Vec<InputSlot>, value: InputSlot) -> Result<()> {
        self.work
            .charge(Dimension::RetainedOutput, 1)
            .map_err(Stop::Exhausted)?;
        output
            .try_reserve(1)
            .map_err(|_| Stop::Exhausted(self.work.allocation(Dimension::RetainedOutput, 1)))?;
        output.push(value);
        Ok(())
    }
    fn bind_local(&mut self, key: (u32, u32), value: Value) -> Result<()> {
        self.work
            .charge(Dimension::RetainedOutput, 1)
            .map_err(Stop::Exhausted)?;
        self.locals
            .try_reserve(1)
            .map_err(|_| Stop::Exhausted(self.work.allocation(Dimension::RetainedOutput, 1)))?;
        self.locals.push((key, value));
        Ok(())
    }
    fn number_for_type(&self, ty: u32, number: &ProtocolNumber) -> Result<()> {
        let w::Type::Scalar { representation, .. } = self
            .package
            .package()
            .types
            .get(ty as usize)
            .ok_or_else(|| Stop::Refused(Refusal::ValueShape(ty)))?
        else {
            return Err(Stop::Refused(Refusal::ValueShape(ty)));
        };
        self.number(ty, representation, number)
    }

    fn find_object(&self, key: &ObjectKey) -> Result<&'a ObjectInput> {
        let population = self
            .populations
            .iter()
            .find(|population| population.domain_matches(key))
            .ok_or_else(|| Stop::Refused(Refusal::PopulationDomain(key.clone())))?;
        population
            .object(key)
            .ok_or_else(|| Stop::Refused(Refusal::Dangling(key.clone())))
    }

    fn object_position(&self, key: &ObjectKey) -> Result<usize> {
        let population = self
            .populations
            .iter()
            .find(|population| population.domain_matches(key))
            .ok_or_else(|| Stop::Refused(Refusal::PopulationDomain(key.clone())))?;
        let local = population
            .object_index(key)
            .map_err(|_| Stop::Refused(Refusal::Dangling(key.clone())))?;
        population
            .offset
            .checked_add(local)
            .ok_or_else(|| Stop::Exhausted(counter_overflow(&self.work, Dimension::GraphExpansion)))
    }

    fn reaches(
        &mut self,
        start: &ObjectKey,
        target: &ObjectKey,
        edge: &w::ExportRef,
    ) -> Result<bool> {
        if !same_graph(start, target) {
            return Err(Stop::Refused(Refusal::ValueShape(self.request.value.index)));
        }
        let start = &self.find_object(start)?.key;
        let object_count = self
            .populations
            .iter()
            .try_fold(0_usize, |total, population| {
                total.checked_add(population.objects.len()).ok_or_else(|| {
                    Stop::Exhausted(counter_overflow(&self.work, Dimension::GraphExpansion))
                })
            })?;
        let mut expanded = Vec::new();
        expanded.try_reserve(object_count).map_err(|_| {
            Stop::Exhausted(
                self.work
                    .allocation(Dimension::GraphExpansion, object_count),
            )
        })?;
        expanded.resize(object_count, false);
        self.dfs(start, target, edge, 1, &mut expanded)
    }

    fn dfs(
        &mut self,
        current: &'a ObjectKey,
        target: &ObjectKey,
        edge: &w::ExportRef,
        depth: usize,
        expanded: &mut [bool],
    ) -> Result<bool> {
        let position = self.object_position(current)?;
        let expanded_entry = expanded.get_mut(position).ok_or_else(|| {
            Stop::Exhausted(counter_overflow(&self.work, Dimension::GraphExpansion))
        })?;
        if *expanded_entry {
            return Ok(false);
        }
        self.work
            .charge(Dimension::ActiveGraphDepth, depth)
            .map_err(Stop::Exhausted)?;
        self.work
            .charge(Dimension::GraphExpansion, 1)
            .map_err(Stop::Exhausted)?;
        *expanded_entry = true;
        let object = self.find_object(current)?;
        let field = object
            .fields
            .iter()
            .find(|field| field.field == *edge)
            .ok_or_else(|| Stop::Refused(Refusal::Field(edge.clone())))?;
        match &field.value {
            FieldValue::Compiled(slot) => self.scan_references(slot, target, edge, depth, expanded),
            FieldValue::Contextual(slot) => {
                self.scan_contextual_references(slot, target, edge, depth, expanded)
            }
        }
    }

    fn scan_references(
        &mut self,
        slot: &'a InputSlot,
        target: &ObjectKey,
        edge: &w::ExportRef,
        depth: usize,
        expanded: &mut [bool],
    ) -> Result<bool> {
        let value = available_ref(slot)?;
        match value.kind() {
            ValueKind::Reference(next) => {
                self.work
                    .charge(Dimension::GraphEdges, 1)
                    .map_err(Stop::Exhausted)?;
                self.work
                    .charge(Dimension::ValueComparison, 1)
                    .map_err(Stop::Exhausted)?;
                if next == target {
                    return Ok(true);
                }
                if self.dfs(next, target, edge, depth + 1, expanded)? {
                    return Ok(true);
                }
                Ok(false)
            }
            ValueKind::Option(None) => Ok(false),
            ValueKind::Option(Some(child)) => {
                self.scan_references(child, target, edge, depth, expanded)
            }
            ValueKind::Sequence(children) => {
                for child in children {
                    if self.scan_references(child, target, edge, depth, expanded)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            _ => Err(Stop::Refused(Refusal::ValueShape(value.value_type))),
        }
    }

    fn scan_contextual_references(
        &mut self,
        slot: &'a ContextualSlot,
        target: &ObjectKey,
        edge: &w::ExportRef,
        depth: usize,
        expanded: &mut [bool],
    ) -> Result<bool> {
        let value = contextual_available_ref(slot)?;
        match value.kind() {
            ContextualValueKind::Reference(next) => {
                self.work
                    .charge(Dimension::GraphEdges, 1)
                    .map_err(Stop::Exhausted)?;
                self.work
                    .charge(Dimension::ValueComparison, 1)
                    .map_err(Stop::Exhausted)?;
                if next == target {
                    return Ok(true);
                }
                if self.dfs(next, target, edge, depth + 1, expanded)? {
                    return Ok(true);
                }
                Ok(false)
            }
            ContextualValueKind::Option(None) => Ok(false),
            ContextualValueKind::Option(Some(child)) => {
                self.scan_contextual_references(child, target, edge, depth, expanded)
            }
            ContextualValueKind::Sequence(children) => {
                for child in children {
                    if self.scan_contextual_references(child, target, edge, depth, expanded)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            _ => Err(Stop::Refused(Refusal::ContextualValue(edge.clone()))),
        }
    }
}

fn valid_digest(value: &CanonicalDigest) -> bool {
    value.algorithm == "sha256"
        && value.domain == "filament-canonical-json-1"
        && value.value.len() == 71
        && value.value.starts_with("sha256:")
        && value
            .value
            .strip_prefix("sha256:")
            .unwrap_or_default()
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn valid_observation_digest(value: &ObservationDigest) -> bool {
    valid_identity(&value.0)
}

fn valid_identity(value: &str) -> bool {
    !value.is_empty() && value.len() <= 4096
}

struct RequiredInputs {
    binders: Vec<w::Handle>,
    populations: Vec<w::Handle>,
}

fn required_inputs(
    work: &mut Work,
    package: &w::Package,
    root: &w::Handle,
) -> Result<RequiredInputs> {
    let mut pending = Vec::new();
    discovery_push(work, &mut pending, root)?;
    let mut visited = boolean_table(
        work,
        package
            .declarations
            .iter()
            .map(|declaration| declaration.values.len()),
    )?;
    let mut required_binders = boolean_table(
        work,
        package
            .declarations
            .iter()
            .map(|declaration| declaration.binders.len()),
    )?;
    let mut required_populations = boolean_table(
        work,
        package
            .declarations
            .iter()
            .map(|declaration| declaration.bindings.len()),
    )?;
    while let Some(handle) = pending.pop() {
        let seen = visited
            .get_mut(handle.declaration as usize)
            .and_then(|declaration| declaration.get_mut(handle.index as usize))
            .ok_or_else(|| Stop::Refused(Refusal::AdmittedInvariant(handle.clone())))?;
        if *seen {
            continue;
        }
        work.charge(Dimension::InputAggregateEntries, 1)
            .map_err(Stop::Exhausted)?;
        *seen = true;
        let node = package
            .declarations
            .get(handle.declaration as usize)
            .and_then(|declaration| declaration.values.get(handle.index as usize))
            .ok_or_else(|| Stop::Refused(Refusal::AdmittedInvariant(handle.clone())))?;
        match &node.operation {
            w::ValueOperation::Read { binder } if binder.declaration == root.declaration => {
                work.charge(Dimension::InputAggregateEntries, 1)
                    .map_err(Stop::Exhausted)?;
                let declaration = &package.declarations[binder.declaration as usize];
                let value = declaration
                    .binders
                    .get(binder.index as usize)
                    .ok_or_else(|| Stop::Refused(Refusal::AdmittedInvariant(binder.clone())))?;
                if value.initializer.0.is_none()
                    && !matches!(value.kind, w::BinderKind::Let | w::BinderKind::Query)
                {
                    let selected = required_binders
                        .get_mut(binder.declaration as usize)
                        .and_then(|declaration| declaration.get_mut(binder.index as usize))
                        .ok_or_else(|| Stop::Refused(Refusal::AdmittedInvariant(binder.clone())))?;
                    if !*selected {
                        work.charge(Dimension::InputAggregateEntries, 1)
                            .map_err(Stop::Exhausted)?;
                        *selected = true;
                    }
                }
            }
            w::ValueOperation::Group { value } | w::ValueOperation::Pre { value, .. } => {
                discovery_push(work, &mut pending, value)?
            }
            w::ValueOperation::Unary { operator, value } => {
                if *operator == w::Unary::Deref {
                    require_population_for_value(work, package, value, &mut required_populations)?;
                }
                discovery_push(work, &mut pending, value)?;
            }
            w::ValueOperation::Field { base, .. } => {
                require_population_for_value(work, package, base, &mut required_populations)?;
                discovery_push(work, &mut pending, base)?;
            }
            w::ValueOperation::Binary { left, right, .. } => {
                discovery_push(work, &mut pending, left)?;
                discovery_push(work, &mut pending, right)?;
            }
            w::ValueOperation::If {
                condition,
                then_value,
                else_value,
            } => {
                discovery_push(work, &mut pending, condition)?;
                discovery_push(work, &mut pending, then_value)?;
                discovery_push(work, &mut pending, else_value)?;
            }
            w::ValueOperation::Let {
                initializer, body, ..
            } => {
                discovery_push(work, &mut pending, initializer)?;
                discovery_push(work, &mut pending, body)?;
            }
            w::ValueOperation::Call {
                predicate,
                arguments,
            } => {
                for argument in arguments {
                    discovery_push(work, &mut pending, argument)?;
                }
                let declaration = package
                    .declarations
                    .get(*predicate as usize)
                    .ok_or_else(|| Stop::Refused(Refusal::RequestDeclaration(*predicate)))?;
                let w::Body::Predicate { root, .. } = &declaration.body else {
                    return Err(Stop::Refused(Refusal::RequestDeclaration(*predicate)));
                };
                discovery_push(work, &mut pending, root)?;
            }
            w::ValueOperation::Size { collection, .. } => {
                discovery_push(work, &mut pending, collection)?
            }
            w::ValueOperation::Contains { collection, member } => {
                discovery_push(work, &mut pending, collection)?;
                discovery_push(work, &mut pending, member)?;
            }
            w::ValueOperation::Query {
                collection, body, ..
            } => {
                discovery_push(work, &mut pending, collection)?;
                discovery_push(work, &mut pending, body)?;
            }
            w::ValueOperation::Parent { reference, .. } => {
                discovery_push(work, &mut pending, reference)?
            }
            w::ValueOperation::Reaches {
                start,
                target,
                universe,
                ..
            } => {
                let start_value = package
                    .declarations
                    .get(start.declaration as usize)
                    .and_then(|declaration| declaration.values.get(start.index as usize))
                    .ok_or_else(|| Stop::Refused(Refusal::AdmittedInvariant(start.clone())))?;
                require_population(
                    work,
                    package,
                    start.declaration,
                    universe,
                    &start_value.anchor,
                    &mut required_populations,
                )?;
                discovery_push(work, &mut pending, start)?;
                discovery_push(work, &mut pending, target)?;
            }
            w::ValueOperation::Boolean { .. }
            | w::ValueOperation::Number { .. }
            | w::ValueOperation::Text { .. }
            | w::ValueOperation::Enum { .. }
            | w::ValueOperation::Read { .. } => {}
        }
    }
    Ok(RequiredInputs {
        binders: selected_handles(work, required_binders)?,
        populations: selected_handles(work, required_populations)?,
    })
}

fn boolean_table(work: &Work, lengths: impl Iterator<Item = usize>) -> Result<Vec<Vec<bool>>> {
    let mut table = Vec::new();
    for length in lengths {
        table
            .try_reserve(1)
            .map_err(|_| Stop::Exhausted(work.allocation(Dimension::InputAggregateEntries, 1)))?;
        let mut row = Vec::new();
        row.try_reserve(length).map_err(|_| {
            Stop::Exhausted(work.allocation(Dimension::InputAggregateEntries, length))
        })?;
        row.resize(length, false);
        table.push(row);
    }
    Ok(table)
}

fn selected_handles(work: &Work, selected: Vec<Vec<bool>>) -> Result<Vec<w::Handle>> {
    let count = selected.iter().try_fold(0_usize, |total, values| {
        total
            .checked_add(values.iter().filter(|value| **value).count())
            .ok_or_else(|| {
                Stop::Exhausted(counter_overflow(work, Dimension::InputAggregateEntries))
            })
    })?;
    let mut handles = Vec::new();
    handles
        .try_reserve(count)
        .map_err(|_| Stop::Exhausted(work.allocation(Dimension::InputAggregateEntries, count)))?;
    for (declaration, values) in selected.into_iter().enumerate() {
        for (index, value) in values.into_iter().enumerate() {
            if value {
                handles.push(w::Handle {
                    declaration: u32::try_from(declaration).map_err(|_| {
                        Stop::Exhausted(counter_overflow(work, Dimension::InputAggregateEntries))
                    })?,
                    index: u32::try_from(index).map_err(|_| {
                        Stop::Exhausted(counter_overflow(work, Dimension::InputAggregateEntries))
                    })?,
                });
            }
        }
    }
    Ok(handles)
}

fn selected_handle(selected: &[w::Handle], handle: &w::Handle) -> bool {
    selected
        .binary_search_by(|candidate| handle_cmp(candidate, handle))
        .is_ok()
}

fn handle_cmp(left: &w::Handle, right: &w::Handle) -> Ordering {
    (left.declaration, left.index).cmp(&(right.declaration, right.index))
}

fn require_population_for_value(
    work: &mut Work,
    package: &w::Package,
    value: &w::Handle,
    selected: &mut [Vec<bool>],
) -> Result<()> {
    let value_node = package
        .declarations
        .get(value.declaration as usize)
        .and_then(|declaration| declaration.values.get(value.index as usize))
        .ok_or_else(|| Stop::Refused(Refusal::AdmittedInvariant(value.clone())))?;
    let ty = package
        .types
        .get(value_node.value_type as usize)
        .ok_or_else(|| Stop::Refused(Refusal::AdmittedInvariant(value.clone())))?;
    let universe = match ty {
        w::Type::Reference { universe, .. } => Some(universe.clone()),
        w::Type::Object { export } => population_export(package, export),
        _ => None,
    };
    if let Some(universe) = universe {
        require_population(
            work,
            package,
            value.declaration,
            &universe,
            &value_node.anchor,
            selected,
        )?;
    }
    Ok(())
}

fn require_population(
    work: &mut Work,
    package: &w::Package,
    declaration: u32,
    universe: &w::ExportRef,
    anchor: &w::Handle,
    selected: &mut [Vec<bool>],
) -> Result<()> {
    let declaration_value = package
        .declarations
        .get(declaration as usize)
        .ok_or_else(|| Stop::Refused(Refusal::RequestDeclaration(declaration)))?;
    let mut matches = declaration_value
        .bindings
        .iter()
        .enumerate()
        .filter(|(_, binding)| {
            binding.kind == w::BindingKind::Population
                && binding.model.0.as_ref() == Some(universe)
                && &binding.anchor == anchor
        });
    let Some((index, _)) = matches.next() else {
        return Err(Stop::Refused(Refusal::AdmittedInvariant(anchor.clone())));
    };
    if matches.next().is_some() {
        return Err(Stop::Refused(Refusal::AdmittedInvariant(anchor.clone())));
    }
    let selected = selected
        .get_mut(declaration as usize)
        .and_then(|bindings| bindings.get_mut(index))
        .ok_or_else(|| Stop::Refused(Refusal::AdmittedInvariant(anchor.clone())))?;
    if !*selected {
        work.charge(Dimension::InputAggregateEntries, 1)
            .map_err(Stop::Exhausted)?;
        *selected = true;
    }
    Ok(())
}

fn population_export(package: &w::Package, object: &w::ExportRef) -> Option<w::ExportRef> {
    let model = package.models.get(object.model as usize)?;
    let object_path = model.exports.get(object.export as usize)?.path.first()?;
    let export = model.exports.iter().position(|export| {
        export.kind == w::ExportKind::Population && export.path.first() == Some(object_path)
    })?;
    Some(w::ExportRef {
        model: object.model,
        export: u32::try_from(export).ok()?,
    })
}

fn discovery_push(work: &mut Work, pending: &mut Vec<w::Handle>, handle: &w::Handle) -> Result<()> {
    work.charge(Dimension::InputAggregateEntries, 1)
        .map_err(Stop::Exhausted)?;
    pending
        .try_reserve(1)
        .map_err(|_| Stop::Exhausted(work.allocation(Dimension::InputAggregateEntries, 1)))?;
    pending.push(handle.clone());
    Ok(())
}

fn valid_adapter(value: &w::ArtifactRef) -> bool {
    value.ref_version == "ix.artifact-ref/3-draft"
        && value.kind == w::ArtifactKind::Binding
        && value.wire.identity == "quire.state.authority-adapter"
        && value.wire.version == "1"
        && !value.authority.is_empty()
        && !value.identity.is_empty()
        && !value.revision.namespace.is_empty()
        && !value.revision.value.is_empty()
}

fn wire_integer(value: &w::Integer) -> Option<i64> {
    match value.checked().ok()? {
        ProtocolNumber::Integer(value) => Some(value.value()),
        ProtocolNumber::Rational(_) => None,
    }
}

fn export_model(value: &w::ExportRef) -> u32 {
    value.model
}

fn available(slot: InputSlot) -> Result<Value> {
    match slot {
        InputSlot::Available(value) => Ok(value),
        InputSlot::Unavailable(missing) => Err(Stop::Incomplete(missing)),
    }
}

fn compiled_field(field: &FieldInput) -> Result<Value> {
    match &field.value {
        FieldValue::Compiled(slot) => available(slot.clone()),
        FieldValue::Contextual(_) => {
            Err(Stop::Refused(Refusal::ContextualValue(field.field.clone())))
        }
    }
}

fn ensure_available(value: &Value) -> Result<()> {
    match value.kind() {
        ValueKind::Record(fields) => {
            for field in fields {
                match &field.value {
                    FieldValue::Compiled(slot) => {
                        let child = available_ref(slot)?;
                        ensure_available(child)?;
                    }
                    FieldValue::Contextual(slot) => ensure_contextual_available(slot)?,
                }
            }
        }
        ValueKind::Option(Some(slot)) => {
            let child = available_ref(slot)?;
            ensure_available(child)?;
        }
        ValueKind::Sequence(values) => {
            for slot in values {
                let child = available_ref(slot)?;
                ensure_available(child)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn ensure_contextual_available(slot: &ContextualSlot) -> Result<()> {
    let value = contextual_available_ref(slot)?;
    match value.kind() {
        ContextualValueKind::Record(fields) => {
            for field in fields {
                match &field.value {
                    FieldValue::Compiled(slot) => ensure_available(available_ref(slot)?)?,
                    FieldValue::Contextual(slot) => ensure_contextual_available(slot)?,
                }
            }
        }
        ContextualValueKind::Option(Some(child)) => ensure_contextual_available(child)?,
        ContextualValueKind::Sequence(children) => {
            for child in children {
                ensure_contextual_available(child)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn boolean(value: &Value) -> Result<bool> {
    if let ValueKind::Boolean(value) = value.kind() {
        Ok(*value)
    } else {
        Err(Stop::Refused(Refusal::ValueShape(value.value_type)))
    }
}
fn number_value(value: &Value) -> Result<&ProtocolNumber> {
    if let ValueKind::Number(value) = value.kind() {
        Ok(value)
    } else {
        Err(Stop::Refused(Refusal::ValueShape(value.value_type)))
    }
}
fn sequence(value: &Value) -> Result<&[InputSlot]> {
    if let ValueKind::Sequence(value) = value.kind() {
        Ok(value)
    } else {
        Err(Stop::Refused(Refusal::ValueShape(value.value_type)))
    }
}
fn object_key(value: &Value) -> Result<&ObjectKey> {
    match value.kind() {
        ValueKind::Object(key) | ValueKind::Reference(key) => Ok(key),
        _ => Err(Stop::Refused(Refusal::ValueShape(value.value_type))),
    }
}
fn reference_key(value: &Value) -> Result<&ObjectKey> {
    if let ValueKind::Reference(key) = value.kind() {
        Ok(key)
    } else {
        Err(Stop::Refused(Refusal::ValueShape(value.value_type)))
    }
}

fn identity_equal(left: &ObjectKey, right: &ObjectKey) -> bool {
    left.model == right.model
        && left.universe == right.universe
        && left.object_type == right.object_type
        && left.identifier == right.identifier
}
fn same_graph(left: &ObjectKey, right: &ObjectKey) -> bool {
    left.model == right.model
        && left.universe == right.universe
        && left.object_type == right.object_type
        && left.observation.anchor == right.observation.anchor
        && left.observation.snapshot == right.observation.snapshot
        && left.observation.window == right.observation.window
}

fn object_key_cmp(left: &ObjectKey, right: &ObjectKey) -> Ordering {
    (
        left.observation.anchor.declaration,
        left.observation.anchor.index,
        left.observation.snapshot.0.as_str(),
        left.observation
            .window
            .as_ref()
            .map(|value| value.0.as_str()),
        left.observation.record.0.as_str(),
        left.model,
        left.universe.model,
        left.universe.export,
        left.object_type.model,
        left.object_type.export,
        left.identifier.as_str(),
    )
        .cmp(&(
            right.observation.anchor.declaration,
            right.observation.anchor.index,
            right.observation.snapshot.0.as_str(),
            right
                .observation
                .window
                .as_ref()
                .map(|value| value.0.as_str()),
            right.observation.record.0.as_str(),
            right.model,
            right.universe.model,
            right.universe.export,
            right.object_type.model,
            right.object_type.export,
            right.identifier.as_str(),
        ))
}

fn available_ref(slot: &InputSlot) -> Result<&Value> {
    match slot {
        InputSlot::Available(value) => Ok(value),
        InputSlot::Unavailable(missing) => Err(Stop::Incomplete(missing.clone())),
    }
}

fn contextual_available_ref(slot: &ContextualSlot) -> Result<&ContextualValue> {
    match slot {
        ContextualSlot::Available(value) => Ok(value),
        ContextualSlot::Unavailable(missing) => Err(Stop::Incomplete(missing.clone())),
    }
}

fn zero_for(ty: &w::Type) -> Result<ProtocolNumber> {
    match ty {
        w::Type::Scalar {
            representation: w::Representation::Integer { .. },
            ..
        } => Ok(ProtocolNumber::Integer(ExactInteger::new(0))),
        w::Type::Scalar {
            representation: w::Representation::Rational { .. },
            ..
        } => Ok(ProtocolNumber::Rational(
            ExactRational::new(0, 1).map_err(|_| Stop::Refused(Refusal::ValueShape(u32::MAX)))?,
        )),
        _ => Err(Stop::Refused(Refusal::ValueShape(u32::MAX))),
    }
}

fn negate(value: &ProtocolNumber) -> Result<ProtocolNumber> {
    match value {
        ProtocolNumber::Integer(value) => value
            .value()
            .checked_neg()
            .map(ExactInteger::new)
            .map(ProtocolNumber::Integer)
            .ok_or_else(|| Stop::Refused(Refusal::Bounds(u32::MAX))),
        ProtocolNumber::Rational(value) => value
            .numerator()
            .checked_neg()
            .and_then(|n| ExactRational::new(n, value.denominator()).ok())
            .map(ProtocolNumber::Rational)
            .ok_or_else(|| Stop::Refused(Refusal::Bounds(u32::MAX))),
    }
}

fn compare(left: &ProtocolNumber, right: &ProtocolNumber) -> Result<Ordering> {
    match (left, right) {
        (ProtocolNumber::Integer(a), ProtocolNumber::Integer(b)) => Ok(a.value().cmp(&b.value())),
        (ProtocolNumber::Rational(a), ProtocolNumber::Rational(b)) => {
            Ok((i128::from(a.numerator()) * i128::from(b.denominator()))
                .cmp(&(i128::from(b.numerator()) * i128::from(a.denominator()))))
        }
        _ => Err(Stop::Refused(Refusal::ValueShape(u32::MAX))),
    }
}

fn arithmetic(
    operator: w::Binary,
    left: &ProtocolNumber,
    right: &ProtocolNumber,
) -> Result<ProtocolNumber> {
    match (left, right) {
        (ProtocolNumber::Integer(a), ProtocolNumber::Integer(b)) => {
            let value = match operator {
                w::Binary::Add => a.value().checked_add(b.value()),
                w::Binary::Subtract => a.value().checked_sub(b.value()),
                w::Binary::Multiply => a.value().checked_mul(b.value()),
                _ => None,
            };
            value
                .map(ExactInteger::new)
                .map(ProtocolNumber::Integer)
                .ok_or_else(|| Stop::Refused(Refusal::Bounds(u32::MAX)))
        }
        (ProtocolNumber::Rational(a), ProtocolNumber::Rational(b)) => {
            let (an, ad, bn, bd) = (
                i128::from(a.numerator()),
                i128::from(a.denominator()),
                i128::from(b.numerator()),
                i128::from(b.denominator()),
            );
            let pair = match operator {
                w::Binary::Add => an
                    .checked_mul(bd)
                    .and_then(|left| bn.checked_mul(ad).and_then(|right| left.checked_add(right)))
                    .zip(ad.checked_mul(bd)),
                w::Binary::Subtract => an
                    .checked_mul(bd)
                    .and_then(|left| bn.checked_mul(ad).and_then(|right| left.checked_sub(right)))
                    .zip(ad.checked_mul(bd)),
                w::Binary::Multiply => an.checked_mul(bn).zip(ad.checked_mul(bd)),
                w::Binary::RationalDivide if bn != 0 => an.checked_mul(bd).zip(ad.checked_mul(bn)),
                _ => None,
            }
            .ok_or_else(|| Stop::Refused(Refusal::Bounds(u32::MAX)))?;
            rational(pair.0, pair.1)
        }
        _ => Err(Stop::Refused(Refusal::ValueShape(u32::MAX))),
    }
}

fn rational(mut numerator: i128, mut denominator: i128) -> Result<ProtocolNumber> {
    if denominator < 0 {
        numerator = -numerator;
        denominator = -denominator;
    }
    let divisor = gcd(numerator.unsigned_abs(), denominator.unsigned_abs());
    numerator /= i128::try_from(divisor).map_err(|_| Stop::Refused(Refusal::Bounds(u32::MAX)))?;
    denominator /= i128::try_from(divisor).map_err(|_| Stop::Refused(Refusal::Bounds(u32::MAX)))?;
    let numerator =
        i64::try_from(numerator).map_err(|_| Stop::Refused(Refusal::Bounds(u32::MAX)))?;
    let denominator =
        i64::try_from(denominator).map_err(|_| Stop::Refused(Refusal::Bounds(u32::MAX)))?;
    ExactRational::new(numerator, denominator)
        .map(ProtocolNumber::Rational)
        .map_err(|_| Stop::Refused(Refusal::Bounds(u32::MAX)))
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        (left, right) = (right, left % right);
    }
    left
}

fn counter_overflow(work: &Work, dimension: Dimension) -> Exhaustion {
    Exhaustion {
        dimension,
        cause: super::ExhaustionCause::CounterOverflow,
        used: work.usage.predicate_call_depth,
        requested: 1,
        limit: work.limits.predicate_call_depth,
        locus: work.locus.clone(),
    }
}
