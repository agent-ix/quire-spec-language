// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-042: authored family bodies and static requirements for later runtime input.
mod compensations;
use super::context::Declaration;
use super::{
    layout::DeclLayout,
    metadata::{copy_reference, Metadata},
    types::{index, text, ValueBuilder},
};
use crate::checking::{composed::DeclarationTypes, NativeType};
use crate::linking::composed::{
    definition_source::RegisteredDefinition as R,
    models::ModelTarget,
    scopes::{self, Anchor, BinderKind, BinderType, DeclarationScope},
};
use crate::protocol_artifact::{wire as w, work::Work, Dimension, Error, Invalid, Unsupported};
use crate::syntax::{composed as c, ClauseKind};
use crate::{Span, Spanned};

pub(super) fn binders(
    typed: &DeclarationTypes<'_>,
    scope: &DeclarationScope,
    layout: &DeclLayout,
    builder: &mut ValueBuilder<'_>,
    work: &mut Work,
) -> Result<Vec<w::Binder>, Error> {
    let mut result = Vec::new();
    for &original in &layout.binders {
        work.visit()?;
        let bound = &scope.binders[original];
        let value_type = builder.ty(binder_type(typed, original)?, work)?;
        let name = match (bound.name.as_deref(), bound.kind) {
            (Some(name), _) => name,
            (None, BinderKind::SelfValue) => "self",
            (None, BinderKind::ResultValue) => "result",
            (None, _) => return Err(Error::Invalid(Invalid::Binding)),
        };
        work.charge(Dimension::Entries, 1)?;
        result.push(w::Binder {
            name: text(name, work)?,
            kind: binder_kind(bound.kind),
            value_type,
            scope: layout.binder_scope(original)?,
            anchor: layout.anchor(bound.anchor)?,
            initializer: w::Nullable(None),
            locus: layout.locus(bound.span)?,
        });
    }
    Ok(result)
}
pub(super) fn initializers(
    syntax: &c::Declaration,
    scope: &DeclarationScope,
    layout: &DeclLayout,
    binders: &mut [w::Binder],
    work: &mut Work,
) -> Result<(), Error> {
    for (local, &original) in layout.binders.iter().enumerate() {
        work.visit()?;
        let source = scope
            .binders
            .get(original)
            .ok_or(Error::Invalid(Invalid::Binding))?;
        if let BinderType::Initializer(value) = source.ty {
            binders
                .get_mut(local)
                .ok_or(Error::Invalid(Invalid::Binding))?
                .initializer = w::Nullable(Some(layout.value(value)?));
        }
    }
    let captures = match &syntax.kind {
        c::DeclarationKind::Temporal { captures, .. } => captures.as_slice(),
        c::DeclarationKind::Protocol(p) => p.captures.as_slice(),
        c::DeclarationKind::Predicate { .. } | c::DeclarationKind::State { .. } => &[],
    };
    capture_initializers(captures, scope, layout, binders, work)?;
    if let c::DeclarationKind::Protocol(protocol) = &syntax.kind {
        for requirement in &protocol.requirements {
            work.visit()?;
            if let c::ProtocolRequirement::Compensation(value) = requirement {
                capture_initializers(&value.registration_captures, scope, layout, binders, work)?;
                capture_initializers(&value.activation_captures, scope, layout, binders, work)?;
            }
        }
    }
    Ok(())
}
fn capture_initializers(
    captures: &[c::Capture],
    scope: &DeclarationScope,
    layout: &DeclLayout,
    binders: &mut [w::Binder],
    work: &mut Work,
) -> Result<(), Error> {
    for capture in captures {
        work.visit()?;
        work.charge(Dimension::References, layout.binders.len())?;
        let binder = layout.binder_at(scope, capture.parameter.name.span, BinderKind::Capture)?;
        binders
            .get_mut(binder.index as usize)
            .ok_or(Error::Invalid(Invalid::Binding))?
            .initializer = w::Nullable(Some(layout.value(capture.value)?));
    }
    Ok(())
}
fn binder_kind(kind: BinderKind) -> w::BinderKind {
    match kind {
        BinderKind::Parameter => w::BinderKind::Parameter,
        BinderKind::Input => w::BinderKind::Input,
        BinderKind::Trigger => w::BinderKind::Trigger,
        BinderKind::Capture => w::BinderKind::Capture,
        BinderKind::Let => w::BinderKind::Let,
        BinderKind::Query => w::BinderKind::Query,
        BinderKind::SelfValue => w::BinderKind::SelfValue,
        BinderKind::ResultValue => w::BinderKind::Result,
        BinderKind::InvocationParameter => w::BinderKind::InvocationParameter,
        BinderKind::EventRecord => w::BinderKind::Event,
        BinderKind::Finish => w::BinderKind::Finish,
        BinderKind::Fifo => w::BinderKind::Fifo,
        BinderKind::ForwardEffect => w::BinderKind::ForwardEffect,
        BinderKind::CompensationTrigger => w::BinderKind::CompensationTrigger,
        BinderKind::EarlierAttempt => w::BinderKind::EarlierAttempt,
        BinderKind::LaterAttempt => w::BinderKind::LaterAttempt,
        BinderKind::Recovery => w::BinderKind::Recovery,
    }
}
fn anchor_kind(anchor: Anchor) -> w::AnchorKind {
    match anchor {
        Anchor::Predicate => w::AnchorKind::Predicate,
        Anchor::Current => w::AnchorKind::Current,
        Anchor::InvocationInput => w::AnchorKind::InvocationInput,
        Anchor::InvocationPre => w::AnchorKind::InvocationPre,
        Anchor::InvocationPost => w::AnchorKind::InvocationPost,
        Anchor::Activation => w::AnchorKind::Activation,
        Anchor::TemporalInstant => w::AnchorKind::TemporalInstant,
        Anchor::ProtocolInstant => w::AnchorKind::ProtocolInstant,
        Anchor::Fifo(_) => w::AnchorKind::Fifo,
        Anchor::Registration(_) => w::AnchorKind::Registration,
        Anchor::CompensationActivation(_) => w::AnchorKind::CompensationActivation,
        Anchor::Retry(_) => w::AnchorKind::Retry,
        Anchor::Recovery(_) => w::AnchorKind::Recovery,
        Anchor::Control(_) => w::AnchorKind::Control,
        Anchor::Finish => w::AnchorKind::Finish,
    }
}
pub(super) struct Requirement<'a> {
    pub name: &'a str,
    pub kind: w::BindingKind,
    pub selected: R,
    pub value_type: Option<u32>,
    pub model: Option<w::ExportRef>,
    pub subject: w::Subject,
    pub anchor: Anchor,
    pub requires: Vec<u32>,
    pub span: Span,
}
pub(super) struct Runtime<'a, 'm> {
    layout: &'a DeclLayout,
    meta: &'a Metadata<'m>,
    bindings: Vec<w::BindingRequirement>,
    anchors: Vec<w::Anchor>,
}
impl Runtime<'_, '_> {
    pub(super) fn metadata(&self) -> &Metadata<'_> {
        self.meta
    }
    pub(super) fn add(
        &mut self,
        requirement: Requirement<'_>,
        work: &mut Work,
    ) -> Result<u32, Error> {
        let Requirement {
            name,
            kind,
            selected,
            value_type,
            model,
            subject,
            anchor,
            requires,
            span,
        } = requirement;
        let contract = self.meta.definition_dependency(selected, work)?;
        let authority = copy_reference(self.meta.dependencies[contract as usize].artifact, work)?;
        let at = index(self.bindings.len())?;
        work.charge(Dimension::Entries, 1)?;
        self.bindings.push(w::BindingRequirement {
            name: text(name, work)?,
            kind,
            value_type: w::Nullable(value_type),
            authority,
            contract,
            model: w::Nullable(model),
            subject,
            anchor: self.layout.anchor(anchor)?,
            scope: self.layout.handle(0),
            relation: w::Nullable(None),
            requires,
            locus: self.layout.locus(span)?,
        });
        Ok(at)
    }

    pub(super) fn set_relation(&mut self, binding: u32, relation: u32) -> Result<(), Error> {
        let binding = self
            .bindings
            .get_mut(binding as usize)
            .ok_or(Error::Invalid(Invalid::Binding))?;
        if binding.relation.0.replace(relation).is_some() {
            return Err(Error::Invalid(Invalid::Duplicate));
        }
        Ok(())
    }
    fn declaration_subject(&self) -> w::Subject {
        w::Subject::Declaration {
            declaration: self.layout.declaration,
        }
    }
    pub(super) fn bind_anchor(&mut self, anchor: Anchor, binding: u32) -> Result<(), Error> {
        let index = self.layout.anchor(anchor)?.index as usize;
        self.anchors
            .get_mut(index)
            .ok_or(Error::Invalid(Invalid::Binding))?
            .binding = w::Nullable(Some(binding));
        Ok(())
    }
    fn anchor_binding(&self, anchor: Anchor, work: &mut Work) -> Result<u32, Error> {
        work.visit()?;
        self.anchors
            .get(self.layout.anchor(anchor)?.index as usize)
            .and_then(|value| value.binding.0)
            .ok_or(Error::Invalid(Invalid::Binding))
    }
    fn require(&mut self, binding: u32, prerequisite: u32, work: &mut Work) -> Result<(), Error> {
        work.visit()?;
        self.bindings
            .get(prerequisite as usize)
            .ok_or(Error::Invalid(Invalid::Binding))?;
        let binding = self
            .bindings
            .get_mut(binding as usize)
            .ok_or(Error::Invalid(Invalid::Binding))?;
        for existing in &binding.requires {
            work.visit()?;
            if *existing == prerequisite {
                return Ok(());
            }
        }
        work.charge(Dimension::Entries, 1)?;
        binding.requires.push(prerequisite);
        binding.requires.sort_unstable();
        Ok(())
    }
}
pub(super) fn nominal(
    ty: &NativeType<'_>,
    builder: &ValueBuilder<'_>,
    work: &mut Work,
) -> Result<w::ExportRef, Error> {
    match ty {
        NativeType::Scalar { model, role, .. } => {
            builder.export(model, w::ExportKind::Scalar, role.name.as_str(), None, work)
        }
        NativeType::Enumeration { model, declaration } => builder.export(
            model,
            w::ExportKind::Enum,
            declaration.name().as_str(),
            None,
            work,
        ),
        NativeType::Record { model, declaration } => builder.export(
            model,
            w::ExportKind::Record,
            declaration.name().as_str(),
            None,
            work,
        ),
        NativeType::Object { model, role } => builder.export(
            model,
            w::ExportKind::Object,
            role.record.as_str(),
            None,
            work,
        ),
        NativeType::Reference { model, role } => builder.export(
            model,
            w::ExportKind::Reference,
            role.reference.as_str(),
            None,
            work,
        ),
        NativeType::Boolean | NativeType::Option(_) | NativeType::Sequence { .. } => {
            Err(Error::Unsupported(Unsupported::Export))
        }
    }
}
pub(super) fn binder_type<'a, 'm>(
    typed: &'a DeclarationTypes<'m>,
    original: usize,
) -> Result<&'a NativeType<'m>, Error> {
    // The private type report retains one row per original scope binder ordinal.
    typed
        .binders()
        .get(original)
        .filter(|b| b.binder == original)
        .and_then(|b| b.ty.as_ref())
        .ok_or(Error::Invalid(Invalid::Type))
}
pub(super) fn parameter_handle(
    layout: &DeclLayout,
    scope: &DeclarationScope,
    p: &c::Parameter,
    kind: BinderKind,
) -> Result<w::Handle, Error> {
    layout.binder_at(scope, p.name.span, kind)
}
fn activation(
    value: &c::Activation,
    layout: &DeclLayout,
    scope: &DeclarationScope,
) -> Result<w::Activation, Error> {
    Ok(match value {
        c::Activation::Origin { .. } => w::Activation::Origin {
            anchor: layout.anchor(Anchor::Activation)?,
        },
        c::Activation::Each { trigger, guard, .. } => w::Activation::Each {
            trigger: parameter_handle(layout, scope, trigger, BinderKind::Trigger)?,
            guard: w::Nullable(guard.map(|id| layout.value(id)).transpose()?),
            anchor: layout.anchor(Anchor::Activation)?,
        },
    })
}
fn captures(
    values: &[c::Capture],
    layout: &DeclLayout,
    scope: &DeclarationScope,
    work: &mut Work,
) -> Result<Vec<w::Handle>, Error> {
    let mut result = Vec::new();
    for capture in values {
        work.visit()?;
        work.charge(Dimension::Entries, 1)?;
        result.push(parameter_handle(
            layout,
            scope,
            &capture.parameter,
            BinderKind::Capture,
        )?);
    }
    Ok(result)
}
pub(super) fn model_type<'s, 'm>(
    context: &'s Declaration<'_, 'm>,
    span: Span,
    work: &mut Work,
) -> Result<&'s NativeType<'m>, Error> {
    let report = context.exports;
    for occurrence in &report.occurrences {
        work.visit()?;
        if occurrence.span == span {
            if let ModelTarget::Type(ty) = &occurrence.target {
                return Ok(ty.native());
            }
        }
    }
    Err(Error::Invalid(Invalid::Model))
}
pub(super) fn body(
    context: &Declaration<'_, '_>,
    meta: &Metadata<'_>,
    binders: &[w::Binder],
    builder: &mut ValueBuilder<'_>,
    work: &mut Work,
) -> Result<(Vec<w::Anchor>, Vec<w::BindingRequirement>, w::Body), Error> {
    let Declaration {
        typed,
        scope,
        syntax,
        layout,
        profiles,
        ..
    } = context;
    let mut runtime = Runtime {
        layout,
        meta,
        bindings: Vec::new(),
        anchors: Vec::new(),
    };
    for &(a, span) in &layout.anchors {
        work.charge(Dimension::Entries, 1)?;
        let owner = match a {
            Anchor::Control(c) => Some(layout.control(c)?),
            Anchor::Fifo(i) => Some(layout.handle(index(i)?)),
            Anchor::Registration(i)
            | Anchor::CompensationActivation(i)
            | Anchor::Retry(i)
            | Anchor::Recovery(i) => Some(layout.compensation(i)?),
            Anchor::Predicate
            | Anchor::Current
            | Anchor::InvocationInput
            | Anchor::InvocationPre
            | Anchor::InvocationPost
            | Anchor::Activation
            | Anchor::TemporalInstant
            | Anchor::ProtocolInstant
            | Anchor::Finish => None,
        };
        runtime.anchors.push(w::Anchor {
            kind: anchor_kind(a),
            owner: w::Nullable(owner),
            binding: w::Nullable(None),
            locus: layout.locus(span)?,
        });
    }
    // Root and phase requirements use actual admitted nominal binder types.
    let mut root = None;
    for &original in &layout.binders {
        work.visit()?;
        let b = &scope.binders[original];
        if matches!(b.kind, BinderKind::Input | BinderKind::SelfValue) {
            let handle = layout.binder(original)?;
            let model = nominal(binder_type(typed, original)?, builder, work)?;
            root = Some((binders[handle.index as usize].value_type, model));
            break;
        }
    }
    if let Some((value_type, model)) = &root {
        for &(anchor, span) in &layout.anchors {
            let (kind, selected) = match anchor {
                Anchor::Activation => (w::BindingKind::WorkflowInstance, R::ObservationBinding),
                Anchor::Current | Anchor::TemporalInstant | Anchor::ProtocolInstant => {
                    (w::BindingKind::Snapshot, R::ObservationBinding)
                }
                Anchor::InvocationInput | Anchor::InvocationPre | Anchor::InvocationPost => {
                    (w::BindingKind::Invocation, R::ObservationBinding)
                }
                Anchor::Finish => (w::BindingKind::Closure, R::Progress),
                Anchor::Predicate
                | Anchor::Control(_)
                | Anchor::Fifo(_)
                | Anchor::Registration(_)
                | Anchor::CompensationActivation(_)
                | Anchor::Retry(_)
                | Anchor::Recovery(_) => continue,
            };
            let name = anchor_kind(anchor).as_str();
            let at = runtime.add(
                Requirement {
                    name,
                    kind,
                    selected,
                    value_type: Some(*value_type),
                    model: Some(model.clone()),
                    subject: runtime.declaration_subject(),
                    anchor,
                    requires: Vec::new(),
                    span,
                },
                work,
            )?;
            runtime.bind_anchor(anchor, at)?;
        }
    }
    let body = match &syntax.kind {
        c::DeclarationKind::Predicate {
            parameters, body, ..
        } => {
            let mut params = Vec::new();
            for parameter in parameters {
                work.charge(Dimension::Entries, 1)?;
                params.push(parameter_handle(
                    layout,
                    scope,
                    parameter,
                    BinderKind::Parameter,
                )?);
            }
            w::Body::Predicate {
                parameters: params,
                result: 0,
                root: layout.value(*body)?,
            }
        }
        c::DeclarationKind::State {
            kind,
            context: _,
            operation,
            body,
        } => {
            // An operation clause retains a BoundOperation, not a redundant
            // context-type occurrence. Its typed self binder owns that context.
            let context_export = root
                .as_ref()
                .map(|(_, model)| model.clone())
                .ok_or(Error::Invalid(Invalid::Model))?;
            let operation = if let Some(name) = operation {
                Some(operation_export(context, name.span, builder, work)?)
            } else {
                None
            };
            let clause_kind = match kind {
                ClauseKind::Invariant => w::ClauseKind::Invariant,
                ClauseKind::Precondition => w::ClauseKind::Pre,
                ClauseKind::Postcondition => w::ClauseKind::Post,
            };
            w::Body::State {
                clause_kind,
                context: context_export,
                operation: w::Nullable(operation),
                root: layout.value(*body)?,
            }
        }
        c::DeclarationKind::Temporal {
            input,
            clock,
            activation: activation_,
            captures: captures_,
            formula,
        } => {
            let definition = *profiles.first().ok_or(Error::Invalid(Invalid::Profile))?;
            let selected = meta.registered[definition as usize];
            work.bytes(clock.value.len().saturating_add(6))?;
            let clock_name = format!("clock:{}", clock.value);
            let at = runtime.add(
                Requirement {
                    name: &clock_name,
                    kind: w::BindingKind::Clock,
                    selected,
                    value_type: None,
                    model: None,
                    subject: runtime.declaration_subject(),
                    anchor: Anchor::TemporalInstant,
                    requires: Vec::new(),
                    span: clock.span,
                },
                work,
            )?;
            for (kind, suffix) in [
                (w::BindingKind::Progress, "progress"),
                (w::BindingKind::Closure, "closure"),
            ] {
                work.bytes(
                    clock_name
                        .len()
                        .saturating_add(suffix.len())
                        .saturating_add(1),
                )?;
                let name = format!("{clock_name}:{suffix}");
                work.charge(Dimension::Entries, 1)?;
                runtime.add(
                    Requirement {
                        name: &name,
                        kind,
                        selected: R::Progress,
                        value_type: None,
                        model: None,
                        subject: runtime.declaration_subject(),
                        anchor: Anchor::TemporalInstant,
                        requires: vec![at],
                        span: clock.span,
                    },
                    work,
                )?;
            }
            w::Body::Temporal {
                input: parameter_handle(layout, scope, input, BinderKind::Input)?,
                clock: at,
                activation: activation(activation_, layout, scope)?,
                captures: captures(captures_, layout, scope, work)?,
                root: layout.temporal(*formula)?,
            }
        }
        c::DeclarationKind::Protocol(p) => {
            let mut roles = Vec::new();
            for (i, role) in p.roles.iter().enumerate() {
                work.visit()?;
                // Model/type admission below may refuse before the retained
                // role record is built. Establish the complete owning locus
                // first so a multi-unit package cannot combine this span with
                // a stale source index from the preceding declaration.
                work.locus = Some(layout.locus(role.span)?);
                let ty = model_type(context, qualified_span(&role.model), work)?;
                let model = nominal(ty, builder, work)?;
                let value_type = builder.ty(ty, work)?;
                work.bytes(role.name.value.len().saturating_add(5))?;
                let name = format!("role:{}", role.name.value);
                let at = runtime.add(
                    Requirement {
                        name: &name,
                        kind: w::BindingKind::RoleInstance,
                        selected: R::ObservationBinding,
                        value_type: Some(value_type),
                        model: Some(model.clone()),
                        subject: w::Subject::Role {
                            role: layout.handle(index(i)?),
                        },
                        anchor: Anchor::ProtocolInstant,
                        requires: Vec::new(),
                        span: role.span,
                    },
                    work,
                )?;
                work.charge(Dimension::Entries, 1)?;
                roles.push(w::Role {
                    name: text(&role.name.value, work)?,
                    model,
                    instance: at,
                    locus: layout.locus(role.span)?,
                });
            }
            let channels = super::channels::lower(context, &roles, &mut runtime, builder, work)?;
            let mut relationships = Vec::new();
            for relationship in &p.relationships {
                work.visit()?;
                let (model, relation) =
                    relationship_authority(context, relationship, builder, work)?;
                work.bytes(relationship.name.value.len().saturating_add(13))?;
                let name = format!("relationship:{}", relationship.name.value);
                let binding = runtime.add(
                    Requirement {
                        name: &name,
                        kind: w::BindingKind::Relationship,
                        selected: R::ObservationBinding,
                        value_type: None,
                        model: Some(model.clone()),
                        subject: w::Subject::Declaration {
                            declaration: layout.declaration,
                        },
                        anchor: Anchor::ProtocolInstant,
                        requires: Vec::new(),
                        span: relationship.span,
                    },
                    work,
                )?;
                runtime.set_relation(binding, relation)?;
                work.charge(Dimension::Entries, 1)?;
                relationships.push(w::Relationship {
                    name: text(&relationship.name.value, work)?,
                    model,
                    binding,
                    locus: layout.locus(relationship.span)?,
                });
            }
            let mut temporal_requirements = Vec::new();
            for reference in context.references {
                work.visit()?;
                if reference.kind == crate::linking::composed::DependencyKind::TemporalRequirement {
                    let target = reference
                        .target
                        .and_then(|d| meta.declaration_indices.get(&d))
                        .copied()
                        .ok_or(Error::Invalid(Invalid::Dependency))?;
                    work.charge(Dimension::Entries, 1)?;
                    temporal_requirements.push(target);
                }
            }
            temporal_requirements.sort_unstable();
            temporal_requirements.dedup();
            let controls =
                super::controls::controls(context, &roles, &channels, &mut runtime, builder, work)?;
            let compensations =
                compensations::lower(context, &roles, &controls, &mut runtime, builder, work)?;
            let causal_edges = super::controls::edges(&controls, layout, work)?;
            let finish = parameter_handle(layout, scope, &p.finish.parameter, BinderKind::Finish)?;
            let closure = runtime.anchor_binding(Anchor::Finish, work)?;
            w::Body::Protocol {
                input: parameter_handle(layout, scope, &p.input, BinderKind::Input)?,
                activation: activation(&p.activation, layout, scope)?,
                captures: captures(&p.captures, layout, scope, work)?,
                roles,
                relationships,
                channels,
                compensations,
                temporal_requirements,
                controls,
                causal_edges,
                run: layout.control(p.run)?,
                finish: Box::new(w::Finish {
                    name: text(&p.finish.name.value, work)?,
                    binder: finish,
                    constraint: layout.value(p.finish.constraint)?,
                    closure,
                    locus: layout.locus(p.finish.span)?,
                }),
            }
        }
    };
    for need in super::populations::collect(context, work)? {
        work.locus = Some(layout.locus(need.span)?);
        let model = builder.export(
            need.model,
            w::ExportKind::Population,
            need.role.record.as_str(),
            Some(need.role.universe.as_str()),
            work,
        )?;
        let value_type = builder.ty(
            &NativeType::Object {
                model: need.model,
                role: need.role,
            },
            work,
        )?;
        let anchor = layout.anchor(need.anchor)?.index;
        work.visit()?;
        let prerequisite = runtime
            .anchors
            .get(anchor as usize)
            .ok_or(Error::Invalid(Invalid::Binding))?
            .binding
            .0;
        let mut requires = Vec::new();
        if let Some(prerequisite) = prerequisite {
            work.charge(Dimension::Entries, 1)?;
            requires.push(prerequisite);
        }
        let name = population_name("population", &model, anchor, work)?;
        let population = runtime.add(
            Requirement {
                name: &name,
                kind: w::BindingKind::Population,
                selected: R::ObservationBinding,
                value_type: Some(value_type),
                model: Some(model.clone()),
                subject: runtime.declaration_subject(),
                anchor: need.anchor,
                requires,
                span: need.span,
            },
            work,
        )?;
        let name = population_name("population-closure", &model, anchor, work)?;
        work.charge(Dimension::Entries, 1)?;
        runtime.add(
            Requirement {
                name: &name,
                kind: w::BindingKind::Closure,
                selected: R::Progress,
                value_type: Some(value_type),
                model: Some(model),
                subject: runtime.declaration_subject(),
                anchor: need.anchor,
                requires: vec![population],
                span: need.span,
            },
            work,
        )?;
    }
    Ok((runtime.anchors, runtime.bindings, body))
}

fn relationship_authority(
    context: &Declaration<'_, '_>,
    relationship: &c::Relationship,
    builder: &ValueBuilder<'_>,
    work: &mut Work,
) -> Result<(w::ExportRef, u32), Error> {
    let bound = relationship_binding(context, relationship, work)?;
    let export = bound.export();
    if export.path.len() != 1 {
        return Err(Error::Invalid(Invalid::Model));
    }
    let model = builder.export(
        bound.model(),
        w::ExportKind::Relationship,
        &export.path[0],
        None,
        work,
    )?;
    let relation = builder.producer_relation(bound.model(), work)?;
    Ok((model, relation))
}

pub(super) fn relationship_binding<'a, 'm>(
    context: &'a Declaration<'_, 'm>,
    relationship: &c::Relationship,
    work: &mut Work,
) -> Result<&'a crate::linking::composed::models::BoundRelationship<'m>, Error> {
    let span = Span {
        start: relationship.model.model.span.start,
        end: relationship.model.name.span.end,
    };
    for occurrence in &context.exports.occurrences {
        work.visit()?;
        if occurrence.span != span {
            continue;
        }
        if let ModelTarget::Relationship(bound) = &occurrence.target {
            return Ok(bound);
        }
        return Err(Error::Invalid(Invalid::Model));
    }
    Err(Error::Invalid(Invalid::Model))
}

fn population_name(
    prefix: &str,
    model: &w::ExportRef,
    anchor: u32,
    work: &mut Work,
) -> Result<String, Error> {
    let digits = |value: u32| value.checked_ilog10().map_or(1, |power| power as usize + 1);
    work.bytes(prefix.len() + 3 + digits(model.model) + digits(model.export) + digits(anchor))?;
    Ok(format!(
        "{prefix}:{}:{}:{anchor}",
        model.model, model.export
    ))
}
pub(super) fn operation_export(
    context: &Declaration<'_, '_>,
    span: Span,
    builder: &ValueBuilder<'_>,
    work: &mut Work,
) -> Result<w::ExportRef, Error> {
    let report = context.exports;
    for o in &report.occurrences {
        work.visit()?;
        if o.span.start <= span.start && span.end <= o.span.end {
            if let ModelTarget::Operation(op) = &o.target {
                return builder.export(
                    op.model(),
                    w::ExportKind::Operation,
                    op.role().context.as_str(),
                    Some(op.role().name.as_str()),
                    work,
                );
            }
        }
    }
    Err(Error::Invalid(Invalid::Model))
}
pub(super) fn selected_profile(
    context: &Declaration<'_, '_>,
    profile: &Spanned<String>,
    work: &mut Work,
) -> Result<u32, Error> {
    if context.profile_uses.len() != context.profiles.len() {
        return Err(Error::Invalid(Invalid::Profile));
    }
    for (selected, index) in context.profile_uses.iter().zip(context.profiles) {
        work.visit()?;
        if selected.alias.span == profile.span {
            return Ok(*index);
        }
    }
    Err(Error::Invalid(Invalid::Profile))
}
pub(super) fn structural(
    scope: &DeclarationScope,
    span: Span,
    kind: scopes::StructuralKind,
    layout: &DeclLayout,
    work: &mut Work,
) -> Result<w::Handle, Error> {
    structural_symbol(scope, span, kind, work)?
        .control
        .map(|control| layout.control(control))
        .ok_or(Error::Invalid(Invalid::Reference))?
}
pub(super) fn structural_symbol<'a>(
    scope: &'a DeclarationScope,
    span: Span,
    kind: scopes::StructuralKind,
    work: &mut Work,
) -> Result<&'a scopes::Symbol, Error> {
    for reference in &scope.references {
        work.visit()?;
        if reference.span == span && reference.required == kind {
            let symbol = scope
                .symbols
                .get(
                    reference
                        .target
                        .ok_or(Error::Invalid(Invalid::Reference))?
                        .index(),
                )
                .ok_or(Error::Invalid(Invalid::Reference))?;
            return Ok(symbol);
        }
    }
    Err(Error::Invalid(Invalid::Reference))
}

fn qualified_span(name: &c::QualifiedName) -> Span {
    Span {
        start: name.model.span.start,
        end: name.name.span.end,
    }
}
