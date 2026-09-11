// SPDX-License-Identifier: AGPL-3.0-only
//! FR-036/040/042: concrete family prerequisites and original temporal syntax.
//! Closed decisions and received Boolean partitions have separate proof rules.

mod decisions;

use std::collections::{BTreeMap, BTreeSet};

use crate::checking::composed::proofs::{ProofDisposition, ProofReport};
use crate::checking::composed::DeclarationTypes;
use crate::linking::composed::definition_source::RegisteredDefinition as Definition;
use crate::linking::composed::definitions::UseKind;
use crate::linking::composed::scopes::{BinderKind, BinderType, DeclarationScope};
use crate::protocol_artifact::{wire as w, work::Work, Dimension, Error, Invalid, Unsupported};
use crate::syntax::composed::{self as c, ComposedUnit};
use crate::syntax::{BinaryOp, ExprId, ExprKind, UnaryOp};
use crate::Span;

use super::layout::DeclLayout;
use super::types::{index, integer};

pub(super) fn check(
    proofs: &ProofReport<'_, '_, '_>,
    sources: &[u32],
    work: &mut Work,
) -> Result<(), Error> {
    let binding = proofs.types().binding();
    let namespace = binding.namespace();
    let definitions = binding
        .definitions()
        .ok_or(Error::Invalid(Invalid::Definition))?;
    if !binding.complete()
        || !definitions.complete
        || definitions.edition_refusal.is_some()
        || proofs.exhaustion().is_some()
    {
        return Err(Error::Unsupported(Unsupported::FamilyProof));
    }
    work.charge(Dimension::Declarations, namespace.declarations().len())?;
    if proofs.declarations().len() != namespace.declarations().len() {
        return Err(Error::Unsupported(Unsupported::FamilyProof));
    }
    for proof in proofs.declarations() {
        work.visit()?;
        let source = *sources
            .get(proof.unit().index())
            .ok_or(Error::Invalid(Invalid::Owner))?;
        let syntax = namespace
            .syntax(proof.declaration())
            .ok_or(Error::Invalid(Invalid::Owner))?;
        locate(source, syntax.span, work)?;
        if proof.disposition() != ProofDisposition::Discharged || !proof.complete() {
            return Err(Error::Unsupported(Unsupported::FamilyProof));
        }
        let profiles = definitions
            .declarations
            .get(proof.declaration().index())
            .ok_or(Error::Invalid(Invalid::Definition))?;
        if !profiles.complete || profiles.uses.is_empty() {
            return Err(Error::Invalid(Invalid::Profile));
        }
        for selected in &profiles.uses {
            work.visit()?;
            locate(source, selected.alias.span, work)?;
            if selected.unit != proof.unit() || !selected.complete || selected.refusal.is_some() {
                return Err(Error::Invalid(Invalid::Profile));
            }
            let root = *selected
                .closure
                .first()
                .ok_or(Error::Invalid(Invalid::Profile))?;
            if !profile(root, selected.kind) {
                return Err(Error::Invalid(Invalid::Profile));
            }
        }
        let unit = namespace
            .unit(proof.unit())
            .ok_or(Error::Invalid(Invalid::Owner))?;
        match &syntax.kind {
            c::DeclarationKind::Predicate { .. } | c::DeclarationKind::State { .. } => {}
            c::DeclarationKind::Temporal { .. } => {
                let range = c::arena::owned_range(
                    unit.temporal_nodes(),
                    syntax.span,
                    |node| node.span,
                    || work.visit(),
                )?;
                for at in range {
                    work.visit()?;
                    let node = &unit.temporal_nodes()[at];
                    locate(source, node.span, work)?;
                    temporal_shape(&node.kind, work)?;
                }
            }
            c::DeclarationKind::Protocol(protocol) => {
                let typed = proofs
                    .types()
                    .declaration(proof.declaration())
                    .ok_or(Error::Invalid(Invalid::Type))?;
                let scope = binding
                    .scopes()
                    .and_then(|scopes| scopes.declaration(proof.declaration()))
                    .ok_or(Error::Invalid(Invalid::Binding))?;
                protocol_check(unit, typed, scope, syntax.span, source, protocol, work)?;
            }
        }
    }
    work.locus = None;
    Ok(())
}

fn profile(root: Definition, kind: UseKind) -> bool {
    match kind {
        UseKind::Predicate => matches!(root, Definition::StateQueries | Definition::StateGraph),
        UseKind::State | UseKind::StateCheck => matches!(
            root,
            Definition::StateCore | Definition::StateQueries | Definition::StateGraph
        ),
        UseKind::Temporal | UseKind::TimedObligation => matches!(
            root,
            Definition::EventPosition | Definition::FixedSample | Definition::TimestampedWindow
        ),
        UseKind::Protocol => root == Definition::Protocol,
    }
}

fn locate(source: u32, span: Span, work: &mut Work) -> Result<(), Error> {
    work.locus = Some(w::Locus {
        source,
        span: w::Span {
            start: index(span.start)?,
            end: index(span.end)?,
        },
    });
    Ok(())
}

#[derive(Clone, Copy)]
enum Progress {
    None,
    Observable,
    NeedsAuthority,
}

fn protocol_check(
    unit: &ComposedUnit,
    typed: &DeclarationTypes<'_>,
    scope: &DeclarationScope,
    span: Span,
    source: u32,
    protocol: &c::Protocol,
    work: &mut Work,
) -> Result<(), Error> {
    let constants = constants(unit, typed, scope, source, work)?;
    let range = c::arena::owned_range(unit.controls(), span, |node| node.span, || work.visit())?;
    let mut progress = BTreeMap::new();
    for at in range {
        work.visit()?;
        let node = &unit.controls()[at];
        locate(source, node.span, work)?;
        let value = match &node.kind {
            c::ControlKind::Sequence(children) => {
                combined(children.iter().copied(), &progress, work)?
            }
            c::ControlKind::Parallel { branches, join } => {
                let mut names = BTreeSet::new();
                for branch in branches {
                    work.visit()?;
                    work.bytes(branch.name.value.len().saturating_mul(branches.len() + 1))?;
                    work.charge(Dimension::Entries, 1)?;
                    if !names.insert(branch.name.value.as_str()) {
                        return Err(Error::Invalid(Invalid::Control));
                    }
                }
                if join.len() != branches.len() {
                    return Err(Error::Invalid(Invalid::Control));
                }
                for joined in join {
                    work.visit()?;
                    work.bytes(joined.value.len().saturating_mul(branches.len() + 1))?;
                    if !names.remove(joined.value.as_str()) {
                        return Err(Error::Invalid(Invalid::Control));
                    }
                }
                combined(
                    branches.iter().map(|branch| branch.control),
                    &progress,
                    work,
                )?
            }
            c::ControlKind::Choice { visible, cases, .. } => {
                let mut closed = true;
                for expression in visible
                    .iter()
                    .copied()
                    .chain(cases.iter().map(|case| case.guard))
                {
                    work.visit()?;
                    closed &= constants.contains_key(&expression.0);
                }
                if !closed {
                    let context = decisions::Context {
                        unit,
                        typed,
                        scope,
                        protocol,
                        source,
                    };
                    let feasible = decisions::partition(&context, c::ControlId(at), work)?;
                    if feasible.len() != cases.len() {
                        return Err(Error::Invalid(Invalid::Control));
                    }
                    let mut result = Progress::Observable;
                    for (case, feasible) in cases.iter().zip(feasible) {
                        work.visit()?;
                        if feasible
                            && !matches!(
                                child(case.control, &progress, work)?,
                                Progress::Observable
                            )
                        {
                            // Conservative atom independence can retain branches
                            // excluded by an unavailable correlation authority.
                            result = Progress::NeedsAuthority;
                        }
                    }
                    result
                } else {
                    visible_constants(visible, &constants, work)?;
                    let mut selected = None;
                    for case in cases {
                        work.visit()?;
                        locate(
                            source,
                            unit.expression(case.guard)
                                .ok_or(Error::Invalid(Invalid::Reference))?
                                .span,
                            work,
                        )?;
                        if constant(case.guard, &constants, work)?
                            && selected.replace(case.control).is_some()
                        {
                            return Err(Error::Invalid(Invalid::Control));
                        }
                    }
                    child(
                        selected.ok_or(Error::Invalid(Invalid::Control))?,
                        &progress,
                        work,
                    )?
                }
            }
            c::ControlKind::Repeat {
                visible,
                maximum,
                guard,
                body,
                exhausted,
                ..
            } => {
                visible_constants(visible, &constants, work)?;
                let maximum = natural(&maximum.value, work)?;
                let guard = constant(*guard, &constants, work)?;
                let body = child(*body, &progress, work)?;
                let exhausted = child(*exhausted, &progress, work)?;
                if !guard {
                    Progress::None
                } else if maximum == 0 {
                    exhausted
                } else {
                    match body {
                        Progress::Observable => Progress::Observable,
                        Progress::None => return Err(Error::Invalid(Invalid::Control)),
                        Progress::NeedsAuthority => {
                            return Err(Error::Unsupported(Unsupported::FamilyProof))
                        }
                    }
                }
            }
            c::ControlKind::Await {
                within,
                event,
                then,
                timeout,
                ..
            } => {
                bounds(within, work)?;
                let success = combined([*event, *then].into_iter(), &progress, work)?;
                let timeout = child(*timeout, &progress, work)?;
                match (success, timeout) {
                    (Progress::Observable, Progress::Observable) => Progress::Observable,
                    // Deadline-based progress needs the actual selected temporal
                    // authority; the mere finite syntax interval is not that proof.
                    _ => Progress::NeedsAuthority,
                }
            }
            c::ControlKind::Event(_) | c::ControlKind::Commit { .. } => Progress::Observable,
            c::ControlKind::Check { .. } => Progress::None,
        };
        work.charge(Dimension::Entries, 1)?;
        progress.insert(at, value);
    }
    child(protocol.run, &progress, work)?;
    for channel in &protocol.channels {
        work.visit()?;
        locate(source, channel.span, work)?;
        bounds(&channel.delivery, work)?;
    }
    for requirement in &protocol.requirements {
        work.visit()?;
        match requirement {
            c::ProtocolRequirement::Temporal { .. } => {}
            c::ProtocolRequirement::Compensation(value) => {
                locate(source, value.span, work)?;
                bounds(&value.within, work)?;
                if natural(&value.attempts.value, work)? == 0 {
                    return Err(Error::Invalid(Invalid::NumericDomain));
                }
            }
        }
    }
    Ok(())
}

fn child(
    id: c::ControlId,
    progress: &BTreeMap<usize, Progress>,
    work: &mut Work,
) -> Result<Progress, Error> {
    work.visit()?;
    progress
        .get(&id.0)
        .copied()
        .ok_or(Error::Invalid(Invalid::Control))
}

fn combined(
    children: impl Iterator<Item = c::ControlId>,
    progress: &BTreeMap<usize, Progress>,
    work: &mut Work,
) -> Result<Progress, Error> {
    let mut result = Progress::None;
    for id in children {
        result = match (result, child(id, progress, work)?) {
            (Progress::Observable, _) | (_, Progress::Observable) => Progress::Observable,
            (Progress::NeedsAuthority, _) | (_, Progress::NeedsAuthority) => {
                Progress::NeedsAuthority
            }
            (Progress::None, Progress::None) => Progress::None,
        };
    }
    Ok(result)
}

// A small closed Boolean interpretation, not a numeric solver or a visibility
// oracle for model values. Every unknown semantic input stays unknown.
fn constants(
    unit: &ComposedUnit,
    typed: &DeclarationTypes<'_>,
    scope: &DeclarationScope,
    source: u32,
    work: &mut Work,
) -> Result<BTreeMap<usize, bool>, Error> {
    let mut result: BTreeMap<usize, bool> = BTreeMap::new();
    for node in typed.nodes() {
        work.visit()?;
        let syntax = unit
            .expression(node.expression)
            .ok_or(Error::Invalid(Invalid::Reference))?;
        locate(source, syntax.span, work)?;
        let mut get = |id: ExprId| -> Result<Option<bool>, Error> {
            work.visit()?;
            Ok(result.get(&id.0).copied())
        };
        let value = match &syntax.kind {
            c::ValueKind::Shared(kind) => match kind {
                ExprKind::Boolean(value) => Some(*value),
                ExprKind::Group { inner } => get(*inner)?,
                ExprKind::Unary {
                    op: UnaryOp::Not,
                    argument,
                } => get(*argument)?.map(|value| !value),
                ExprKind::Binary { op, left, right } => match op {
                    // Closed means every original operand is closed. Truth
                    // simplification cannot hide a visibility obligation.
                    BinaryOp::And => get(*left)?.zip(get(*right)?).map(|(a, b)| a & b),
                    BinaryOp::Or => get(*left)?.zip(get(*right)?).map(|(a, b)| a | b),
                    BinaryOp::Implies => get(*left)?.zip(get(*right)?).map(|(a, b)| !a | b),
                    BinaryOp::Equal => get(*left)?
                        .zip(get(*right)?)
                        .map(|(left, right)| left == right),
                    BinaryOp::NotEqual => get(*left)?
                        .zip(get(*right)?)
                        .map(|(left, right)| left != right),
                    BinaryOp::Add
                    | BinaryOp::Subtract
                    | BinaryOp::Multiply
                    | BinaryOp::Divide
                    | BinaryOp::Remainder
                    | BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual => None,
                },
                ExprKind::If {
                    condition,
                    then_value,
                    else_value,
                } => get(*condition)?
                    .zip(get(*then_value)?)
                    .zip(get(*else_value)?)
                    .map(
                        |((condition, then_value), else_value)| {
                            if condition {
                                then_value
                            } else {
                                else_value
                            }
                        },
                    ),
                ExprKind::Let { value, body, .. } => {
                    get(*value)?.zip(get(*body)?).map(|(_, body)| body)
                }
                ExprKind::Name(_) => {
                    let original = node
                        .binder
                        .and_then(|binder| scope.binders.get(binder.index()));
                    match original.map(|binder| (&binder.kind, &binder.ty)) {
                        Some((BinderKind::Let, BinderType::Initializer(initializer))) => {
                            get(*initializer)?
                        }
                        _ => None,
                    }
                }
                ExprKind::Integer(_)
                | ExprKind::Text(_)
                | ExprKind::SelfValue
                | ExprKind::ResultValue
                | ExprKind::EnumValue { .. }
                | ExprKind::Field { .. }
                | ExprKind::Unary {
                    op: UnaryOp::Negate,
                    ..
                }
                | ExprKind::Call { .. }
                | ExprKind::Quantifier { .. }
                | ExprKind::Reaches { .. } => None,
            },
            c::ValueKind::Invoke { .. }
            | c::ValueKind::Rational { .. }
            | c::ValueKind::Product { .. }
            | c::ValueKind::Size { .. }
            | c::ValueKind::Contains { .. }
            | c::ValueKind::Query { .. } => None,
        };
        if let Some(value) = value {
            work.charge(Dimension::Entries, 1)?;
            result.insert(node.expression.0, value);
        }
    }
    Ok(result)
}

fn constant(id: ExprId, values: &BTreeMap<usize, bool>, work: &mut Work) -> Result<bool, Error> {
    work.visit()?;
    values
        .get(&id.0)
        .copied()
        .ok_or(Error::Unsupported(Unsupported::FamilyProof))
}

fn visible_constants(
    visible: &[ExprId],
    values: &BTreeMap<usize, bool>,
    work: &mut Work,
) -> Result<(), Error> {
    for &at in visible {
        constant(at, values, work)?;
    }
    Ok(())
}

fn natural(value: &str, work: &mut Work) -> Result<i64, Error> {
    work.bytes(value.len())?;
    let value = value
        .parse::<i64>()
        .map_err(|_| Error::Invalid(Invalid::NumericDomain))?;
    if value < 0 {
        return Err(Error::Invalid(Invalid::NumericDomain));
    }
    Ok(value)
}

pub(super) fn interval(value: &c::Interval, work: &mut Work) -> Result<w::Interval, Error> {
    let (lower, upper) = bounds(value, work)?;
    work.charge(Dimension::Entries, 1)?;
    Ok(w::Interval {
        lower: integer(lower, work)?,
        upper: integer(upper, work)?,
    })
}

fn bounds(value: &c::Interval, work: &mut Work) -> Result<(i64, i64), Error> {
    let lower = natural(&value.lower.value, work)?;
    let upper = natural(&value.upper.value, work)?;
    if lower > upper {
        return Err(Error::Invalid(Invalid::NumericDomain));
    }
    Ok((lower, upper))
}

fn temporal_shape(kind: &c::TemporalKind, work: &mut Work) -> Result<(), Error> {
    let (timed, selected) = match kind {
        c::TemporalKind::Constant(_) | c::TemporalKind::Holds(_) | c::TemporalKind::Group(_) => {
            return Ok(())
        }
        c::TemporalKind::Unary { op, interval, .. } => (
            match op.value {
                c::TemporalOp::Not => false,
                c::TemporalOp::Eventually
                | c::TemporalOp::Always
                | c::TemporalOp::Once
                | c::TemporalOp::Historically => true,
                c::TemporalOp::Implies
                | c::TemporalOp::Or
                | c::TemporalOp::And
                | c::TemporalOp::Until
                | c::TemporalOp::Release
                | c::TemporalOp::Since
                | c::TemporalOp::Triggered => return Err(Error::Invalid(Invalid::Type)),
            },
            interval,
        ),
        c::TemporalKind::Binary { op, interval, .. } => (
            match op.value {
                c::TemporalOp::Implies | c::TemporalOp::Or | c::TemporalOp::And => false,
                c::TemporalOp::Until
                | c::TemporalOp::Release
                | c::TemporalOp::Since
                | c::TemporalOp::Triggered => true,
                c::TemporalOp::Not
                | c::TemporalOp::Eventually
                | c::TemporalOp::Always
                | c::TemporalOp::Once
                | c::TemporalOp::Historically => return Err(Error::Invalid(Invalid::Type)),
            },
            interval,
        ),
    };
    if timed != selected.is_some() {
        return Err(Error::Invalid(Invalid::Profile));
    }
    if let Some(value) = selected {
        bounds(value, work)?;
    }
    Ok(())
}

pub(super) fn temporal(
    unit: &ComposedUnit,
    layout: &DeclLayout,
    work: &mut Work,
) -> Result<Vec<w::Temporal>, Error> {
    let mut result = Vec::new();
    for &at in &layout.temporal {
        work.visit()?;
        let node = unit
            .temporal(at)
            .ok_or(Error::Invalid(Invalid::Reference))?;
        let locus = layout.locus(node.span)?;
        work.locus = Some(locus.clone());
        temporal_shape(&node.kind, work)?;
        let operation = match &node.kind {
            c::TemporalKind::Constant(value) => w::TemporalOperation::Constant { value: *value },
            c::TemporalKind::Holds(value) => w::TemporalOperation::Holds {
                value: layout.value(*value)?,
            },
            c::TemporalKind::Group(value) => w::TemporalOperation::Group {
                value: layout.temporal(*value)?,
            },
            c::TemporalKind::Unary {
                op,
                interval: range,
                argument,
            } => w::TemporalOperation::Unary {
                operator: match op.value {
                    c::TemporalOp::Not => w::TemporalUnary::Not,
                    c::TemporalOp::Eventually => w::TemporalUnary::Eventually,
                    c::TemporalOp::Always => w::TemporalUnary::Always,
                    c::TemporalOp::Once => w::TemporalUnary::Once,
                    c::TemporalOp::Historically => w::TemporalUnary::Historically,
                    c::TemporalOp::Implies
                    | c::TemporalOp::Or
                    | c::TemporalOp::And
                    | c::TemporalOp::Until
                    | c::TemporalOp::Release
                    | c::TemporalOp::Since
                    | c::TemporalOp::Triggered => return Err(Error::Invalid(Invalid::Type)),
                },
                interval: w::Nullable(
                    range
                        .as_ref()
                        .map(|range| interval(range, work))
                        .transpose()?,
                ),
                value: layout.temporal(*argument)?,
            },
            c::TemporalKind::Binary {
                op,
                interval: range,
                left,
                right,
            } => w::TemporalOperation::Binary {
                operator: match op.value {
                    c::TemporalOp::Implies => w::TemporalBinary::Implies,
                    c::TemporalOp::Or => w::TemporalBinary::Or,
                    c::TemporalOp::And => w::TemporalBinary::And,
                    c::TemporalOp::Until => w::TemporalBinary::Until,
                    c::TemporalOp::Release => w::TemporalBinary::Release,
                    c::TemporalOp::Since => w::TemporalBinary::Since,
                    c::TemporalOp::Triggered => w::TemporalBinary::Triggered,
                    c::TemporalOp::Not
                    | c::TemporalOp::Eventually
                    | c::TemporalOp::Always
                    | c::TemporalOp::Once
                    | c::TemporalOp::Historically => return Err(Error::Invalid(Invalid::Type)),
                },
                interval: w::Nullable(
                    range
                        .as_ref()
                        .map(|range| interval(range, work))
                        .transpose()?,
                ),
                left: layout.temporal(*left)?,
                right: layout.temporal(*right)?,
            },
        };
        work.charge(Dimension::Entries, 1)?;
        result.push(w::Temporal {
            original_node: index(at.0)?,
            locus,
            operation,
        });
    }
    Ok(result)
}
