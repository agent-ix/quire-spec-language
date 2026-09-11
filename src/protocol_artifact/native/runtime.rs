// SPDX-License-Identifier: AGPL-3.0-only
//! FR-042: authored family bodies and static requirements for later runtime input.
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
) -> Result<(), Error> {
    for (local, &original) in layout.binders.iter().enumerate() {
        if let BinderType::Initializer(value) = scope.binders[original].ty {
            binders[local].initializer = w::Nullable(Some(layout.value(value)?));
        }
    }
    let captures = match &syntax.kind {
        c::DeclarationKind::Temporal { captures, .. } => captures.as_slice(),
        c::DeclarationKind::Protocol(p) => p.captures.as_slice(),
        c::DeclarationKind::Predicate { .. } | c::DeclarationKind::State { .. } => &[],
    };
    for capture in captures {
        let binder = layout.binder_at(scope, capture.parameter.name.span, BinderKind::Capture)?;
        binders[binder.index as usize].initializer =
            w::Nullable(Some(layout.value(capture.value)?));
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
    fn declaration_subject(&self) -> w::Subject {
        w::Subject::Declaration {
            declaration: self.layout.declaration,
        }
    }
    pub(super) fn bind_anchor(&mut self, anchor: Anchor, binding: u32) -> Result<(), Error> {
        let index = self.layout.anchor(anchor)?.index as usize;
        self.anchors[index].binding = w::Nullable(Some(binding));
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
        NativeType::Reference { .. }
        | NativeType::Boolean
        | NativeType::Option(_)
        | NativeType::Sequence { .. } => Err(Error::Unsupported(Unsupported::Export)),
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
            Anchor::Registration(_)
            | Anchor::CompensationActivation(_)
            | Anchor::Retry(_)
            | Anchor::Recovery(_) => return Err(Error::Unsupported(Unsupported::Export)),
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
            if !p.relationships.is_empty()
                || p.requirements
                    .iter()
                    .any(|r| matches!(r, c::ProtocolRequirement::Compensation(_)))
            {
                return Err(Error::Unsupported(Unsupported::Export));
            }
            let mut roles = Vec::new();
            for (i, role) in p.roles.iter().enumerate() {
                work.visit()?;
                let ty = model_type(context, qualified_span(&role.model), work)?;
                let model = nominal(ty, builder, work)?;
                let value_type = builder.ty(ty, work)?;
                let at = runtime.add(
                    Requirement {
                        name: &format!("role:{}", role.name.value),
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
            let causal_edges = super::controls::edges(&controls, layout, work)?;
            let finish = parameter_handle(layout, scope, &p.finish.parameter, BinderKind::Finish)?;
            let closure = runtime.anchors[layout.anchor(Anchor::Finish)?.index as usize]
                .binding
                .0
                .ok_or(Error::Invalid(Invalid::Binding))?;
            w::Body::Protocol {
                input: parameter_handle(layout, scope, &p.input, BinderKind::Input)?,
                activation: activation(&p.activation, layout, scope)?,
                captures: captures(&p.captures, layout, scope, work)?,
                roles,
                relationships: Vec::new(),
                channels,
                compensations: Vec::new(),
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
    Ok((runtime.anchors, runtime.bindings, body))
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
            return symbol
                .control
                .map(|control| layout.control(control))
                .ok_or(Error::Unsupported(Unsupported::Export))?;
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
