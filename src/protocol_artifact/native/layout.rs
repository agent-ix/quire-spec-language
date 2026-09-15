// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-036/040/042: original occurrence identities mapped to canonical local tables.
use crate::checking::composed::DeclarationTypes;
use crate::linking::composed::scopes::{Anchor, BinderKind, BinderType, DeclarationScope};
use crate::linking::composed::{DeclarationId, SyntaxNamespace};
use crate::protocol_artifact::{wire as w, work::Work, Dimension, Error, Invalid};
use crate::syntax::{composed as c, ExprId, ExprKind, UnaryOp};
use crate::Span;
use std::collections::BTreeMap;

use super::types::index;

pub(super) struct DeclLayout {
    pub declaration: u32,
    pub source: u32,
    pub values: Vec<ExprId>,
    pub controls: Vec<c::ControlId>,
    pub compensations: Vec<usize>,
    pub temporal: Vec<c::TemporalId>,
    pub binders: Vec<usize>,
    pub anchors: Vec<(Anchor, Span)>,
    pub scopes: Vec<w::Scope>,
    value_indices: BTreeMap<usize, u32>,
    control_indices: BTreeMap<usize, u32>,
    compensation_indices: BTreeMap<usize, u32>,
    temporal_indices: BTreeMap<usize, u32>,
    binder_indices: BTreeMap<usize, u32>,
    value_scopes: BTreeMap<usize, u32>,
    value_anchors: BTreeMap<usize, Anchor>,
    binder_scopes: BTreeMap<usize, u32>,
    calls: BTreeMap<usize, u32>,
}
impl DeclLayout {
    pub fn handle(&self, index: u32) -> w::Handle {
        w::Handle {
            declaration: self.declaration,
            index,
        }
    }
    pub fn locus(&self, span: Span) -> Result<w::Locus, Error> {
        Ok(w::Locus {
            source: self.source,
            span: w::Span {
                start: index(span.start)?,
                end: index(span.end)?,
            },
        })
    }
    pub fn value(&self, id: ExprId) -> Result<w::Handle, Error> {
        self.lookup(&self.value_indices, id.0)
    }
    pub fn control(&self, id: c::ControlId) -> Result<w::Handle, Error> {
        self.lookup(&self.control_indices, id.0)
    }
    pub fn compensation(&self, requirement: usize) -> Result<w::Handle, Error> {
        self.lookup(&self.compensation_indices, requirement)
    }
    pub fn compensation_named(
        &self,
        protocol: &c::Protocol,
        name: Span,
        work: &mut Work,
    ) -> Result<w::Handle, Error> {
        for &requirement in &self.compensations {
            work.visit()?;
            let Some(c::ProtocolRequirement::Compensation(value)) =
                protocol.requirements.get(requirement)
            else {
                return Err(Error::Invalid(Invalid::Reference));
            };
            if value.name.span == name {
                return self.compensation(requirement);
            }
        }
        Err(Error::Invalid(Invalid::Reference))
    }
    pub fn temporal(&self, id: c::TemporalId) -> Result<w::Handle, Error> {
        self.lookup(&self.temporal_indices, id.0)
    }
    pub fn binder(&self, id: usize) -> Result<w::Handle, Error> {
        self.lookup(&self.binder_indices, id)
    }
    pub fn scope(&self, id: ExprId) -> Result<w::Handle, Error> {
        self.lookup(&self.value_scopes, id.0)
    }
    pub fn value_anchor(&self, id: ExprId) -> Result<w::Handle, Error> {
        self.anchor(
            *self
                .value_anchors
                .get(&id.0)
                .ok_or(Error::Invalid(Invalid::Reference))?,
        )
    }
    pub fn binder_scope(&self, id: usize) -> Result<w::Handle, Error> {
        self.lookup(&self.binder_scopes, id)
    }
    pub fn call_target(&self, id: ExprId) -> Result<u32, Error> {
        self.calls
            .get(&id.0)
            .copied()
            .ok_or(Error::Invalid(Invalid::Call))
    }
    pub fn anchor(&self, anchor: Anchor) -> Result<w::Handle, Error> {
        let position = self
            .anchors
            .iter()
            .position(|(value, _)| *value == anchor)
            .ok_or(Error::Invalid(Invalid::Reference))?;
        Ok(self.handle(index(position)?))
    }
    pub fn binder_at(
        &self,
        scope: &DeclarationScope,
        span: Span,
        kind: BinderKind,
    ) -> Result<w::Handle, Error> {
        let mut found = None;
        for &id in &self.binders {
            let b = &scope.binders[id];
            if b.span == span && b.kind == kind {
                if found.is_some() {
                    return Err(Error::Invalid(Invalid::Duplicate));
                }
                found = Some(self.binder(id)?);
            }
        }
        found.ok_or(Error::Invalid(Invalid::Reference))
    }
    fn lookup(&self, map: &BTreeMap<usize, u32>, id: usize) -> Result<w::Handle, Error> {
        map.get(&id)
            .copied()
            .map(|i| self.handle(i))
            .ok_or(Error::Invalid(Invalid::Reference))
    }
}
pub(super) fn build(
    namespace: &SyntaxNamespace,
    typed: &DeclarationTypes<'_>,
    scope: &DeclarationScope,
    declaration: u32,
    source: u32,
    declarations: &BTreeMap<DeclarationId, u32>,
    work: &mut Work,
) -> Result<DeclLayout, Error> {
    let unit = namespace
        .unit(typed.unit())
        .ok_or(Error::Invalid(Invalid::Owner))?;
    let syntax = namespace
        .syntax(typed.declaration())
        .ok_or(Error::Invalid(Invalid::Owner))?;
    let mut result = DeclLayout {
        declaration,
        source,
        values: Vec::new(),
        controls: Vec::new(),
        compensations: Vec::new(),
        temporal: Vec::new(),
        binders: Vec::new(),
        anchors: Vec::new(),
        scopes: Vec::new(),
        value_indices: BTreeMap::new(),
        control_indices: BTreeMap::new(),
        compensation_indices: BTreeMap::new(),
        temporal_indices: BTreeMap::new(),
        binder_indices: BTreeMap::new(),
        value_scopes: BTreeMap::new(),
        value_anchors: BTreeMap::new(),
        binder_scopes: BTreeMap::new(),
        calls: BTreeMap::new(),
    };
    work.charge(Dimension::Entries, 1)?;
    result.scopes.push(w::Scope {
        parent: w::Nullable(None),
        locus: result.locus(syntax.span)?,
    });
    if let c::DeclarationKind::Protocol(protocol) = &syntax.kind {
        for (original, requirement) in protocol.requirements.iter().enumerate() {
            work.visit()?;
            if matches!(requirement, c::ProtocolRequirement::Compensation(_)) {
                let position = index(result.compensations.len())?;
                work.charge(Dimension::Entries, 2)?;
                result.compensations.push(original);
                result.compensation_indices.insert(original, position);
            }
        }
    }
    let mut folded = std::collections::BTreeSet::new();
    for node in typed.nodes() {
        work.visit()?;
        if let c::ValueKind::Shared(ExprKind::Unary {
            op: UnaryOp::Negate,
            argument,
        }) = &unit.expressions()[node.expression.0].kind
        {
            if matches!(
                unit.expressions()[argument.0].kind,
                c::ValueKind::Shared(ExprKind::Integer(_))
            ) {
                work.charge(Dimension::Entries, 1)?;
                folded.insert(argument.0);
            }
        }
    }
    for node in typed.nodes() {
        work.visit()?;
        if !folded.contains(&node.expression.0) {
            work.charge(Dimension::Entries, 3)?;
            result.values.push(node.expression);
        }
    }
    result.values.sort_by_key(|id| {
        let s = unit.expressions()[id.0].span;
        (s.start, s.end, id.0)
    });
    for (i, id) in result.values.iter().enumerate() {
        result.value_indices.insert(id.0, index(i)?);
        result.value_scopes.insert(id.0, 0);
    }
    let control = c::arena::owned_range(unit.controls(), syntax.span, |n| n.span, || work.visit())?;
    for i in control {
        work.charge(Dimension::Entries, 2)?;
        result.controls.push(c::ControlId(i));
    }
    result.controls.sort_by_key(|id| {
        let s = unit.controls()[id.0].span;
        (s.start, s.end, id.0)
    });
    for (i, id) in result.controls.iter().enumerate() {
        result.control_indices.insert(id.0, index(i)?);
    }
    for ordinal in 0..result.controls.len() {
        let id = result.controls[ordinal];
        add_anchor(
            &mut result,
            Anchor::Control(id),
            unit.controls()[id.0].span,
            work,
        )?;
    }

    let temporal = c::arena::owned_range(
        unit.temporal_nodes(),
        syntax.span,
        |n| n.span,
        || work.visit(),
    )?;
    for i in temporal {
        work.charge(Dimension::Entries, 2)?;
        result.temporal.push(c::TemporalId(i));
    }
    result.temporal.sort_by_key(|id| {
        let s = unit.temporal_nodes()[id.0].span;
        (s.start, s.end, id.0)
    });
    for (i, id) in result.temporal.iter().enumerate() {
        result.temporal_indices.insert(id.0, index(i)?);
    }
    for (i, b) in scope.binders.iter().enumerate() {
        work.visit()?;
        work.charge(Dimension::Entries, 3)?;
        result.binders.push(i);
        result.binder_scopes.insert(i, 0);
        add_anchor(
            &mut result,
            b.anchor,
            anchor_span(b.anchor, syntax, unit)?,
            work,
        )?;
    }
    result.binders.sort_by_key(|&id| {
        let s = scope.binders[id].span;
        (s.start, s.end, id)
    });
    for (i, id) in result.binders.iter().enumerate() {
        result.binder_indices.insert(*id, index(i)?);
    }
    for node in typed.nodes() {
        work.visit()?;
        if let Some(a) = node.anchor {
            add_anchor(&mut result, a, anchor_span(a, syntax, unit)?, work)?;
        }
    }
    match &syntax.kind {
        c::DeclarationKind::Predicate { .. } => {
            add_anchor(&mut result, Anchor::Predicate, syntax.span, work)?
        }
        c::DeclarationKind::State { .. } => {}
        c::DeclarationKind::Temporal { activation, .. } => add_anchor(
            &mut result,
            Anchor::Activation,
            activation_span(activation),
            work,
        )?,
        c::DeclarationKind::Protocol(p) => add_anchor(
            &mut result,
            Anchor::Activation,
            activation_span(&p.activation),
            work,
        )?,
    }
    if let c::DeclarationKind::Protocol(p) = &syntax.kind {
        add_anchor(&mut result, Anchor::Finish, p.finish.span, work)?;
    }
    for ordinal in 0..result.values.len() {
        let id = result.values[ordinal];
        work.visit()?;
        let span = unit.expressions()[id.0].span;
        let anchor = evaluation_anchor(span, syntax, unit, &result.controls, work)?;
        add_anchor(
            &mut result,
            anchor,
            anchor_span(anchor, syntax, unit)?,
            work,
        )?;
        work.charge(Dimension::Entries, 1)?;
        result.value_anchors.insert(id.0, anchor);
    }
    // A pre(...) subtree evaluates against the actual invocation pre anchor.
    for ordinal in 0..result.values.len() {
        let id = result.values[ordinal];
        work.visit()?;
        if let c::ValueKind::Shared(ExprKind::Call {
            builtin: crate::syntax::Builtin::Pre,
            argument,
        }) = &unit.expressions()[id.0].kind
        {
            let span = unit.expressions()[argument.0].span;
            add_anchor(
                &mut result,
                Anchor::InvocationPre,
                anchor_span(Anchor::InvocationPre, syntax, unit)?,
                work,
            )?;
            for child in &result.values {
                work.visit()?;
                let s = unit.expressions()[child.0].span;
                if span.start <= s.start && s.end <= span.end {
                    result.value_anchors.insert(child.0, Anchor::InvocationPre);
                }
            }
        }
    }
    for occurrence in &scope.values {
        work.visit()?;
        if result
            .value_anchors
            .get(&occurrence.expression.0)
            .is_some_and(|a| *a != occurrence.evaluation_anchor)
        {
            return Err(Error::Invalid(Invalid::Scope));
        }
    }
    result.anchors.sort_by_key(|(_, s)| (s.start, s.end));
    // Pre and post self slots share authored spelling, but select different
    // immutable observations. Keep their environments distinct without lookup.
    if matches!(
        syntax.kind,
        c::DeclarationKind::State {
            kind: crate::syntax::ClauseKind::Postcondition,
            ..
        }
    ) {
        for anchor in [Anchor::InvocationPre, Anchor::InvocationPost] {
            work.charge(Dimension::Entries, 1)?;
            let i = index(result.scopes.len())?;
            result.scopes.push(w::Scope {
                parent: w::Nullable(Some(result.handle(0))),
                locus: result.locus(syntax.span)?,
            });
            for (binder, b) in scope.binders.iter().enumerate() {
                work.visit()?;
                if b.anchor == anchor
                    && matches!(b.kind, BinderKind::SelfValue | BinderKind::ResultValue)
                {
                    result.binder_scopes.insert(binder, i);
                }
            }
            for id in &result.values {
                work.visit()?;
                if result.value_anchors.get(&id.0) == Some(&anchor) {
                    result.value_scopes.insert(id.0, i);
                }
            }
        }
    }
    // Activation trigger and final observation are separate authored regions.
    // Parameter/capture values remain the already-resolved immutable identities.
    let activation_region = match &syntax.kind {
        c::DeclarationKind::Temporal {
            activation,
            captures,
            ..
        } => Some((activation_span(activation), captures.as_slice())),
        c::DeclarationKind::Protocol(p) => {
            Some((activation_span(&p.activation), p.captures.as_slice()))
        }
        c::DeclarationKind::Predicate { .. } | c::DeclarationKind::State { .. } => None,
    };
    if let Some((mut region, captures)) = activation_region {
        for capture in captures {
            work.visit()?;
            region.end = region.end.max(capture.span.end);
        }
        let ids = scope
            .binders
            .iter()
            .enumerate()
            .filter(|(_, b)| b.kind == BinderKind::Trigger)
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        for binder in ids {
            phase_scope(&mut result, unit, region, binder, work)?;
        }
    }
    if let c::DeclarationKind::Protocol(p) = &syntax.kind {
        for (binder, value) in scope.binders.iter().enumerate() {
            work.visit()?;
            if value.kind == BinderKind::Finish {
                phase_scope(&mut result, unit, p.finish.span, binder, work)?;
            }
        }
        for (original, requirement) in p.requirements.iter().enumerate() {
            work.visit()?;
            if let c::ProtocolRequirement::Compensation(value) = requirement {
                compensation_scopes(&mut result, unit, scope, value, original, work)?;
            }
        }
    }
    // Immutable captured/event slots keep their exact bound read regions. A
    // joined record keeps the original BinderId; no spelling-based lookup or
    // synthetic final-iteration result is introduced during lowering.
    for (binder, bound) in scope.binders.iter().enumerate() {
        work.visit()?;
        if matches!(bound.kind, BinderKind::Capture | BinderKind::EventRecord) {
            let mut region = bound.span;
            for read in &scope.values {
                work.visit()?;
                if read.target.index() == binder {
                    region.start = region.start.min(read.span.start);
                    region.end = region.end.max(read.span.end);
                }
            }
            work.charge(Dimension::Entries, 1)?;
            let i = index(result.scopes.len())?;
            result.scopes.push(w::Scope {
                parent: w::Nullable(Some(result.handle(0))),
                locus: result.locus(region)?,
            });
            result.binder_scopes.insert(binder, i);
        }
        if let Anchor::Fifo(channel) = bound.anchor {
            let region = anchor_span(Anchor::Fifo(channel), syntax, unit)?;
            phase_scope(&mut result, unit, region, binder, work)?;
            let i = *result
                .binder_scopes
                .get(&binder)
                .ok_or(Error::Invalid(Invalid::Scope))?;
            // The source resolver starts FIFO keys in an isolated environment.
            result.scopes[i as usize].parent = w::Nullable(None);
        }
    }
    // Preserve lexical let/query owners from their actual initializer/domain.
    // The binder scope includes its token and body; the initializer/collection
    // stays in the parent, including for a query nested in another collection.
    let mut regions = Vec::new();
    for id in &result.values {
        work.visit()?;
        let binding = match &unit.expressions()[id.0].kind {
            c::ValueKind::Shared(ExprKind::Let { name, value, body }) => {
                Some((name.span, BinderKind::Let, *value, *body))
            }
            c::ValueKind::Shared(ExprKind::Quantifier {
                name,
                domain,
                predicate,
                ..
            }) => Some((name.span, BinderKind::Query, *domain, *predicate)),
            c::ValueKind::Query {
                binder,
                domain,
                body,
                ..
            } => Some((binder.span, BinderKind::Query, *domain, *body)),
            _ => None,
        };
        if let Some((span, kind, input, body)) = binding {
            let mut found = None;
            for (index, binder) in scope.binders.iter().enumerate() {
                work.visit()?;
                let same_input = match (&binder.ty, kind) {
                    (BinderType::Initializer(value), BinderKind::Let)
                    | (BinderType::ElementOf(value), BinderKind::Query) => *value == input,
                    _ => false,
                };
                if binder.kind == kind
                    && binder.span == span
                    && same_input
                    && found.replace(index).is_some()
                {
                    return Err(Error::Invalid(Invalid::Duplicate));
                }
            }
            let binder = found.ok_or(Error::Invalid(Invalid::Scope))?;
            work.charge(Dimension::Entries, 1)?;
            regions.push((
                Span {
                    start: span.start,
                    end: unit.expressions()[body.0].span.end,
                },
                unit.expressions()[body.0].span,
                binder,
                *id,
            ));
        }
    }
    regions.sort_by_key(|(s, _, _, _)| (s.start, std::cmp::Reverse(s.end)));
    for (region, body, binder, expression) in regions {
        work.charge(Dimension::Entries, 1)?;
        let i = index(result.scopes.len())?;
        // Prior outer regions assigned only their bodies. A nested query in an
        // outer collection therefore retains the true surrounding environment,
        // even though the outer binder token precedes that collection in text.
        work.visit()?;
        let parent = *result
            .value_scopes
            .get(&expression.0)
            .ok_or(Error::Invalid(Invalid::Scope))?;
        result.scopes.push(w::Scope {
            parent: w::Nullable(Some(result.handle(parent))),
            locus: result.locus(region)?,
        });
        result.binder_scopes.insert(binder, i);
        for id in &result.values {
            work.visit()?;
            let span = unit.expressions()[id.0].span;
            if body.start <= span.start && span.end <= body.end {
                result.value_scopes.insert(id.0, i);
            }
        }
    }
    // Preserve each already-resolved read's immutable owner, including records
    // re-exported after an all-branch join. ScopeReport remains the authority for
    // path availability; this pass does not rerun its native name resolver.
    for read in &scope.values {
        work.visit()?;
        let target = *result
            .binder_scopes
            .get(&read.target.index())
            .ok_or(Error::Invalid(Invalid::Scope))?;
        let mut current = result.value_scopes.get(&read.expression.0).copied();
        let mut visible = false;
        while let Some(i) = current {
            work.visit()?;
            if i == target {
                visible = true;
                break;
            }
            current = result.scopes[i as usize].parent.0.as_ref().map(|p| p.index);
        }
        if !visible {
            let region = &result.scopes[target as usize].locus.span;
            // Capture/event regions are built from these admitted reads; their
            // visibility authority is the ScopeReport cross-check above. For
            // authored phase/let regions, this also checks lexical containment.
            if region.start as usize > read.span.start || read.span.end > region.end as usize {
                return Err(Error::Invalid(Invalid::Scope));
            }
            result.value_scopes.insert(read.expression.0, target);
        }
    }
    // Canonical source order is independent of the lowering traversal. Remap
    // every parent, binder and value scope together after all regions exist.
    work.charge(Dimension::Entries, result.scopes.len().saturating_mul(2))?;
    let mut order = (0..result.scopes.len()).collect::<Vec<_>>();
    order.sort_by_key(|&i| {
        let s = &result.scopes[i].locus.span;
        (s.start, s.end, i)
    });
    let mut remap = vec![0; order.len()];
    for (new, &old) in order.iter().enumerate() {
        remap[old] = index(new)?;
    }
    // Root starts at the declaration keyword, before any generated child region.
    let mut old = std::mem::take(&mut result.scopes)
        .into_iter()
        .map(Some)
        .collect::<Vec<_>>();
    for i in order {
        let mut scope = old[i].take().ok_or(Error::Invalid(Invalid::Scope))?;
        if let Some(parent) = &mut scope.parent.0 {
            parent.index = remap[parent.index as usize];
        }
        result.scopes.push(scope);
    }
    for scope in result.value_scopes.values_mut() {
        *scope = remap[*scope as usize];
    }
    for scope in result.binder_scopes.values_mut() {
        *scope = remap[*scope as usize];
    }

    for reference in namespace
        .declaration(typed.declaration())
        .ok_or(Error::Invalid(Invalid::Owner))?
        .references()
    {
        work.visit()?;
        if let crate::linking::composed::DependencySite::Expression(id) = reference.site {
            let target = reference
                .target
                .and_then(|target| declarations.get(&target))
                .copied()
                .ok_or(Error::Invalid(Invalid::Call))?;
            work.charge(Dimension::Entries, 1)?;
            result.calls.insert(id.0, target);
        }
    }
    Ok(result)
}
fn add_anchor(
    layout: &mut DeclLayout,
    anchor: Anchor,
    span: Span,
    work: &mut Work,
) -> Result<(), Error> {
    for (existing, _) in &layout.anchors {
        work.visit()?;
        if *existing == anchor {
            return Ok(());
        }
    }
    work.charge(Dimension::Entries, 1)?;
    layout.anchors.push((anchor, span));
    Ok(())
}
pub(super) fn activation_span(a: &c::Activation) -> Span {
    match a {
        c::Activation::Origin { span } | c::Activation::Each { span, .. } => *span,
    }
}
fn anchor_span(a: Anchor, decl: &c::Declaration, unit: &c::ComposedUnit) -> Result<Span, Error> {
    Ok(match a {
        Anchor::Predicate
        | Anchor::Current
        | Anchor::InvocationInput
        | Anchor::InvocationPre
        | Anchor::InvocationPost => decl.span,
        Anchor::Control(id) => {
            unit.controls()
                .get(id.0)
                .ok_or(Error::Invalid(Invalid::Owner))?
                .span
        }
        Anchor::Activation => match &decl.kind {
            c::DeclarationKind::Temporal { activation, .. } => activation_span(activation),
            c::DeclarationKind::Protocol(p) => activation_span(&p.activation),
            _ => return Err(Error::Invalid(Invalid::Owner)),
        },
        Anchor::TemporalInstant => match &decl.kind {
            c::DeclarationKind::Temporal { input, .. } => input.span,
            _ => return Err(Error::Invalid(Invalid::Owner)),
        },
        Anchor::ProtocolInstant => match &decl.kind {
            c::DeclarationKind::Protocol(p) => p.input.span,
            _ => return Err(Error::Invalid(Invalid::Owner)),
        },
        Anchor::Finish => match &decl.kind {
            c::DeclarationKind::Protocol(p) => p.finish.span,
            _ => return Err(Error::Invalid(Invalid::Owner)),
        },
        Anchor::Fifo(index) => match &decl.kind {
            c::DeclarationKind::Protocol(p) => {
                p.channels
                    .get(index)
                    .ok_or(Error::Invalid(Invalid::Owner))?
                    .span
            }
            _ => return Err(Error::Invalid(Invalid::Owner)),
        },
        Anchor::Registration(index)
        | Anchor::CompensationActivation(index)
        | Anchor::Retry(index)
        | Anchor::Recovery(index) => {
            let c::DeclarationKind::Protocol(protocol) = &decl.kind else {
                return Err(Error::Invalid(Invalid::Owner));
            };
            let Some(c::ProtocolRequirement::Compensation(value)) =
                protocol.requirements.get(index)
            else {
                return Err(Error::Invalid(Invalid::Reference));
            };
            match a {
                Anchor::Registration(_) => value.forward.span,
                Anchor::CompensationActivation(_) => value.trigger.span,
                Anchor::Retry(_) => Span {
                    start: value.earlier.span.start,
                    end: value.later.span.end,
                },
                Anchor::Recovery(_) => value.recovery.span,
                _ => return Err(Error::Invalid(Invalid::Owner)),
            }
        }
    })
}

fn evaluation_anchor(
    span: Span,
    decl: &c::Declaration,
    unit: &c::ComposedUnit,
    controls: &[c::ControlId],
    work: &mut Work,
) -> Result<Anchor, Error> {
    let contains = |outer: Span| outer.start <= span.start && span.end <= outer.end;
    Ok(match &decl.kind {
        c::DeclarationKind::Predicate { .. } => Anchor::Predicate,
        c::DeclarationKind::State { kind, .. } => match kind {
            crate::syntax::ClauseKind::Invariant => Anchor::Current,
            crate::syntax::ClauseKind::Precondition => Anchor::InvocationPre,
            crate::syntax::ClauseKind::Postcondition => Anchor::InvocationPost,
        },
        c::DeclarationKind::Temporal {
            activation,
            captures,
            ..
        } => {
            if contains(activation_span(activation))
                || captures.iter().any(|capture| contains(capture.span))
            {
                Anchor::Activation
            } else {
                Anchor::TemporalInstant
            }
        }
        c::DeclarationKind::Protocol(p) => {
            let mut anchor = Anchor::ProtocolInstant;
            for (original, requirement) in p.requirements.iter().enumerate() {
                work.visit()?;
                if let c::ProtocolRequirement::Compensation(value) = requirement {
                    for capture in &value.registration_captures {
                        work.visit()?;
                        if contains(capture.span) {
                            anchor = Anchor::Registration(original);
                        }
                    }
                    if contains(expression_span(unit, value.guard)?) {
                        anchor = Anchor::CompensationActivation(original);
                    }
                    for capture in &value.activation_captures {
                        work.visit()?;
                        if contains(capture.span) {
                            anchor = Anchor::CompensationActivation(original);
                        }
                    }
                    if contains(expression_span(unit, value.retry)?) {
                        anchor = Anchor::Retry(original);
                    }
                    if contains(expression_span(unit, value.recover)?) {
                        anchor = Anchor::Recovery(original);
                    }
                }
            }
            for (index, channel) in p.channels.iter().enumerate() {
                work.visit()?;
                if let c::Ordering::Fifo { key, .. } = &channel.ordering {
                    if contains(unit.expressions()[key.0].span) {
                        anchor = Anchor::Fifo(index);
                    }
                }
            }
            if contains(activation_span(&p.activation))
                || p.captures.iter().any(|capture| contains(capture.span))
            {
                anchor = Anchor::Activation;
            }
            let mut size = usize::MAX;
            for &id in controls {
                work.visit()?;
                let s = unit.controls()[id.0].span;
                if contains(s) && s.end - s.start < size {
                    size = s.end - s.start;
                    anchor = Anchor::Control(id);
                }
            }
            if contains(p.finish.span) {
                anchor = Anchor::Finish;
            }
            anchor
        }
    })
}

fn expression_span(unit: &c::ComposedUnit, id: ExprId) -> Result<Span, Error> {
    unit.expressions()
        .get(id.0)
        .map(|value| value.span)
        .ok_or(Error::Invalid(Invalid::Reference))
}

fn compensation_scopes(
    layout: &mut DeclLayout,
    unit: &c::ComposedUnit,
    scope: &DeclarationScope,
    value: &c::Compensation,
    original: usize,
    work: &mut Work,
) -> Result<(), Error> {
    let mut registration = value.forward.span;
    for capture in &value.registration_captures {
        work.visit()?;
        registration.end = registration.end.max(capture.span.end);
    }
    let mut activation = Span {
        start: value.trigger.span.start,
        end: expression_span(unit, value.guard)?.end,
    };
    for capture in &value.activation_captures {
        work.visit()?;
        activation.end = activation.end.max(capture.span.end);
    }
    let phases = [
        (Anchor::Registration(original), registration),
        (Anchor::CompensationActivation(original), activation),
        (
            Anchor::Retry(original),
            Span {
                start: value.earlier.span.start,
                end: expression_span(unit, value.retry)?.end,
            },
        ),
        (
            Anchor::Recovery(original),
            Span {
                start: value.recovery.span.start,
                end: expression_span(unit, value.recover)?.end,
            },
        ),
    ];
    for (anchor, region) in phases {
        let mut phase = None;
        for (binder, bound) in scope.binders.iter().enumerate() {
            work.visit()?;
            if bound.anchor != anchor
                || !matches!(
                    bound.kind,
                    BinderKind::ForwardEffect
                        | BinderKind::CompensationTrigger
                        | BinderKind::EarlierAttempt
                        | BinderKind::LaterAttempt
                        | BinderKind::Recovery
                )
            {
                continue;
            }
            if let Some(phase) = phase {
                layout.binder_scopes.insert(binder, phase);
            } else {
                phase_scope(layout, unit, region, binder, work)?;
                phase = Some(
                    *layout
                        .binder_scopes
                        .get(&binder)
                        .ok_or(Error::Invalid(Invalid::Scope))?,
                );
            }
        }
    }
    Ok(())
}

fn phase_scope(
    layout: &mut DeclLayout,
    unit: &c::ComposedUnit,
    region: Span,
    binder: usize,
    work: &mut Work,
) -> Result<(), Error> {
    work.charge(Dimension::Entries, 1)?;
    let i = index(layout.scopes.len())?;
    layout.scopes.push(w::Scope {
        parent: w::Nullable(Some(layout.handle(0))),
        locus: layout.locus(region)?,
    });
    layout.binder_scopes.insert(binder, i);
    for id in &layout.values {
        work.visit()?;
        let span = unit.expressions()[id.0].span;
        if region.start <= span.start && span.end <= region.end {
            layout.value_scopes.insert(id.0, i);
        }
    }
    Ok(())
}
