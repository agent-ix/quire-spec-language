// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-042: finite typed wire graphs, without native parsing or runtime unfolding.

mod control;

use std::collections::BTreeSet;

use super::value_graph::ValueGraph;
use super::{intake, wire::*, work::Work, Dimension, Error, Invalid, ProtocolNumber, Unsupported};

type Result<T = ()> = std::result::Result<T, Error>;

pub(super) fn integer(value: &Integer, work: &mut Work) -> Result<i64> {
    numeric_bytes(&value.0, work)?;
    match value.checked()? {
        ProtocolNumber::Integer(value) => Ok(value.value()),
        ProtocolNumber::Rational(_) => Err(Error::Invalid(Invalid::WrongNumericKind)),
    }
}

fn numeric_bytes(value: &super::NumberWire, work: &mut Work) -> Result {
    work.visit()?;
    match value {
        super::NumberWire::Integer { decimal } => work.bytes(decimal.len()),
        super::NumberWire::Rational {
            numerator,
            denominator,
        } => work.bytes(numerator.len().saturating_add(denominator.len())),
    }
}

fn interval(value: &Interval, work: &mut Work) -> Result {
    let lower = integer(&value.lower, work)?;
    let upper = integer(&value.upper, work)?;
    if lower < 0 || upper < lower {
        Err(Error::Invalid(Invalid::NumericDomain))
    } else {
        Ok(())
    }
}

fn nonnegative(value: &Integer, work: &mut Work) -> Result<i64> {
    let value = integer(value, work)?;
    if value < 0 {
        Err(Error::Invalid(Invalid::NumericDomain))
    } else {
        Ok(value)
    }
}

/// Validate raw exact numeric fields before any Serde encoding can turn an error
/// into human-readable serializer text. Native bounds retain their tagged kind.
pub(super) fn numbers(package: &Package, work: &mut Work) -> Result {
    for value in &package.types {
        work.visit()?;
        match value {
            Type::Scalar { representation, .. } => match representation {
                Representation::Integer { minimum, maximum } => {
                    if integer(minimum, work)? > integer(maximum, work)? {
                        return Err(Error::Invalid(Invalid::NumericDomain));
                    }
                }
                Representation::Rational {
                    numerator_minimum,
                    numerator_maximum,
                    maximum_denominator,
                } => {
                    if integer(numerator_minimum, work)? > integer(numerator_maximum, work)?
                        || integer(maximum_denominator, work)? <= 0
                    {
                        return Err(Error::Invalid(Invalid::NumericDomain));
                    }
                }
                Representation::Text { maximum_scalars } => {
                    nonnegative(maximum_scalars, work)?;
                }
            },
            Type::Sequence { maximum, .. } => {
                nonnegative(maximum, work)?;
            }
            Type::Boolean {}
            | Type::Enum { .. }
            | Type::Record { .. }
            | Type::Object { .. }
            | Type::Reference { .. }
            | Type::Option { .. } => {}
        }
    }
    for declaration in &package.declarations {
        work.locus = Some(declaration.locus.clone());
        for value in &declaration.values {
            work.visit()?;
            if let ValueOperation::Number { value } = &value.operation {
                numeric_bytes(&value.0, work)?;
                value.checked()?;
            }
        }
        for value in &declaration.temporal {
            work.visit()?;
            match &value.operation {
                TemporalOperation::Unary {
                    interval: value, ..
                }
                | TemporalOperation::Binary {
                    interval: value, ..
                } => {
                    if let Some(value) = &value.0 {
                        interval(value, work)?;
                    }
                }
                TemporalOperation::Constant { .. }
                | TemporalOperation::Holds { .. }
                | TemporalOperation::Group { .. } => {}
            }
        }
        if let Body::Protocol {
            channels,
            controls,
            compensations,
            causal_edges,
            ..
        } = &declaration.body
        {
            for channel in channels {
                interval(&channel.delivery, work)?;
            }
            for control in controls {
                work.visit()?;
                match &control.operation {
                    ControlOperation::Repeat { maximum, .. } => {
                        nonnegative(maximum, work)?;
                    }
                    ControlOperation::Await { within, .. } => interval(within, work)?,
                    ControlOperation::Sequence { .. }
                    | ControlOperation::Choice { .. }
                    | ControlOperation::Parallel { .. }
                    | ControlOperation::Event { .. }
                    | ControlOperation::Check { .. }
                    | ControlOperation::Commit { .. } => {}
                }
            }
            for compensation in compensations {
                interval(&compensation.within, work)?;
                if integer(&compensation.maximum_attempts, work)? <= 0 {
                    return Err(Error::Invalid(Invalid::NumericDomain));
                }
            }
            for edge in causal_edges {
                if let Some(maximum) = &edge.maximum.0 {
                    nonnegative(maximum, work)?;
                }
            }
        }
    }
    work.locus = None;
    Ok(())
}

#[derive(Clone, Copy)]
enum Local {
    Value,
    Binder,
    Anchor,
    Scope,
    Temporal,
    Control,
    Role,
    Channel,
    Compensation,
}

pub(super) struct Graph<'a, 'w> {
    package: &'a Package,
    work: &'w mut Work,
    type_seen: Vec<bool>,
    next_type: usize,
    dependencies: Vec<Vec<usize>>,
}

impl<'a> Graph<'a, '_> {
    fn local(&mut self, owner: usize, handle: &Handle, kind: Local) -> Result<usize> {
        self.work.visit()?;
        if handle.declaration as usize != owner {
            return Err(Error::Invalid(Invalid::Owner));
        }
        let declaration = &self.package.declarations[owner];
        let count = match (kind, &declaration.body) {
            (Local::Value, _) => declaration.values.len(),
            (Local::Binder, _) => declaration.binders.len(),
            (Local::Anchor, _) => declaration.anchors.len(),
            (Local::Scope, _) => declaration.scopes.len(),
            (Local::Temporal, _) => declaration.temporal.len(),
            (Local::Control, Body::Protocol { controls, .. }) => controls.len(),
            (Local::Role, Body::Protocol { roles, .. }) => roles.len(),
            (Local::Channel, Body::Protocol { channels, .. }) => channels.len(),
            (Local::Compensation, Body::Protocol { compensations, .. }) => compensations.len(),
            (
                Local::Control | Local::Role | Local::Channel | Local::Compensation,
                Body::Predicate { .. } | Body::State { .. } | Body::Temporal { .. },
            ) => return Err(Error::Invalid(Invalid::Owner)),
        };
        if handle.index as usize >= count {
            return Err(Error::Invalid(Invalid::Reference));
        }
        Ok(handle.index as usize)
    }

    fn ty(&mut self, index: u32) -> Result<&'a Type> {
        self.work.visit()?;
        self.package
            .types
            .get(index as usize)
            .ok_or(Error::Invalid(Invalid::Reference))
    }

    fn first_type(&mut self, index: u32, depth: usize) -> Result {
        self.work.charge(Dimension::Depth, depth)?;
        let ty = self.ty(index)?;
        if !self.type_seen[index as usize] {
            if index as usize != self.next_type {
                return Err(Error::Invalid(Invalid::Order));
            }
            self.type_seen[index as usize] = true;
            self.next_type += 1;
            match ty {
                Type::Option { value } => self.first_type(*value, depth + 1)?,
                Type::Sequence { element, .. } => self.first_type(*element, depth + 1)?,
                Type::Boolean {}
                | Type::Scalar { .. }
                | Type::Enum { .. }
                | Type::Record { .. }
                | Type::Object { .. }
                | Type::Reference { .. } => {}
            }
        }
        Ok(())
    }

    fn export(&mut self, value: &ExportRef, kinds: &[ExportKind]) -> Result<&'a Export> {
        self.work.visit()?;
        let export = self
            .package
            .models
            .get(value.model as usize)
            .and_then(|model| model.exports.get(value.export as usize))
            .ok_or(Error::Invalid(Invalid::Reference))?;
        if !kinds.is_empty() && !kinds.contains(&export.kind) {
            return Err(Error::Invalid(Invalid::Type));
        }
        Ok(export)
    }

    fn locus(&mut self, owner: usize, locus: &Locus) -> Result {
        self.work.visit()?;
        self.work.locus = Some(locus.clone());
        let declaration = &self.package.declarations[owner];
        if !contains(&declaration.locus, locus) {
            return Err(Error::Invalid(Invalid::Locus));
        }
        let source = self
            .package
            .sources
            .get(locus.source as usize)
            .ok_or(Error::Invalid(Invalid::Locus))?;
        intake::span(&source.text, &locus.span)
    }

    fn profile(&mut self, index: u32, family: Family) -> Result {
        self.work.visit()?;
        let definition = self
            .package
            .definitions
            .get(index as usize)
            .ok_or(Error::Invalid(Invalid::Reference))?;
        let name = definition.identity.as_str();
        let state = matches!(
            name,
            "quire.state.core/v1" | "quire.state.queries/v1" | "quire.state.graph/v1"
        );
        let temporal = matches!(
            name,
            "quire.temporal.event-position.false-extension/v1"
                | "quire.temporal.fixed-sample.false-extension/v1"
                | "quire.temporal.timestamped-event.finite-window/v1"
        );
        let valid = match family {
            Family::Value => state || temporal || name == "quire.protocol.finite-global/v1",
            Family::State => state,
            Family::Predicate => matches!(name, "quire.state.queries/v1" | "quire.state.graph/v1"),
            Family::Temporal => temporal,
            Family::Protocol => name == "quire.protocol.finite-global/v1",
        };
        if valid {
            Ok(())
        } else {
            Err(Error::Invalid(Invalid::Profile))
        }
    }

    fn binding(
        &mut self,
        owner: usize,
        index: u32,
        kinds: &[BindingKind],
    ) -> Result<&'a BindingRequirement> {
        self.work.visit()?;
        let value = self.package.declarations[owner]
            .bindings
            .get(index as usize)
            .ok_or(Error::Invalid(Invalid::Reference))?;
        if !kinds.is_empty() && !kinds.contains(&value.kind) {
            return Err(Error::Invalid(Invalid::Binding));
        }
        Ok(value)
    }

    fn value(&mut self, owner: usize, value: &Handle) -> Result<&'a Value> {
        let index = self.local(owner, value, Local::Value)?;
        Ok(&self.package.declarations[owner].values[index])
    }

    fn boolean(&mut self, owner: usize, value: &Handle) -> Result {
        let value = self.value(owner, value)?;
        if matches!(self.ty(value.value_type)?, Type::Boolean {}) {
            Ok(())
        } else {
            Err(Error::Invalid(Invalid::Type))
        }
    }

    fn value_type(&mut self, owner: usize, value: &Handle, expected: u32) -> Result {
        let actual = self.value(owner, value)?.value_type;
        self.ty(expected)?;
        if actual == expected {
            Ok(())
        } else {
            Err(Error::Invalid(Invalid::Type))
        }
    }

    fn binder(
        &mut self,
        owner: usize,
        binder: &Handle,
        kinds: &[BinderKind],
    ) -> Result<&'a Binder> {
        let index = self.local(owner, binder, Local::Binder)?;
        let binder = &self.package.declarations[owner].binders[index];
        if !kinds.is_empty() && !kinds.contains(&binder.kind) {
            return Err(Error::Invalid(Invalid::Binding));
        }
        Ok(binder)
    }

    fn activation(&mut self, owner: usize, value: &Activation) -> Result {
        let anchor = match value {
            Activation::Origin { anchor } => anchor,
            Activation::Each {
                trigger,
                guard,
                anchor,
            } => {
                self.binder(owner, trigger, &[BinderKind::Trigger])?;
                if let Some(value) = &guard.0 {
                    self.boolean(owner, value)?;
                }
                anchor
            }
        };
        let index = self.local(owner, anchor, Local::Anchor)?;
        if self.package.declarations[owner].anchors[index].kind != AnchorKind::Activation {
            return Err(Error::Invalid(Invalid::Binding));
        }
        Ok(())
    }

    fn captures(&mut self, owner: usize, values: &[Handle]) -> Result {
        self.work.charge(Dimension::Entries, values.len())?;
        let mut seen = BTreeSet::new();
        for value in values {
            self.binder(owner, value, &[BinderKind::Capture])?;
            if !seen.insert(value.index) {
                return Err(Error::Invalid(Invalid::Duplicate));
            }
        }
        Ok(())
    }

    fn dependency(&mut self, owner: usize, target: u32) -> Result {
        self.work.visit()?;
        if target as usize >= self.package.declarations.len() {
            return Err(Error::Invalid(Invalid::Reference));
        }
        self.work.charge(Dimension::Entries, 1)?;
        self.dependencies[owner].push(target as usize);
        Ok(())
    }
}

#[derive(Clone, Copy)]
enum Family {
    Value,
    Predicate,
    State,
    Temporal,
    Protocol,
}

fn contains(outer: &Locus, inner: &Locus) -> bool {
    outer.source == inner.source
        && outer.span.start <= inner.span.start
        && inner.span.end <= outer.span.end
}

fn ordered<'a>(values: impl IntoIterator<Item = (&'a Locus, u32)>, work: &mut Work) -> Result {
    let mut previous = None;
    for (locus, original) in values {
        work.visit()?;
        let key = (locus.source, locus.span.start, locus.span.end, original);
        if previous.is_some_and(|previous| previous >= key) {
            return Err(Error::Invalid(Invalid::Order));
        }
        previous = Some(key);
    }
    Ok(())
}

fn edges(count: usize, work: &mut Work) -> Result<Vec<Vec<usize>>> {
    work.charge(Dimension::Entries, count)?;
    Ok(vec![Vec::new(); count])
}

fn edge(graph: &mut [Vec<usize>], from: usize, to: usize, work: &mut Work) -> Result {
    work.charge(Dimension::Entries, 1)?;
    graph[from].push(to);
    Ok(())
}

/// Iterative tri-color traversal, charging each inspected edge and stack entry.
pub(super) fn acyclic(graph: &[Vec<usize>], work: &mut Work) -> Result {
    work.charge(Dimension::Entries, graph.len())?;
    let mut color = vec![0u8; graph.len()];
    for root in 0..graph.len() {
        work.visit()?;
        if color[root] != 0 {
            continue;
        }
        work.charge(Dimension::Entries, 1)?;
        let mut stack = vec![(root, 0)];
        color[root] = 1;
        while let Some((node, next)) = stack.last_mut() {
            if *next == graph[*node].len() {
                color[*node] = 2;
                stack.pop();
                continue;
            }
            work.visit()?;
            let child = graph[*node][*next];
            *next += 1;
            if child >= graph.len() {
                return Err(Error::Invalid(Invalid::Reference));
            }
            match color[child] {
                1 => return Err(Error::Invalid(Invalid::Cycle)),
                2 => {}
                _ => {
                    work.charge(Dimension::Depth, stack.len() + 1)?;
                    work.charge(Dimension::Entries, 1)?;
                    color[child] = 1;
                    stack.push((child, 0));
                }
            }
        }
    }
    Ok(())
}

impl<'a> Graph<'a, '_> {
    fn locals(&mut self, owner: usize) -> Result {
        let declaration = &self.package.declarations[owner];
        for text in [
            &declaration.name,
            &declaration.requirement.package,
            &declaration.requirement.identity,
            &declaration.clause,
        ] {
            intake::name(text)?;
        }
        intake::revision(&declaration.requirement.revision)?;
        match &declaration.execution {
            Execution::Initialization { name } | Execution::Handler { name } => intake::name(name)?,
            Execution::Pre { operation } | Execution::Post { operation } => {
                self.export(operation, &[ExportKind::Operation])?;
            }
        }
        ordered(
            declaration
                .scopes
                .iter()
                .enumerate()
                .map(|(i, value)| (&value.locus, i as u32)),
            self.work,
        )?;
        ordered(
            declaration
                .anchors
                .iter()
                .enumerate()
                .map(|(i, value)| (&value.locus, i as u32)),
            self.work,
        )?;
        ordered(
            declaration
                .binders
                .iter()
                .enumerate()
                .map(|(i, value)| (&value.locus, i as u32)),
            self.work,
        )?;
        ordered(
            declaration
                .values
                .iter()
                .map(|value| (&value.locus, value.original_expression)),
            self.work,
        )?;
        ordered(
            declaration
                .temporal
                .iter()
                .map(|value| (&value.locus, value.original_node)),
            self.work,
        )?;
        let mut scopes = edges(declaration.scopes.len(), self.work)?;
        for (index, scope) in declaration.scopes.iter().enumerate() {
            self.locus(owner, &scope.locus)?;
            if let Some(parent) = &scope.parent.0 {
                let parent = self.local(owner, parent, Local::Scope)?;
                if !contains(&declaration.scopes[parent].locus, &scope.locus) {
                    return Err(Error::Invalid(Invalid::Scope));
                }
                edge(&mut scopes, index, parent, self.work)?;
            }
        }
        acyclic(&scopes, self.work)?;
        for anchor in &declaration.anchors {
            self.locus(owner, &anchor.locus)?;
            let kind = match anchor.kind {
                AnchorKind::Fifo => Some(Local::Channel),
                AnchorKind::Registration
                | AnchorKind::CompensationActivation
                | AnchorKind::Retry
                | AnchorKind::Recovery => Some(Local::Compensation),
                AnchorKind::Control => Some(Local::Control),
                AnchorKind::Predicate
                | AnchorKind::Current
                | AnchorKind::InvocationInput
                | AnchorKind::InvocationPre
                | AnchorKind::InvocationPost
                | AnchorKind::Activation
                | AnchorKind::TemporalInstant
                | AnchorKind::ProtocolInstant
                | AnchorKind::Finish => None,
            };
            match (kind, &anchor.owner.0) {
                (Some(kind), Some(handle)) => {
                    self.local(owner, handle, kind)?;
                }
                (None, None) => {}
                (Some(_), None) | (None, Some(_)) => return Err(Error::Invalid(Invalid::Owner)),
            }
            if let Some(binding) = anchor.binding.0 {
                self.binding(owner, binding, &[])?;
            }
        }
        self.work
            .charge(Dimension::Entries, declaration.binders.len())?;
        let mut names = BTreeSet::new();
        for binder in &declaration.binders {
            intake::name(&binder.name)?;
            self.work.bytes(binder.name.len())?;
            if !names.insert((binder.scope.index, binder.name.as_str())) {
                return Err(Error::Invalid(Invalid::Duplicate));
            }
            self.locus(owner, &binder.locus)?;
            self.first_type(binder.value_type, 1)?;
            let scope = self.local(owner, &binder.scope, Local::Scope)?;
            if !contains(&declaration.scopes[scope].locus, &binder.locus) {
                return Err(Error::Invalid(Invalid::Scope));
            }
            self.local(owner, &binder.anchor, Local::Anchor)?;
            if let Some(value) = &binder.initializer.0 {
                self.value_type(owner, value, binder.value_type)?;
                if !matches!(binder.kind, BinderKind::Let | BinderKind::Capture) {
                    return Err(Error::Invalid(Invalid::Binding));
                }
            } else if binder.kind == BinderKind::Let {
                return Err(Error::Invalid(Invalid::Binding));
            }
        }
        let mut bindings = edges(declaration.bindings.len(), self.work)?;
        self.work
            .charge(Dimension::Entries, declaration.bindings.len())?;
        let mut names = BTreeSet::new();
        for (index, binding) in declaration.bindings.iter().enumerate() {
            self.locus(owner, &binding.locus)?;
            intake::name(&binding.name)?;
            self.work.bytes(binding.name.len())?;
            if !names.insert(binding.name.as_str()) {
                return Err(Error::Invalid(Invalid::Duplicate));
            }
            intake::reference(&binding.authority)?;
            self.local(owner, &binding.anchor, Local::Anchor)?;
            self.local(owner, &binding.scope, Local::Scope)?;
            if let Some(ty) = binding.value_type.0 {
                self.first_type(ty, 1)?;
            }
            if let Some(model) = &binding.model.0 {
                self.export(model, &[])?;
            }
            if binding.kind == BindingKind::CompensationEffect {
                self.compensation_effect_identity(owner, index, binding)?;
            } else if binding.kind == BindingKind::Relationship {
                if binding.value_type.0.is_some()
                    || binding.model.0.is_none()
                    || binding.relation.0.is_none()
                {
                    return Err(Error::Invalid(Invalid::Binding));
                }
            } else if (binding.value_type.0.is_none() || binding.model.0.is_none())
                && !matches!(
                    binding.kind,
                    BindingKind::Clock
                        | BindingKind::Observation
                        | BindingKind::Progress
                        | BindingKind::Closure
                )
            {
                return Err(Error::Invalid(Invalid::Binding));
            }
            for dependency in [Some(binding.contract), binding.relation.0]
                .into_iter()
                .flatten()
            {
                self.work.visit()?;
                if dependency as usize >= self.package.dependencies.len() {
                    return Err(Error::Invalid(Invalid::Reference));
                }
            }
            // Authority is an independently retained static dependency, never a
            // freely asserted provider label detached from offered bytes.
            let mut found = false;
            for dependency in &self.package.dependencies {
                self.work.visit()?;
                if intake::key(&dependency.artifact) == intake::key(&binding.authority) {
                    intake::same_reference(&dependency.artifact, &binding.authority, self.work)?;
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(Error::Invalid(Invalid::Binding));
            }
            match &binding.subject {
                Subject::Declaration { declaration } => {
                    if *declaration as usize != owner {
                        return Err(Error::Invalid(Invalid::Owner));
                    }
                }
                Subject::Role { role } => {
                    self.local(owner, role, Local::Role)?;
                }
                Subject::Channel { channel } => {
                    self.local(owner, channel, Local::Channel)?;
                }
                Subject::Control { control } => {
                    self.local(owner, control, Local::Control)?;
                }
                Subject::Compensation { compensation } => {
                    self.local(owner, compensation, Local::Compensation)?;
                }
            }
            intake::sorted_indices(&binding.requires, self.work)?;
            for required in &binding.requires {
                self.binding(owner, *required, &[])?;
                edge(&mut bindings, index, *required as usize, self.work)?;
            }
        }
        acyclic(&bindings, self.work)
    }

    fn sequence(&mut self, ty: u32) -> Result<(u32, i64)> {
        match self.ty(ty)? {
            Type::Sequence { element, maximum } => Ok((*element, integer(maximum, self.work)?)),
            _ => Err(Error::Invalid(Invalid::Type)),
        }
    }

    fn numeric(&mut self, ty: u32, dimensionless: bool, rational: bool) -> Result {
        let Type::Scalar {
            unit,
            representation,
            ..
        } = self.ty(ty)?
        else {
            return Err(Error::Invalid(Invalid::Type));
        };
        if dimensionless && unit.0.is_some() {
            return Err(Error::Invalid(Invalid::Type));
        }
        match representation {
            Representation::Integer { .. } if !rational => Ok(()),
            Representation::Rational { .. } => Ok(()),
            Representation::Integer { .. } | Representation::Text { .. } => {
                Err(Error::Invalid(Invalid::Type))
            }
        }
    }

    fn equality(&mut self, ty: u32) -> Result {
        match self.ty(ty)? {
            Type::Boolean {}
            | Type::Scalar { .. }
            | Type::Enum { .. }
            | Type::Object { .. }
            | Type::Reference { .. } => Ok(()),
            Type::Record { .. } | Type::Option { .. } | Type::Sequence { .. } => {
                Err(Error::Invalid(Invalid::Type))
            }
        }
    }

    fn count_type(&mut self, ty: u32, maximum: i64) -> Result {
        match self.ty(ty)? {
            Type::Scalar {
                unit,
                representation:
                    Representation::Integer {
                        minimum,
                        maximum: bound,
                    },
                ..
            } if unit.0.is_none()
                && integer(minimum, self.work)? <= 0
                && integer(bound, self.work)? >= maximum =>
            {
                Ok(())
            }
            _ => Err(Error::Invalid(Invalid::Type)),
        }
    }

    fn values(&mut self, owner: usize) -> Result<ValueGraph<'a>> {
        let declaration = &self.package.declarations[owner];
        self.work
            .charge(Dimension::Entries, declaration.values.len())?;
        let mut original = BTreeSet::new();
        for value in &declaration.values {
            self.locus(owner, &value.locus)?;
            if value.original_expression > 1_048_576 {
                return Err(Error::Invalid(Invalid::StructuralInteger));
            }
            if !original.insert(value.original_expression) {
                return Err(Error::Invalid(Invalid::Duplicate));
            }
            if let Some(locus) = &value.operator_locus.0 {
                self.locus(owner, locus)?;
                if !contains(&value.locus, locus) {
                    return Err(Error::Invalid(Invalid::Locus));
                }
            }
            self.first_type(value.value_type, 1)?;
            self.profile(value.profile, Family::Value)?;
            let scope = self.local(owner, &value.scope, Local::Scope)?;
            if !contains(&declaration.scopes[scope].locus, &value.locus) {
                return Err(Error::Invalid(Invalid::Scope));
            }
            self.local(owner, &value.anchor, Local::Anchor)?;
            match &value.origin {
                Origin::Independent {} => {}
                Origin::Anchor { anchor } => {
                    self.local(owner, anchor, Local::Anchor)?;
                }
                Origin::Selected { value } => {
                    self.local(owner, value, Local::Value)?;
                }
            }
            match &value.operation {
                ValueOperation::Boolean { .. } => {
                    if !matches!(self.ty(value.value_type)?, Type::Boolean {}) {
                        return Err(Error::Invalid(Invalid::Type));
                    }
                }
                ValueOperation::Number { value: number } => {
                    numeric_bytes(&number.0, self.work)?;
                    let number = number.checked()?;
                    let Type::Scalar { representation, .. } = self.ty(value.value_type)? else {
                        return Err(Error::Invalid(Invalid::Type));
                    };
                    let valid = match (number, representation) {
                        (
                            ProtocolNumber::Integer(number),
                            Representation::Integer { minimum, maximum },
                        ) => {
                            number.value() >= integer(minimum, self.work)?
                                && number.value() <= integer(maximum, self.work)?
                        }
                        (
                            ProtocolNumber::Rational(number),
                            Representation::Rational {
                                numerator_minimum,
                                numerator_maximum,
                                maximum_denominator,
                            },
                        ) => {
                            number.numerator() >= integer(numerator_minimum, self.work)?
                                && number.numerator() <= integer(numerator_maximum, self.work)?
                                && number.denominator() <= integer(maximum_denominator, self.work)?
                        }
                        _ => false,
                    };
                    if !valid {
                        return Err(Error::Invalid(Invalid::NumericDomain));
                    }
                }
                ValueOperation::Text { value: text } => {
                    self.work.bytes(text.len())?;
                    let Type::Scalar {
                        representation: Representation::Text { maximum_scalars },
                        ..
                    } = self.ty(value.value_type)?
                    else {
                        return Err(Error::Invalid(Invalid::Type));
                    };
                    if text.chars().count() as i128
                        > i128::from(integer(maximum_scalars, self.work)?)
                    {
                        return Err(Error::Invalid(Invalid::NumericDomain));
                    }
                }
                ValueOperation::Enum { variant } => {
                    self.export(variant, &[ExportKind::Variant])?;
                }
                ValueOperation::Read { binder } => {
                    let binder = self.binder(owner, binder, &[])?;
                    if binder.value_type != value.value_type {
                        return Err(Error::Invalid(Invalid::Type));
                    }
                    let mut scope = Some(value.scope.index as usize);
                    let mut visible = false;
                    while let Some(index) = scope {
                        self.work.visit()?;
                        if index == binder.scope.index as usize {
                            visible = true;
                            break;
                        }
                        scope = declaration.scopes[index]
                            .parent
                            .0
                            .as_ref()
                            .map(|value| value.index as usize);
                    }
                    if !visible {
                        return Err(Error::Invalid(Invalid::Scope));
                    }
                    if let Some(initializer) = &binder.initializer.0 {
                        let initialized = self.value(owner, initializer)?;
                        if initialized.locus.span.end > value.locus.span.start {
                            return Err(Error::Invalid(Invalid::Scope));
                        }
                    }
                }
                ValueOperation::Group { value: inner } => {
                    self.value_type(owner, inner, value.value_type)?;
                }
                ValueOperation::Field { field, .. } => {
                    self.export(field, &[ExportKind::Field])?;
                }
                ValueOperation::Unary {
                    operator,
                    value: inner,
                } => {
                    let operand = self.value(owner, inner)?.value_type;
                    match operator {
                        Unary::Not => {
                            self.boolean(owner, inner)?;
                            if !matches!(self.ty(value.value_type)?, Type::Boolean {}) {
                                return Err(Error::Invalid(Invalid::Type));
                            }
                        }
                        Unary::Negate => {
                            self.numeric(value.value_type, false, false)?;
                            self.value_type(owner, inner, value.value_type)?;
                        }
                        Unary::Present => {
                            if !matches!(self.ty(operand)?, Type::Option { .. })
                                || !matches!(self.ty(value.value_type)?, Type::Boolean {})
                            {
                                return Err(Error::Invalid(Invalid::Type));
                            }
                        }
                        Unary::Value => {
                            if !matches!(self.ty(operand)?, Type::Option { value: result } if *result == value.value_type)
                            {
                                return Err(Error::Invalid(Invalid::Type));
                            }
                        }
                        Unary::Deref => {
                            let Type::Reference { object, .. } = self.ty(operand)? else {
                                return Err(Error::Invalid(Invalid::Type));
                            };
                            if !matches!(self.ty(value.value_type)?, Type::Object { export } if export == object)
                            {
                                return Err(Error::Invalid(Invalid::Type));
                            }
                        }
                    }
                }
                ValueOperation::Binary {
                    operator,
                    left,
                    right,
                } => {
                    let ty = self.value(owner, left)?.value_type;
                    self.value_type(owner, right, ty)?;
                    match operator {
                        Binary::Implies | Binary::Or | Binary::And => {
                            self.boolean(owner, left)?;
                            if value.value_type != ty {
                                return Err(Error::Invalid(Invalid::Type));
                            }
                        }
                        Binary::Equal
                        | Binary::NotEqual
                        | Binary::Less
                        | Binary::LessEqual
                        | Binary::Greater
                        | Binary::GreaterEqual => {
                            if !matches!(self.ty(value.value_type)?, Type::Boolean {}) {
                                return Err(Error::Invalid(Invalid::Type));
                            }
                            if matches!(operator, Binary::Equal | Binary::NotEqual) {
                                self.equality(ty)?;
                            } else if !matches!(self.ty(ty)?, Type::Scalar { .. }) {
                                return Err(Error::Invalid(Invalid::Type));
                            }
                        }
                        Binary::Add
                        | Binary::Subtract
                        | Binary::Multiply
                        | Binary::RationalDivide => {
                            self.numeric(
                                ty,
                                matches!(operator, Binary::Multiply | Binary::RationalDivide),
                                *operator == Binary::RationalDivide,
                            )?;
                            if value.value_type != ty {
                                return Err(Error::Invalid(Invalid::Type));
                            }
                        }
                    }
                }
                ValueOperation::If {
                    condition,
                    then_value,
                    else_value,
                } => {
                    self.boolean(owner, condition)?;
                    self.value_type(owner, then_value, value.value_type)?;
                    self.value_type(owner, else_value, value.value_type)?;
                }
                ValueOperation::Let {
                    binder,
                    initializer,
                    body,
                } => {
                    let binder = self.binder(owner, binder, &[BinderKind::Let])?;
                    if binder.initializer.0.as_ref() != Some(initializer) {
                        return Err(Error::Invalid(Invalid::Binding));
                    }
                    self.value_type(owner, initializer, binder.value_type)?;
                    self.value_type(owner, body, value.value_type)?;
                }
                ValueOperation::Pre {
                    value: inner,
                    anchor,
                } => {
                    let anchor = self.local(owner, anchor, Local::Anchor)?;
                    if declaration.anchors[anchor].kind != AnchorKind::InvocationPre {
                        return Err(Error::Invalid(Invalid::Binding));
                    }
                    self.value_type(owner, inner, value.value_type)?;
                }
                ValueOperation::Call {
                    predicate,
                    arguments,
                } => {
                    self.dependency(owner, *predicate)?;
                    let target = &self.package.declarations[*predicate as usize];
                    let Body::Predicate {
                        parameters, result, ..
                    } = &target.body
                    else {
                        return Err(Error::Invalid(Invalid::Call));
                    };
                    if arguments.len() != parameters.len() || *result != value.value_type {
                        return Err(Error::Invalid(Invalid::Call));
                    }
                    for (argument, parameter) in arguments.iter().zip(parameters) {
                        let expected = self
                            .binder(*predicate as usize, parameter, &[BinderKind::Parameter])?
                            .value_type;
                        self.value_type(owner, argument, expected)?;
                    }
                }
                ValueOperation::Size { collection, result } => {
                    let collection = self.value(owner, collection)?.value_type;
                    let (_, maximum) = self.sequence(collection)?;
                    if *result != value.value_type {
                        return Err(Error::Invalid(Invalid::Type));
                    }
                    self.count_type(*result, maximum)?;
                }
                ValueOperation::Contains { collection, member } => {
                    let collection = self.value(owner, collection)?.value_type;
                    let (element, _) = self.sequence(collection)?;
                    self.value_type(owner, member, element)?;
                    self.equality(element)?;
                    if !matches!(self.ty(value.value_type)?, Type::Boolean {}) {
                        return Err(Error::Invalid(Invalid::Type));
                    }
                }
                ValueOperation::Query {
                    operator,
                    binder,
                    collection,
                    body,
                    result,
                } => {
                    let collection_type = self.value(owner, collection)?.value_type;
                    let (element, maximum) = self.sequence(collection_type)?;
                    let binder = self.binder(owner, binder, &[BinderKind::Query])?;
                    if binder.value_type != element || *result != value.value_type {
                        return Err(Error::Invalid(Invalid::Type));
                    }
                    match operator {
                        Query::ForAll | Query::Exists => {
                            self.boolean(owner, body)?;
                            if !matches!(self.ty(*result)?, Type::Boolean {}) {
                                return Err(Error::Invalid(Invalid::Type));
                            }
                        }
                        Query::Filter => {
                            self.boolean(owner, body)?;
                            if *result != collection_type {
                                return Err(Error::Invalid(Invalid::Type));
                            }
                        }
                        Query::Map => {
                            let (element, bound) = self.sequence(*result)?;
                            self.value_type(owner, body, element)?;
                            if bound != maximum {
                                return Err(Error::Invalid(Invalid::Type));
                            }
                        }
                        Query::Count => {
                            self.boolean(owner, body)?;
                            self.count_type(*result, maximum)?;
                        }
                        Query::Sum => {
                            let body = self.value(owner, body)?.value_type;
                            self.sum_type(body, *result)?;
                        }
                    }
                }
                ValueOperation::Parent { edge, universe, .. } => {
                    self.export(edge, &[ExportKind::Field, ExportKind::Relationship])?;
                    self.export(universe, &[ExportKind::Population])?;
                }
                ValueOperation::Reaches {
                    start,
                    target,
                    edge,
                    universe,
                } => {
                    let ty = self.value(owner, start)?.value_type;
                    self.value_type(owner, target, ty)?;
                    self.export(edge, &[ExportKind::Field, ExportKind::Relationship])?;
                    self.export(universe, &[ExportKind::Population])?;
                    if !matches!(self.ty(value.value_type)?, Type::Boolean {}) {
                        return Err(Error::Invalid(Invalid::Type));
                    }
                }
            }
        }
        ValueGraph::new(owner, declaration, self.work)
    }

    fn sum_type(&mut self, projection: u32, total: u32) -> Result {
        let (
            Type::Scalar {
                unit: a,
                representation: projection,
                ..
            },
            Type::Scalar {
                unit: b,
                representation: total,
                ..
            },
        ) = (self.ty(projection)?, self.ty(total)?)
        else {
            return Err(Error::Invalid(Invalid::Type));
        };
        if a != b {
            return Err(Error::Invalid(Invalid::Type));
        }
        let valid = match (projection, total) {
            (
                Representation::Integer {
                    minimum: a,
                    maximum: b,
                },
                Representation::Integer {
                    minimum: c,
                    maximum: d,
                },
            ) => {
                let (a, b, c, d) = (
                    integer(a, self.work)?,
                    integer(b, self.work)?,
                    integer(c, self.work)?,
                    integer(d, self.work)?,
                );
                c <= 0 && d >= 0 && c <= a && d >= b
            }
            (
                Representation::Rational {
                    numerator_minimum: a,
                    numerator_maximum: b,
                    maximum_denominator: c,
                },
                Representation::Rational {
                    numerator_minimum: d,
                    numerator_maximum: e,
                    maximum_denominator: f,
                },
            ) => {
                let (a, b, c, d, e, f) = (
                    integer(a, self.work)?,
                    integer(b, self.work)?,
                    integer(c, self.work)?,
                    integer(d, self.work)?,
                    integer(e, self.work)?,
                    integer(f, self.work)?,
                );
                d <= 0 && e >= 0 && d <= a && e >= b && f >= c
            }
            _ => false,
        };
        if valid {
            Ok(())
        } else {
            Err(Error::Invalid(Invalid::Type))
        }
    }

    fn temporal(&mut self, owner: usize) -> Result {
        let declaration = &self.package.declarations[owner];
        let mut graph = edges(declaration.temporal.len(), self.work)?;
        self.work
            .charge(Dimension::Entries, declaration.temporal.len())?;
        let mut seen = BTreeSet::new();
        for (index, node) in declaration.temporal.iter().enumerate() {
            self.locus(owner, &node.locus)?;
            if node.original_node > 1_048_576 {
                return Err(Error::Invalid(Invalid::StructuralInteger));
            }
            if !seen.insert(node.original_node) {
                return Err(Error::Invalid(Invalid::Duplicate));
            }
            let mut child = |this: &mut Self, value: &Handle| -> Result {
                let target = this.local(owner, value, Local::Temporal)?;
                edge(&mut graph, index, target, this.work)
            };
            match &node.operation {
                TemporalOperation::Constant { .. } => {}
                TemporalOperation::Holds { value } => self.boolean(owner, value)?,
                TemporalOperation::Group { value } => child(self, value)?,
                TemporalOperation::Unary {
                    operator,
                    interval,
                    value,
                } => {
                    if (*operator == TemporalUnary::Not) != interval.0.is_none() {
                        return Err(Error::Invalid(Invalid::Profile));
                    }
                    child(self, value)?;
                }
                TemporalOperation::Binary {
                    operator,
                    interval,
                    left,
                    right,
                } => {
                    let boolean = matches!(
                        operator,
                        TemporalBinary::Implies | TemporalBinary::Or | TemporalBinary::And
                    );
                    if boolean != interval.0.is_none() {
                        return Err(Error::Invalid(Invalid::Profile));
                    }
                    child(self, left)?;
                    child(self, right)?;
                }
            }
        }
        acyclic(&graph, self.work)
    }

    fn body(&mut self, owner: usize, values: &ValueGraph<'_>) -> Result {
        let declaration = &self.package.declarations[owner];
        match &declaration.body {
            Body::Predicate {
                parameters,
                result,
                root,
            } => {
                self.profile(declaration.profile, Family::Predicate)?;
                self.first_type(*result, 1)?;
                self.boolean(owner, root)?;
                if !matches!(self.ty(*result)?, Type::Boolean {}) {
                    return Err(Error::Invalid(Invalid::Type));
                }
                self.work.charge(Dimension::Entries, parameters.len())?;
                let mut seen = BTreeSet::new();
                for parameter in parameters {
                    self.binder(owner, parameter, &[BinderKind::Parameter])?;
                    if !seen.insert(parameter.index) {
                        return Err(Error::Invalid(Invalid::Duplicate));
                    }
                }
                for (index, binder) in declaration.binders.iter().enumerate() {
                    self.work.visit()?;
                    if binder.kind == BinderKind::Parameter && !seen.contains(&(index as u32)) {
                        return Err(Error::Invalid(Invalid::Binding));
                    }
                }
            }
            Body::State {
                clause_kind,
                context,
                operation,
                root,
            } => {
                self.profile(declaration.profile, Family::State)?;
                self.export(context, &[ExportKind::Record, ExportKind::Object])?;
                self.boolean(owner, root)?;
                match (clause_kind, &operation.0, &declaration.execution) {
                    (
                        ClauseKind::Invariant,
                        None,
                        Execution::Initialization { .. } | Execution::Handler { .. },
                    ) => {}
                    (ClauseKind::Pre, Some(selected), Execution::Pre { operation })
                    | (ClauseKind::Post, Some(selected), Execution::Post { operation })
                        if selected == operation =>
                    {
                        self.export(selected, &[ExportKind::Operation])?;
                    }
                    _ => return Err(Error::Invalid(Invalid::Profile)),
                }
            }
            Body::Temporal {
                input,
                clock,
                activation,
                captures,
                root,
            } => {
                self.profile(declaration.profile, Family::Temporal)?;
                self.binder(owner, input, &[BinderKind::Input])?;
                self.binding(owner, *clock, &[BindingKind::Clock])?;
                self.activation(owner, activation)?;
                self.captures(owner, captures)?;
                self.local(owner, root, Local::Temporal)?;
            }
            Body::Protocol { .. } => {
                self.profile(declaration.profile, Family::Protocol)?;
                self.protocol(owner, values)?;
            }
        }
        Ok(())
    }
}

#[derive(Eq, PartialEq, Ord, PartialOrd)]
enum RepresentationKey {
    Integer(i64, i64),
    Rational(i64, i64, i64),
    Text(i64),
}

#[derive(Eq, PartialEq, Ord, PartialOrd)]
enum TypeKey<'a> {
    Boolean,
    Scalar(u32, u32, Option<&'a str>, RepresentationKey),
    Enum(u32, u32),
    Record(u32, u32),
    Object(u32, u32),
    Reference(u32, u32, u32, u32, u32, u32),
    Option(u32),
    Sequence(u32, i64),
}

fn types(package: &Package, work: &mut Work) -> Result {
    let mut graph = edges(package.types.len(), work)?;
    let mut seen = BTreeSet::new();
    for (index, ty) in package.types.iter().enumerate() {
        work.visit()?;
        let key = match ty {
            Type::Boolean {} => TypeKey::Boolean,
            Type::Scalar {
                export,
                unit,
                representation,
            } => {
                if let Some(unit) = &unit.0 {
                    intake::name(unit)?;
                    work.bytes(unit.len())?;
                }
                let representation = match representation {
                    Representation::Integer { minimum, maximum } => {
                        RepresentationKey::Integer(integer(minimum, work)?, integer(maximum, work)?)
                    }
                    Representation::Rational {
                        numerator_minimum,
                        numerator_maximum,
                        maximum_denominator,
                    } => RepresentationKey::Rational(
                        integer(numerator_minimum, work)?,
                        integer(numerator_maximum, work)?,
                        integer(maximum_denominator, work)?,
                    ),
                    Representation::Text { maximum_scalars } => {
                        RepresentationKey::Text(integer(maximum_scalars, work)?)
                    }
                };
                TypeKey::Scalar(
                    export.model,
                    export.export,
                    unit.0.as_deref(),
                    representation,
                )
            }
            Type::Enum { export } => TypeKey::Enum(export.model, export.export),
            Type::Record { export } => TypeKey::Record(export.model, export.export),
            Type::Object { export } => TypeKey::Object(export.model, export.export),
            Type::Reference {
                export,
                object,
                universe,
            } => TypeKey::Reference(
                export.model,
                export.export,
                object.model,
                object.export,
                universe.model,
                universe.export,
            ),
            Type::Option { value } => {
                edge(&mut graph, index, *value as usize, work)?;
                TypeKey::Option(*value)
            }
            Type::Sequence { element, maximum } => {
                edge(&mut graph, index, *element as usize, work)?;
                TypeKey::Sequence(*element, integer(maximum, work)?)
            }
        };
        work.charge(Dimension::Entries, 1)?;
        if !seen.insert(key) {
            return Err(Error::Invalid(Invalid::Duplicate));
        }
    }
    acyclic(&graph, work)
}

fn feature(set: &mut BTreeSet<&'static str>, name: &'static str, work: &mut Work) -> Result {
    work.visit()?;
    if !set.contains(name) {
        work.charge(Dimension::Entries, 1)?;
        set.insert(name);
    }
    Ok(())
}

fn features(package: &Package, work: &mut Work) -> Result {
    let mut families = BTreeSet::new();
    let mut required = BTreeSet::new();
    feature(&mut required, "quire.protocol.bindings/1", work)?;
    feature(&mut required, "quire.protocol.numeric/1", work)?;
    for declaration in &package.declarations {
        work.visit()?;
        let family = match declaration.body {
            Body::Predicate { .. } => "declaration.predicate",
            Body::State { .. } => "family.state",
            Body::Temporal { .. } => "family.temporal",
            Body::Protocol { .. } => "family.protocol",
        };
        feature(&mut families, family, work)?;
        if !declaration.values.is_empty() {
            feature(&mut required, "quire.protocol.values/1", work)?;
        }
        if !declaration.temporal.is_empty() {
            feature(&mut required, "quire.protocol.temporal/1", work)?;
        }
        if let Body::Protocol { controls, .. } = &declaration.body {
            if !controls.is_empty() {
                feature(&mut required, "quire.protocol.control/1", work)?;
            }
        }
    }
    if !families.contains("family.protocol") {
        return Err(Error::Invalid(Invalid::Feature));
    }
    for value in &package.features.declarations {
        work.visit()?;
        if !matches!(
            value.as_str(),
            "declaration.predicate" | "family.state" | "family.temporal" | "family.protocol"
        ) {
            return Err(Error::Unsupported(Unsupported::Feature));
        }
    }
    for value in &package.features.required {
        work.visit()?;
        if !matches!(
            value.as_str(),
            "quire.protocol.bindings/1"
                | "quire.protocol.numeric/1"
                | "quire.protocol.values/1"
                | "quire.protocol.temporal/1"
                | "quire.protocol.control/1"
        ) {
            return Err(Error::Unsupported(Unsupported::Feature));
        }
    }
    if !package.features.optional.is_empty() {
        return Err(Error::Unsupported(Unsupported::Feature));
    }
    if !package
        .features
        .declarations
        .iter()
        .map(String::as_str)
        .eq(families)
        || !package
            .features
            .required
            .iter()
            .map(String::as_str)
            .eq(required)
    {
        return Err(Error::Invalid(Invalid::Feature));
    }
    Ok(())
}

pub(super) fn package(package: &Package, work: &mut Work) -> Result {
    numbers(package, work)?;
    types(package, work)?;
    features(package, work)?;
    let mut dependency_graph = edges(package.dependencies.len(), work)?;
    for (index, dependency) in package.dependencies.iter().enumerate() {
        for required in &dependency.requires {
            work.visit()?;
            edge(&mut dependency_graph, index, *required as usize, work)?;
        }
    }
    acyclic(&dependency_graph, work)?;
    work.charge(Dimension::Entries, package.types.len())?;
    let dependencies = edges(package.declarations.len(), work)?;
    let mut graph = Graph {
        package,
        work,
        type_seen: vec![false; package.types.len()],
        next_type: 0,
        dependencies,
    };
    for owner in 0..package.declarations.len() {
        graph.locals(owner)?;
        let values = graph.values(owner)?;
        graph.temporal(owner)?;
        graph.body(owner, &values)?;
        let offered = &package.declarations[owner].requires;
        intake::sorted_indices(offered, graph.work)?;
        // Traversal retains duplicate call occurrences; the explicit dependency
        // set records each target once, independently of occurrence order.
        let mut found = BTreeSet::new();
        for target in &graph.dependencies[owner] {
            graph.work.visit()?;
            if !found.contains(target) {
                graph.work.charge(Dimension::Entries, 1)?;
                found.insert(*target);
            }
        }
        if !offered.iter().map(|value| *value as usize).eq(found) {
            return Err(Error::Invalid(Invalid::Dependency));
        }
    }
    if graph.type_seen.contains(&false) {
        return Err(Error::Invalid(Invalid::Inventory));
    }
    acyclic(&graph.dependencies, graph.work)?;
    graph.work.locus = None;
    Ok(())
}
