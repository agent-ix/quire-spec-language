// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-015: explicit native inventory and role resolution within the shared linker.

use std::collections::BTreeMap;

use quire_contract_ir::{
    DeclarationEnvironment, ValueDeclaration, ValueDeclarationKind, ValueType,
};

use super::{
    failure, location, DeclarationKey, LinkLimits, LinkedModel, LinkingError, ResolutionTarget,
    Resolver, Shape,
};
use crate::native_model::{NativeModel, NativeModelProfile, OperationRole};
use crate::syntax::{Clause, ExprId};
use crate::{Code, ParsedUnit, Span, Spanned};

type Result<T> = std::result::Result<T, Box<LinkingError>>;

/// FR-041: only selected artifacts cross the historical semantic boundary.
pub(super) fn check_selected_profiles(unit: &ParsedUnit, models: &[LinkedModel<'_>]) -> Result<()> {
    // select_models preserves this shape and order. Defensively refuse a future
    // constructor that breaks it, rather than letting zip silently omit imports.
    if unit.imports().len() != models.len() {
        return Err(failure(
            unit,
            Code::InvalidModelBinding,
            Span { start: 0, end: 0 },
            "selected model inventory does not match authored import count",
        ));
    }
    for (import, selected) in unit.imports().iter().zip(models) {
        // Defensive: native::catalog always supplies admitted native models.
        let model = selected.native_model().ok_or_else(|| {
            failure(
                unit,
                Code::InvalidModelBinding,
                import.span,
                "historical native import has no admitted native model",
            )
        })?;
        match model.profile() {
            NativeModelProfile::V1 => {}
            NativeModelProfile::V2 => {
                return Err(failure(
                    unit,
                    Code::UnsupportedConstruct,
                    import.span,
                    format!(
                        "historical native consumers do not admit {}",
                        model.profile().as_str()
                    ),
                ))
            }
        }
    }
    Ok(())
}

pub(super) fn catalog<'a>(
    unit: &ParsedUnit,
    models: &'a [NativeModel],
    limits: LinkLimits,
) -> Result<Vec<LinkedModel<'a>>> {
    let mut remaining = limits.total_model_bytes;
    let mut sources = BTreeMap::new();
    let mut owners = BTreeMap::new();
    let mut catalog = Vec::with_capacity(models.len());
    for model in models {
        let length = model.artifact_bytes().len();
        if length > limits.model_bytes {
            return Err(failure(
                unit,
                Code::ResourceExhausted,
                Span { start: 0, end: 0 },
                "native model artifact byte limit exceeded",
            ));
        }
        remaining = remaining.checked_sub(length).ok_or_else(|| {
            failure(
                unit,
                Code::ResourceExhausted,
                Span { start: 0, end: 0 },
                "native model inventory byte limit exceeded",
            )
        })?;
        let formal = model.source().identity();
        let native = model.source().source();
        if let Some(previous) = sources.insert(formal, model) {
            let previous_source = previous.source().source();
            if previous_source.identity() != native.identity()
                || previous_source.digest() != native.digest()
            {
                return Err(conflict(
                    model,
                    previous,
                    "one formal source identity has conflicting native source bindings",
                ));
            }
        }
        if let Some(previous) = owners.insert(model.environment().owner(), model) {
            if previous.digest() != model.digest() {
                return Err(conflict(
                    model,
                    previous,
                    "one formal model owner has conflicting native artifacts",
                ));
            }
        }
        catalog.push(LinkedModel {
            environment: model.environment(),
            digest: model.digest(),
            native: Some(model),
        });
    }
    Ok(catalog)
}

fn conflict(model: &NativeModel, previous: &NativeModel, message: &str) -> Box<LinkingError> {
    let mut error = Box::new(LinkingError {
        diagnostic: crate::diagnostic::error(
            model.source().source(),
            Code::InvalidModelBinding,
            crate::Phase::Link,
            0,
            0,
            message,
        ),
        related: Vec::new(),
        upstream: None,
    });
    error.related = [previous, model]
        .into_iter()
        .flat_map(|model| {
            let types = model.environment().types().iter().map(|declaration| {
                location(
                    model.environment(),
                    DeclarationKey::Type(declaration.name().clone()),
                    declaration.source(),
                )
            });
            let values = model.environment().values().iter().map(|declaration| {
                location(
                    model.environment(),
                    DeclarationKey::Value(declaration.name().clone()),
                    declaration.source(),
                )
            });
            types.chain(values)
        })
        .collect();
    error.related.sort();
    error
}

pub(super) fn operation<'a>(
    unit: &ParsedUnit,
    model: &'a NativeModel,
    clause: &Clause,
) -> Result<Option<&'a OperationRole>> {
    let Some(name) = &clause.operation else {
        return Ok(None);
    };
    model
        .roles()
        .operations
        .iter()
        .find(|operation| {
            operation.context.as_str() == clause.context.value
                && operation.name.as_str() == name.value
        })
        .map(Some)
        .ok_or_else(|| {
            failure(
                unit,
                Code::MissingDeclaration,
                name.span,
                "native operation is absent from the selected object context",
            )
        })
}

impl<'a> Resolver<'_, 'a> {
    fn native_for(&self, environment: &DeclarationEnvironment) -> Option<&'a NativeModel> {
        self.models
            .iter()
            .find(|model| std::ptr::eq(model.environment, environment))
            .and_then(|model| model.native)
    }

    pub(super) fn shape(
        &self,
        environment: &'a DeclarationEnvironment,
        ty: &'a ValueType,
    ) -> Shape<'a> {
        if let (Some(model), ValueType::Record { name }) = (self.native_for(environment), ty) {
            for object in &model.roles().objects {
                if &object.record == name {
                    return Shape::Object(environment, &object.record);
                }
                if &object.reference == name {
                    return Shape::Reference(environment, object);
                }
            }
        }
        Shape::Formal(environment, ty)
    }

    pub(super) fn context_value(&mut self, id: ExprId, span: Span) -> Result<Shape<'a>> {
        let model = self.models[self.current];
        if model.native.is_none() {
            return self.value(id, "self", span);
        }
        self.occurrence(
            id,
            span,
            ResolutionTarget::Formal(location(
                model.environment,
                DeclarationKey::Type(self.context.name().clone()),
                self.context.source(),
            )),
        );
        Ok(Shape::Object(model.environment, self.context.name()))
    }

    pub(super) fn check_input_scope(&self, value: &ValueDeclaration, span: Span) -> Result<()> {
        let Some(model) = self.models[self.current].native else {
            return Ok(());
        };
        if value.kind() != ValueDeclarationKind::Input
            || self
                .operation
                .is_some_and(|operation| operation.parameters.contains(value.name()))
        {
            return Ok(());
        }
        if model
            .roles()
            .operations
            .iter()
            .any(|operation| operation.result.as_ref() == Some(value.name()))
        {
            return Err(failure(
                self.unit,
                Code::WrongSnapshot,
                span,
                "operation results are accessed through the result keyword",
            ));
        }
        Err(failure(
            self.unit,
            Code::MissingDeclaration,
            span,
            "invocation input is not a parameter of this clause's operation",
        ))
    }

    pub(super) fn result_value(&mut self, id: ExprId, span: Span) -> Result<Shape<'a>> {
        if self.models[self.current].native.is_none() {
            return self.value(id, "result", span);
        }
        let name = self
            .operation
            .and_then(|operation| operation.result.as_ref())
            .ok_or_else(|| {
                failure(
                    self.unit,
                    Code::WrongSnapshot,
                    span,
                    "result requires an explicit operation result binding",
                )
            })?;
        let value = self.lookup_value(name.as_str(), span)?;
        // Resolving a declared result does not make it available in pre. The
        // checker owns observation availability after this exact name binding.
        Ok(self.bind_value(id, span, value))
    }

    pub(super) fn dereference(
        &mut self,
        id: ExprId,
        argument: ExprId,
        span: Span,
        depth: usize,
    ) -> Result<Shape<'a>> {
        if self.models[self.current].native.is_none() {
            // A missing binding, the same condition already classified
            // InvalidModelBinding above and at this function's own
            // `InvalidModelBinding` return below; not a shape mismatch.
            return Err(failure(
                self.unit,
                Code::InvalidModelBinding,
                span,
                "native references need a concrete formal mapping",
            ));
        }
        let Shape::Reference(environment, role) = self.visit(argument, depth)? else {
            return Err(failure(
                self.unit,
                Code::IllTyped,
                span,
                "dereference requires an explicitly mapped native reference",
            ));
        };
        let declaration = environment
            .types()
            .iter()
            .find(|declaration| declaration.name() == &role.record)
            .ok_or_else(|| {
                failure(
                    self.unit,
                    Code::InvalidModelBinding,
                    span,
                    "reference target declaration is absent",
                )
            })?;
        self.occurrence(
            id,
            span,
            ResolutionTarget::Formal(location(
                environment,
                DeclarationKey::Type(role.record.clone()),
                declaration.source(),
            )),
        );
        Ok(Shape::Object(environment, &role.record))
    }

    pub(super) fn reaches(
        &mut self,
        id: ExprId,
        start: ExprId,
        target: ExprId,
        field: &Spanned<String>,
        span: Span,
        depth: usize,
    ) -> Result<Shape<'a>> {
        if self.models[self.current].native.is_none() {
            // A missing binding, not a shape mismatch; see `dereference` above.
            return Err(failure(
                self.unit,
                Code::InvalidModelBinding,
                span,
                "native reachability needs a concrete reference/population mapping",
            ));
        }
        let (environment, object) = match self.visit(start, depth)? {
            Shape::Object(environment, name) => (environment, name),
            Shape::Reference(environment, role) => (environment, &role.record),
            Shape::Formal(..) | Shape::Boolean | Shape::Integer | Shape::Text | Shape::Unknown => {
                return Err(failure(
                    self.unit,
                    Code::IllTyped,
                    span,
                    "reachability requires an explicit object or reference mapping",
                ));
            }
        };
        self.visit(target, depth)?;
        let edge = self.field(id, field, Shape::Object(environment, object))?;
        let mapped = match edge {
            Shape::Formal(environment, ValueType::Option { value }) => {
                matches!(self.shape(environment, value), Shape::Reference(_, role) if &role.record == object)
            }
            Shape::Formal(..)
            | Shape::Object(..)
            | Shape::Reference(..)
            | Shape::Boolean
            | Shape::Integer
            | Shape::Text
            | Shape::Unknown => false,
        };
        if !mapped {
            return Err(failure(
                self.unit,
                Code::IllTyped,
                field.span,
                "reachability edge requires an optional reference to its object type",
            ));
        }
        // Endpoint type/universe/observation compatibility remains a checker
        // judgment; this stage resolves the actual target and edge names.
        Ok(Shape::Boolean)
    }
}
