// SPDX-License-Identifier: AGPL-3.0-only
//! FR-007: all supplied typed roots, captured observations and finite closure.

use super::super::{FieldBinding, ObjectIdentity, QualifiedName, ValueId, ValueNode};
use super::budget::{Budget, Result, Stage};
use super::{
    snapshot_key, BoundModels, ObservationSelection, PopulationIndexes, PopulationKey,
    RuntimePathSegment, Validator,
};
use crate::checking::{NativeType, Observation};
use crate::linking::{DeclarationIdentity, DeclarationKey, DeclarationLocation, ResolutionTarget};
use crate::native_model::{NativeModel, ObjectRole, ScalarKind, ScalarSite};
use crate::syntax::ClauseKind;
use crate::Code;
use quire_contract_ir as ir;
use std::collections::BTreeSet;

pub(super) fn locus(
    model: &NativeModel,
    key: DeclarationKey,
    source: &ir::SourceSpan,
) -> DeclarationLocation {
    DeclarationLocation {
        identity: DeclarationIdentity {
            owner: model.environment().owner().clone(),
            key,
        },
        source: source.clone(),
    }
}

pub(super) fn require_population<F: FnMut() -> bool>(
    budget: &mut Budget<'_, F>,
    bound_models: &BoundModels,
    indexes: &PopulationIndexes,
    snapshot: usize,
    key: &PopulationKey,
) -> Result<bool> {
    budget.visit()?;
    if !bound_models
        .get(&snapshot)
        .is_some_and(|models| models.contains(&key.model))
    {
        budget.issue(
            Stage::Binding,
            Code::InvalidModelBinding,
            "required population lacks an exact input model binding",
        )?;
    }
    match indexes
        .get(&snapshot)
        .and_then(|populations| populations.get(key))
    {
        None => {
            budget.issue(
                Stage::Population,
                Code::IncompletePopulation,
                "required population is unavailable",
            )?;
            Ok(false)
        }
        Some(population) if !population.complete => {
            budget.issue(
                Stage::Population,
                Code::IncompletePopulation,
                "required population is declared incomplete",
            )?;
            Ok(false)
        }
        Some(population) => Ok(population.unambiguous),
    }
}

impl<'model, F: FnMut() -> bool> Validator<'_, 'model, F> {
    pub(super) fn related(
        &mut self,
        stage: Stage,
        code: Code,
        message: &'static str,
        locus: &DeclarationLocation,
    ) -> Result<()> {
        let mut diagnostic = self.budget.diagnostic(code, message);
        diagnostic.related.push(locus.clone());
        self.budget.observe(stage, *diagnostic)
    }

    pub(super) fn require_population(
        &mut self,
        snapshot: usize,
        key: &PopulationKey,
    ) -> Result<bool> {
        require_population(
            &mut self.budget,
            &self.bound_models,
            &self.indexes,
            snapshot,
            key,
        )
    }

    fn key_maximum(&self, model: &NativeModel, role: &ObjectRole) -> Option<u32> {
        let catalog = self.catalog(model.environment().owner())?;
        let field = *catalog
            .fields
            .get(&(&role.reference, &role.identity_field))?;
        let site = ScalarSite::Field {
            record: role.reference.clone(),
            field: role.identity_field.clone(),
        };
        match catalog.formal(field.value_type(), &site)? {
            NativeType::Scalar { role, .. } => match role.kind {
                ScalarKind::Text { max_scalars } => Some(max_scalars),
                ScalarKind::Integer { .. } | ScalarKind::Rational { .. } => None,
            },
            _ => None,
        }
    }

    fn inspect_key(
        &mut self,
        key: &str,
        model: &NativeModel,
        role: &ObjectRole,
        location: &DeclarationLocation,
    ) -> Result<()> {
        let Some(maximum) = self.key_maximum(model, role) else {
            return self.related(
                Stage::Binding,
                Code::InvalidModelBinding,
                "object role lacks its admitted text identity carrier",
                location,
            );
        };
        if !self.budget.text_bound(key, maximum)? {
            self.related(
                Stage::Value,
                Code::InvalidRuntimeInput,
                "object key exceeds its nominal Unicode scalar bound",
                location,
            )?;
        }
        Ok(())
    }

    fn object_value(
        &mut self,
        identity: &ObjectIdentity,
        model: &'model NativeModel,
        role: &'model ObjectRole,
        snapshot: Option<usize>,
        location: &DeclarationLocation,
    ) -> Result<()> {
        if identity.model != *model.environment().owner()
            || identity.record != role.record
            || identity.universe != role.universe
        {
            return self.related(
                Stage::Value,
                Code::InvalidRuntimeInput,
                "object/reference identity has the wrong model, type or universe",
                location,
            );
        }
        self.inspect_key(&identity.key, model, role, location)?;
        let Some(snapshot) = snapshot else {
            return Ok(());
        }; // Selection already records unavailable observation.
        let key = PopulationKey::of(identity);
        if self.require_population(snapshot, &key)? {
            let present = self
                .indexes
                .get(&snapshot)
                .and_then(|populations| populations.get(&key))
                .is_some_and(|population| population.objects.contains_key(&identity.key));
            if !present {
                self.related(
                    Stage::Value,
                    Code::DanglingReference,
                    "object target is absent from its complete population",
                    location,
                )?;
            }
        }
        Ok(())
    }

    pub(super) fn typed(
        &mut self,
        arena: &[ValueNode],
        value: ValueId,
        ty: &NativeType<'model>,
        snapshot: Option<usize>,
        location: &DeclarationLocation,
    ) -> Result<()> {
        self.budget.visit()?;
        let node = usize::try_from(value.index())
            .ok()
            .and_then(|index| arena.get(index));
        let Some(node) = node else {
            return self.related(
                Stage::Value,
                Code::InvalidRuntimeInput,
                "typed root is outside its artifact arena",
                location,
            );
        };
        match (ty, node) {
            (NativeType::Boolean, ValueNode::Boolean { .. }) => {}
            (
                NativeType::Scalar {
                    representation: ir::ValueType::Integer { value: integer },
                    ..
                },
                ValueNode::Integer { value },
            ) => {
                if *value < integer.minimum() || *value > integer.maximum() {
                    self.related(
                        Stage::Value,
                        Code::InvalidRuntimeInput,
                        "integer is outside its nominal bounds",
                        location,
                    )?;
                }
            }
            (NativeType::Scalar { role, .. }, ValueNode::Text { value }) => {
                let ScalarKind::Text { max_scalars } = role.kind else {
                    return self.related(
                        Stage::Value,
                        Code::InvalidRuntimeInput,
                        "text does not match its nominal scalar kind",
                        location,
                    );
                };
                if !self.budget.text_bound(value, max_scalars)? {
                    self.related(
                        Stage::Value,
                        Code::InvalidRuntimeInput,
                        "text exceeds its nominal Unicode scalar bound",
                        location,
                    )?;
                }
            }
            (
                NativeType::Enumeration { model, declaration },
                ValueNode::Enum {
                    declaration: actual,
                    variant,
                },
            ) => {
                if actual.model != *model.environment().owner()
                    || actual.name != *declaration.name()
                    || !self
                        .catalog(model.environment().owner())
                        .and_then(|catalog| catalog.variants.get(declaration.name()))
                        .is_some_and(|variants| variants.contains(variant))
                {
                    self.related(
                        Stage::Value,
                        Code::InvalidRuntimeInput,
                        "enum owner, declaration or variant does not match its native type",
                        location,
                    )?;
                }
            }
            (
                NativeType::Record { model, declaration },
                ValueNode::Record {
                    declaration: actual,
                    fields,
                },
            ) => {
                if actual.model != *model.environment().owner()
                    || actual.name != *declaration.name()
                {
                    self.related(
                        Stage::Value,
                        Code::InvalidRuntimeInput,
                        "structural record has the wrong native declaration",
                        location,
                    )?;
                } else {
                    self.fields(arena, fields, model, declaration, snapshot)?;
                }
            }
            (NativeType::Option(_), ValueNode::Absent) => {}
            (NativeType::Option(element), ValueNode::Present { value }) => {
                self.budget.visit()?; // One container edge, then one typed child use.
                self.typed(arena, *value, element, snapshot, location)?;
            }
            (NativeType::Sequence { element, maximum }, ValueNode::Sequence { values }) => {
                if u64::try_from(values.len()).map_or(true, |length| length > u64::from(*maximum)) {
                    self.related(
                        Stage::Value,
                        Code::InvalidRuntimeInput,
                        "sequence exceeds its declared maximum",
                        location,
                    )?;
                } else {
                    for (index, value) in values.iter().enumerate() {
                        self.budget.visit()?;
                        self.budget
                            .location
                            .path
                            .push(RuntimePathSegment::Index(index));
                        self.typed(arena, *value, element, snapshot, location)?;
                        self.budget.location.path.pop();
                    }
                }
            }
            (NativeType::Object { model, role }, ValueNode::Object { identity })
            | (NativeType::Reference { model, role }, ValueNode::Reference { identity }) => {
                self.object_value(identity, model, role, snapshot, location)?;
            }
            _ => self.related(
                Stage::Value,
                Code::InvalidRuntimeInput,
                "runtime value kind does not match its native type",
                location,
            )?,
        }
        Ok(())
    }

    fn fields(
        &mut self,
        arena: &[ValueNode],
        fields: &[FieldBinding],
        model: &'model NativeModel,
        declaration: &'model ir::RecordDeclaration,
        snapshot: Option<usize>,
    ) -> Result<()> {
        let mut seen = BTreeSet::new();
        for field in fields {
            self.budget.visit()?;
            self.budget
                .location
                .path
                .push(RuntimePathSegment::Field(field.name.clone()));
            let declared = self
                .catalog(model.environment().owner())
                .and_then(|catalog| catalog.fields.get(&(declaration.name(), &field.name)))
                .copied();
            let related = declared.map_or_else(
                || {
                    locus(
                        model,
                        DeclarationKey::Type(declaration.name().clone()),
                        declaration.source(),
                    )
                },
                |declared| {
                    locus(
                        model,
                        DeclarationKey::Field {
                            record: declaration.name().clone(),
                            field: declared.name().clone(),
                        },
                        declared.source(),
                    )
                },
            );
            if !seen.insert(&field.name) {
                self.related(
                    Stage::Value,
                    Code::InvalidRuntimeInput,
                    "duplicate runtime field binding",
                    &related,
                )?;
            }
            if let Some(declared) = declared {
                let site = ScalarSite::Field {
                    record: declaration.name().clone(),
                    field: field.name.clone(),
                };
                let ty = self
                    .catalog(model.environment().owner())
                    .and_then(|catalog| catalog.formal(declared.value_type(), &site));
                if let Some(ty) = ty {
                    self.typed(arena, field.value, &ty, snapshot, &related)?;
                } else {
                    self.related(
                        Stage::Binding,
                        Code::InvalidModelBinding,
                        "field lacks an admitted native type",
                        &related,
                    )?;
                }
            } else {
                self.related(
                    Stage::Value,
                    Code::InvalidRuntimeInput,
                    "runtime field is not in its native record",
                    &related,
                )?;
            }
            self.budget.location.path.pop();
        }
        for field in declaration.fields() {
            self.budget.visit()?;
            if !seen.contains(field.name()) {
                self.budget
                    .location
                    .path
                    .push(RuntimePathSegment::Field(field.name().clone()));
                let related = locus(
                    model,
                    DeclarationKey::Field {
                        record: declaration.name().clone(),
                        field: field.name().clone(),
                    },
                    field.source(),
                );
                self.related(
                    Stage::Value,
                    Code::InvalidRuntimeInput,
                    "required runtime field is missing",
                    &related,
                )?;
                self.budget.location.path.pop();
            }
        }
        Ok(())
    }

    pub(super) fn inspect_values(&mut self, clause_index: usize) -> Result<()> {
        let checked = self.checked;
        let clause = &checked.clauses()[clause_index];
        for universe in &clause.runtime_requirements().universes {
            for observation in &universe.observations {
                self.budget.poll()?;
                if let Some(snapshot) = self.selected.snapshot(*observation) {
                    self.at_snapshot(snapshot, *observation);
                    let key = PopulationKey {
                        model: universe.model.environment().owner().clone(),
                        record: universe.object.record.clone(),
                        universe: universe.object.universe.clone(),
                    };
                    self.budget.location.path = key.path();
                    self.require_population(snapshot, &key)?;
                }
            }
        }
        for (observation, index) in self.selected.snapshots() {
            self.at_snapshot(index, observation);
            let input = self.input;
            let snapshot = &input.snapshots[index];
            for (position, population) in snapshot.draft().populations.iter().enumerate() {
                self.budget.visit()?;
                let model = self.catalog(&population.model).map(|catalog| catalog.model);
                let role = self
                    .catalog(&population.model)
                    .and_then(|catalog| catalog.objects.get(&population.record))
                    .copied();
                let record = self
                    .catalog(&population.model)
                    .and_then(|catalog| catalog.records.get(&population.record))
                    .copied();
                let (Some(model), Some(role), Some(record)) = (model, role, record) else {
                    continue;
                };
                let key = snapshot_key(snapshot, position);
                for object in &population.objects {
                    self.budget.visit()?;
                    self.budget.location.path = key.path();
                    self.budget
                        .location
                        .path
                        .push(RuntimePathSegment::Object(object.key.clone()));
                    let related = locus(
                        model,
                        DeclarationKey::Type(record.name().clone()),
                        record.source(),
                    );
                    self.inspect_key(&object.key, model, role, &related)?;
                    self.fields(
                        &snapshot.draft().arena,
                        &object.fields,
                        model,
                        record,
                        Some(index),
                    )?;
                }
            }
            let mut seen = BTreeSet::new();
            for root in &snapshot.draft().values {
                self.budget.visit()?;
                self.budget.location.path =
                    vec![RuntimePathSegment::State(root.declaration.clone())];
                if !seen.insert(&root.declaration) {
                    self.budget.issue(
                        Stage::Value,
                        Code::InvalidRuntimeInput,
                        "duplicate State value binding",
                    )?;
                }
                if !self
                    .bound_models
                    .get(&index)
                    .is_some_and(|models| models.contains(&root.declaration.model))
                {
                    self.budget.issue(
                        Stage::Binding,
                        Code::InvalidModelBinding,
                        "State root lacks an exact input model binding",
                    )?;
                }
                let model = self
                    .catalog(&root.declaration.model)
                    .map(|catalog| catalog.model);
                let declaration = self
                    .catalog(&root.declaration.model)
                    .and_then(|catalog| catalog.values.get(&root.declaration.name))
                    .copied();
                let (Some(model), Some(declaration)) = (model, declaration) else {
                    self.budget.issue(
                        Stage::Value,
                        Code::InvalidRuntimeInput,
                        "State root declaration is unknown",
                    )?;
                    continue;
                };
                let location = locus(
                    model,
                    DeclarationKey::Value(declaration.name().clone()),
                    declaration.source(),
                );
                if declaration.kind() != ir::ValueDeclarationKind::State {
                    self.related(
                        Stage::Value,
                        Code::InvalidRuntimeInput,
                        "snapshot root must name a State declaration",
                        &location,
                    )?;
                    continue;
                }
                self.value_declaration(
                    &snapshot.draft().arena,
                    root.value,
                    model,
                    declaration,
                    Some(index),
                    &location,
                )?;
            }
        }
        self.required_state_roots(clause_index)?;
        if let ObservationSelection::Current { self_object, .. } = &self.selection.observation {
            if let Some(snapshot) = self.selected.current {
                self.at_snapshot(snapshot, ir::StateObservation::Current);
                self.context_object(clause_index, self_object, Some(snapshot))?;
            }
        }
        Ok(())
    }

    fn value_declaration(
        &mut self,
        arena: &[ValueNode],
        value: ValueId,
        model: &'model NativeModel,
        declaration: &'model ir::ValueDeclaration,
        snapshot: Option<usize>,
        location: &DeclarationLocation,
    ) -> Result<()> {
        let site = ScalarSite::Value {
            name: declaration.name().clone(),
        };
        let ty = self
            .catalog(model.environment().owner())
            .and_then(|catalog| catalog.formal(declaration.value_type(), &site));
        if let Some(ty) = ty {
            self.typed(arena, value, &ty, snapshot, location)
        } else {
            self.related(
                Stage::Binding,
                Code::InvalidModelBinding,
                "value declaration lacks an admitted native type",
                location,
            )
        }
    }

    fn required_state_roots(&mut self, clause_index: usize) -> Result<()> {
        let checked = self.checked;
        let clause = &checked.clauses()[clause_index];
        for occurrence in checked.linked().clauses()[clause_index].occurrences() {
            self.budget.poll()?;
            let (Some(expression), ResolutionTarget::Formal(location)) =
                (occurrence.expression, &occurrence.target)
            else {
                continue;
            };
            let DeclarationKey::Value(name) = &location.identity.key else {
                continue;
            };
            let declaration = self
                .catalog(&location.identity.owner)
                .and_then(|catalog| catalog.values.get(name))
                .copied();
            if !declaration
                .is_some_and(|declaration| declaration.kind() == ir::ValueDeclarationKind::State)
            {
                continue;
            }
            let Some(Observation::Snapshot(observation)) = clause.observation(expression) else {
                continue;
            };
            self.budget.visit()?;
            self.required_states.insert((
                observation,
                QualifiedName {
                    model: location.identity.owner.clone(),
                    name: name.clone(),
                },
            ));
        }
        for (observation, name) in &self.required_states {
            self.budget.poll()?;
            if let Some(index) = self.selected.snapshot(*observation) {
                self.budget.location.artifact =
                    super::RuntimeReference::Snapshot(self.input.snapshots[index].reference());
                self.budget.location.observation = Some(*observation);
                self.budget.location.path = vec![RuntimePathSegment::State(name.clone())];
                self.budget.visit()?;
                if !self
                    .states
                    .get(&index)
                    .is_some_and(|states| states.contains_key(name))
                {
                    self.budget.issue(
                        Stage::Observation,
                        Code::UnavailableObservation,
                        "required State root is unavailable",
                    )?;
                }
            }
        }
        Ok(())
    }

    fn context_object(
        &mut self,
        clause: usize,
        identity: &ObjectIdentity,
        snapshot: Option<usize>,
    ) -> Result<()> {
        self.budget.visit()?;
        let requirements = self.checked.clauses()[clause].runtime_requirements();
        let DeclarationKey::Type(record) = &requirements.context.identity.key else {
            return self.budget.issue(
                Stage::Binding,
                Code::InvalidModelBinding,
                "checked context is not an object declaration",
            );
        };
        let role = self
            .catalog(requirements.model.environment().owner())
            .and_then(|catalog| catalog.objects.get(record))
            .copied();
        let Some(role) = role else {
            return self.budget.issue(
                Stage::Binding,
                Code::InvalidModelBinding,
                "checked context lacks an admitted object role",
            );
        };
        self.budget.location.path = PopulationKey::of(identity).path();
        self.budget
            .location
            .path
            .push(RuntimePathSegment::Object(identity.key.clone()));
        self.object_value(
            identity,
            requirements.model,
            role,
            snapshot,
            &requirements.context,
        )
    }

    pub(super) fn inspect_invocation(&mut self, clause_index: usize) -> Result<()> {
        let Some(index) = self.selected.invocation else {
            return Ok(());
        };
        let input = self.input;
        let invocation = input.invocations[index].draft();
        let requirements = self.checked.clauses()[clause_index].runtime_requirements();
        let model = requirements.model;
        self.at_invocation(index);
        let bound = self.models(&invocation.models)?;
        if !bound.contains(model.environment().owner()) {
            self.budget.issue(
                Stage::Binding,
                Code::InvalidModelBinding,
                "invocation lacks its exact native model binding",
            )?;
        }
        let Some(operation) = requirements.operation else {
            return self.budget.issue(
                Stage::Binding,
                Code::InvalidModelBinding,
                "checked clause has no selected native operation",
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
        if invocation.context.model != *model.environment().owner()
            || invocation.context.name != operation.context
            || invocation.operation != operation.name
            || invocation.anchor != operation.anchor
        {
            self.related(
                Stage::Observation,
                Code::WrongSnapshot,
                "invocation context, operation or anchor differs from the checked operation",
                &location,
            )?;
        }
        self.budget.location.observation = Some(ir::StateObservation::Pre);
        self.context_object(clause_index, &invocation.self_object, self.selected.pre)?;
        if self.checked.linked().unit().clauses()[clause_index].kind == ClauseKind::Postcondition {
            self.budget.location.observation = Some(ir::StateObservation::Post);
            self.context_object(clause_index, &invocation.self_object, self.selected.post)?;
        }
        self.budget.location.observation = Some(ir::StateObservation::Pre);
        let mut seen = BTreeSet::new();
        let mut parameter_names = BTreeSet::new();
        for name in &operation.parameters {
            self.budget.visit()?;
            parameter_names.insert(name);
        }
        for parameter in &invocation.parameters {
            self.budget.visit()?;
            self.budget.location.path =
                vec![RuntimePathSegment::Parameter(parameter.declaration.clone())];
            if !seen.insert(&parameter.declaration) {
                self.related(
                    Stage::Value,
                    Code::InvalidRuntimeInput,
                    "duplicate invocation parameter",
                    &location,
                )?;
            }
            if parameter.declaration.model != *model.environment().owner()
                || !parameter_names.contains(&parameter.declaration.name)
            {
                self.related(
                    Stage::Value,
                    Code::InvalidRuntimeInput,
                    "invocation parameter is not declared by the selected operation",
                    &location,
                )?;
                continue;
            }
            let declaration = self
                .catalog(model.environment().owner())
                .and_then(|catalog| catalog.values.get(&parameter.declaration.name))
                .copied();
            if let Some(declaration) = declaration {
                let location = locus(
                    model,
                    DeclarationKey::Value(declaration.name().clone()),
                    declaration.source(),
                );
                self.value_declaration(
                    &invocation.arena,
                    parameter.value,
                    model,
                    declaration,
                    self.selected.pre,
                    &location,
                )?;
            }
        }
        for name in &operation.parameters {
            self.budget.visit()?;
            let qualified = QualifiedName {
                model: model.environment().owner().clone(),
                name: name.clone(),
            };
            if !seen.contains(&qualified) {
                self.budget.location.path = vec![RuntimePathSegment::Parameter(qualified)];
                self.related(
                    Stage::Value,
                    Code::InvalidRuntimeInput,
                    "required invocation parameter is missing",
                    &location,
                )?;
            }
        }
        self.budget.location.observation = Some(ir::StateObservation::Post);
        self.budget.location.path = vec![RuntimePathSegment::Result];
        match (&operation.result, invocation.result) {
            (None, None) => {}
            (Some(name), Some(value)) => {
                self.budget.visit()?;
                let declaration = self
                    .catalog(model.environment().owner())
                    .and_then(|catalog| catalog.values.get(name))
                    .copied();
                if let Some(declaration) = declaration {
                    let location = locus(
                        model,
                        DeclarationKey::Value(declaration.name().clone()),
                        declaration.source(),
                    );
                    self.value_declaration(
                        &invocation.arena,
                        value,
                        model,
                        declaration,
                        self.selected.post,
                        &location,
                    )?;
                }
            }
            _ => self.related(
                Stage::Value,
                Code::InvalidRuntimeInput,
                "invocation result presence does not match its operation",
                &location,
            )?,
        }
        Ok(())
    }
}
