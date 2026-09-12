// SPDX-License-Identifier: AGPL-3.0-only
//! FR-042: Boolean decisions over causally available role-owned observations.

mod formula;
mod received;

use std::collections::BTreeMap;

use crate::checking::{composed::DeclarationTypes, NativeType};
use crate::linking::composed::scopes::{BinderId, BinderKind, BinderType, DeclarationScope};
use crate::protocol_artifact::{work::Work, Dimension, Error, Invalid, Unsupported};
use crate::syntax::{composed as c, BinaryOp, ExprId, ExprKind, UnaryOp};

use formula::{Arena, Op};
use received::Received;

pub(super) struct Context<'s, 'm> {
    pub unit: &'s c::ComposedUnit,
    pub typed: &'s DeclarationTypes<'m>,
    pub scope: &'s DeclarationScope,
    pub protocol: &'s c::Protocol,
    pub source: u32,
}

#[derive(Clone, Copy)]
enum Meaning {
    Boolean(usize),
    // A pure immutable record alias retains identity, never a Boolean witness.
    Record(BinderId),
}

fn unsupported() -> Error {
    Error::Unsupported(Unsupported::FamilyProof)
}

pub(super) fn partition(
    context: &Context<'_, '_>,
    choice: c::ControlId,
    work: &mut Work,
) -> Result<Vec<bool>, Error> {
    let control = context
        .unit
        .control(choice)
        .ok_or(Error::Invalid(Invalid::Reference))?;
    let c::ControlKind::Choice { visible, cases, .. } = &control.kind else {
        return Err(Error::Invalid(Invalid::Control));
    };
    let mut received = Received::new(context, choice, work)?;
    let mut arena = Arena::new();
    let mut meanings = BTreeMap::new();
    for node in context.typed.nodes() {
        work.visit()?;
        let syntax = context
            .unit
            .expression(node.expression)
            .ok_or(Error::Invalid(Invalid::Reference))?;
        super::locate(context.source, syntax.span, work)?;
        // Dynamic decisions retain every original Boolean dependency, even
        // when a later operator yields a constant or ignores a let result.
        let meaning = lower(
            context,
            &mut received,
            &mut arena,
            &meanings,
            node.expression,
            work,
        )?;
        if let Some(meaning) = meaning {
            work.charge(Dimension::Entries, 1)?;
            meanings.insert(node.expression.0, meaning);
        }
    }
    let mut visible_roots = Vec::new();
    work.charge(Dimension::Entries, visible.len())?;
    visible_roots
        .try_reserve_exact(visible.len())
        .map_err(|_| Error::Allocation)?;
    for expression in visible {
        work.visit()?;
        let original = context
            .unit
            .expression(*expression)
            .ok_or(Error::Invalid(Invalid::Reference))?;
        super::locate(context.source, original.span, work)?;
        let root = boolean(&meanings, *expression, work)?.ok_or_else(unsupported)?;
        // The arena retains every original operand. It proves whether this
        // advertised result, rather than its individual atoms, determines a case.
        visible_roots.push(root);
    }
    let mut guards = Vec::new();
    for case in cases {
        let original = context
            .unit
            .expression(case.guard)
            .ok_or(Error::Invalid(Invalid::Reference))?;
        super::locate(context.source, original.span, work)?;
        let root = boolean(&meanings, case.guard, work)?.ok_or_else(unsupported)?;
        work.charge(Dimension::Entries, 1)?;
        guards.try_reserve(1).map_err(|_| Error::Allocation)?;
        guards.push(root);
    }
    super::locate(context.source, control.span, work)?;
    arena.partition(&guards, &visible_roots, received.atom_count(), work)
}

fn meaning(
    values: &BTreeMap<usize, Meaning>,
    id: ExprId,
    work: &mut Work,
) -> Result<Option<Meaning>, Error> {
    work.visit()?;
    Ok(values.get(&id.0).copied())
}

fn boolean(
    values: &BTreeMap<usize, Meaning>,
    id: ExprId,
    work: &mut Work,
) -> Result<Option<usize>, Error> {
    Ok(match meaning(values, id, work)? {
        Some(Meaning::Boolean(value)) => Some(value),
        _ => None,
    })
}

fn lower(
    context: &Context<'_, '_>,
    received: &mut Received<'_, '_>,
    arena: &mut Arena,
    values: &BTreeMap<usize, Meaning>,
    id: ExprId,
    work: &mut Work,
) -> Result<Option<Meaning>, Error> {
    let expression = context
        .unit
        .expression(id)
        .ok_or(Error::Invalid(Invalid::Reference))?;
    let kind = match &expression.kind {
        c::ValueKind::Shared(kind) => kind,
        c::ValueKind::Invoke { .. }
        | c::ValueKind::Rational { .. }
        | c::ValueKind::Product { .. }
        | c::ValueKind::Size { .. }
        | c::ValueKind::Contains { .. }
        | c::ValueKind::Query { .. } => return Ok(None),
    };
    let operation = match kind {
        ExprKind::Boolean(value) => Op::Constant(*value),
        ExprKind::Name(_) => {
            let Some(binder) = received.read(id, work)? else {
                return Ok(None);
            };
            work.visit()?;
            let original = context
                .scope
                .binders
                .get(binder.index())
                .ok_or(Error::Invalid(Invalid::Binding))?;
            return match (&original.kind, &original.ty) {
                (BinderKind::Let, BinderType::Initializer(initializer)) => {
                    meaning(values, *initializer, work)
                }
                (BinderKind::EventRecord, _) if received.eligible(binder, work)? => {
                    Ok(Some(Meaning::Record(binder)))
                }
                _ => Ok(None),
            };
        }
        ExprKind::Group { inner } => return meaning(values, *inner, work),
        ExprKind::Field { base, name } => {
            let Some(Meaning::Record(binder)) = meaning(values, *base, work)? else {
                return Ok(None);
            };
            work.visit()?;
            if !matches!(
                context.typed.node(id).and_then(|node| node.ty.as_ref()),
                Some(NativeType::Boolean)
            ) {
                return Ok(None);
            }
            let Some(atom) = received.field(binder, name, work)? else {
                return Ok(None);
            };
            Op::Atom(atom)
        }
        ExprKind::Unary {
            op: UnaryOp::Not,
            argument,
        } => {
            let Some(value) = boolean(values, *argument, work)? else {
                return Ok(None);
            };
            Op::Not(value)
        }
        ExprKind::Binary { op, left, right } => {
            let (Some(left), Some(right)) = (
                boolean(values, *left, work)?,
                boolean(values, *right, work)?,
            ) else {
                return Ok(None);
            };
            match op {
                BinaryOp::And => Op::And(left, right),
                BinaryOp::Or => Op::Or(left, right),
                BinaryOp::Implies => Op::Implies(left, right),
                BinaryOp::Equal => Op::Equal(left, right),
                BinaryOp::NotEqual => Op::NotEqual(left, right),
                BinaryOp::Add
                | BinaryOp::Subtract
                | BinaryOp::Multiply
                | BinaryOp::Divide
                | BinaryOp::Remainder
                | BinaryOp::Less
                | BinaryOp::LessEqual
                | BinaryOp::Greater
                | BinaryOp::GreaterEqual => return Ok(None),
            }
        }
        ExprKind::If {
            condition,
            then_value,
            else_value,
        } => {
            let (Some(condition), Some(then_value), Some(else_value)) = (
                boolean(values, *condition, work)?,
                boolean(values, *then_value, work)?,
                boolean(values, *else_value, work)?,
            ) else {
                return Ok(None);
            };
            Op::If {
                condition,
                then_value,
                else_value,
            }
        }
        ExprKind::Let {
            value: initializer,
            body,
            ..
        } => {
            match (
                meaning(values, *initializer, work)?,
                meaning(values, *body, work)?,
            ) {
                (Some(Meaning::Boolean(initializer)), Some(Meaning::Boolean(body))) => {
                    Op::Let { initializer, body }
                }
                // This initializer is only a transparent, already eligible
                // received-record identity, with no Boolean computation to hide.
                (Some(Meaning::Record(_)), body) => return Ok(body),
                _ => return Ok(None),
            }
        }
        ExprKind::Integer(_)
        | ExprKind::Text(_)
        | ExprKind::SelfValue
        | ExprKind::ResultValue
        | ExprKind::EnumValue { .. }
        | ExprKind::Unary {
            op: UnaryOp::Negate,
            ..
        }
        | ExprKind::Call { .. }
        | ExprKind::Quantifier { .. }
        | ExprKind::Reaches { .. } => return Ok(None),
    };
    Ok(Some(Meaning::Boolean(arena.push(operation, work)?)))
}
