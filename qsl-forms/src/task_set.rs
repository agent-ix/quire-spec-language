// SPDX-License-Identifier: AGPL-3.0-or-later
//! The S2 build of a task set and the claims over it: `taskset`, `schedulable`
//! and `response` (QSpec shared grammar `taskset-decl`, `task`,
//! `signed-quantity` and `schedulability-clause`, FR-419).
//!
//! These forms select no meaning, the contract `time_model` keeps: the sign of
//! a value, its unit, the emptiness of a task set, a `lo` task's `wcet_hi`,
//! the task-set and task names and the policy all stay spelled and spanned.
//! The grammar admits an empty task set, a negative time value and a `lo`
//! task with a `wcet_hi` so that S3, not the recognizer, refuses them
//! (QSL FR-246). Every word these declarations introduce is contextual, so
//! the builders read them as identifier tokens by position.

use qsl_cst::{CstNode, CstToken, LosslessCst, Production, TokenKind};

use super::dispatch::{Construct, FormsRefusal};
use super::syntax::{
    ArrivalForm, CriticalityForm, DeclarationForm, DeclaredName, PolicyForm, PriorityForm,
    ResponseForm, SchedulableForm, SignedQuantityForm, TaskForm, TasksetForm,
};
use super::value::{
    integer, items, nodes_of, only, production_node, quantity_form, text, tokens_of, unexpected,
    Item,
};

fn name_of(token: &CstToken, node: &CstNode) -> Result<DeclaredName, FormsRefusal> {
    Ok(DeclaredName {
        name: text(token, node)?,
        span: token.span(),
    })
}

/// `taskset Name { task* }`.
pub(crate) fn taskset_declaration(
    construct: Construct<'_>,
) -> Result<DeclarationForm, FormsRefusal> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let declaration_items = items(cst, node);
    // `taskset` is a contextual word, so the declared name is the second
    // identifier token.
    let Some(name) = tokens_of(&declaration_items, TokenKind::Identifier)
        .get(1)
        .copied()
    else {
        return Err(unexpected(node));
    };
    let tasks = nodes_of(&declaration_items, Production::Task)
        .into_iter()
        .map(|task| task_form(cst, task))
        .collect::<Result<_, _>>()?;
    Ok(DeclarationForm::Taskset(Box::new(TasksetForm {
        name: name_of(name, node)?,
        tasks,
        span: node.span(),
    })))
}

fn signed_quantity(cst: &LosslessCst, node: &CstNode) -> Result<SignedQuantityForm, FormsRefusal> {
    let quantity_items = items(cst, node);
    Ok(SignedQuantityForm {
        negative: quantity_items
            .iter()
            .any(|item| matches!(item, Item::Token(token) if token.spelling() == b"-")),
        quantity: quantity_form(
            cst,
            only(&quantity_items, Production::QuantityLiteral, node)?,
        )?,
        span: node.span(),
    })
}

/// One `task`. The members follow the grammar's fixed order, so each
/// `signed-quantity` belongs to the member word read before it.
fn task_form(cst: &LosslessCst, node: &CstNode) -> Result<TaskForm, FormsRefusal> {
    let task_items = items(cst, node);
    let words = tokens_of(&task_items, TokenKind::Identifier);
    // `task name (periodic | sporadic) ...`.
    let [_, name, arrival, ..] = words[..] else {
        return Err(unexpected(node));
    };
    let arrival_word = arrival;
    let arrival = match arrival.spelling() {
        b"periodic" => ArrivalForm::Periodic,
        b"sporadic" => ArrivalForm::Sporadic,
        _ => return Err(unexpected(node)),
    };
    let mut member: &[u8] = b"interval";
    let mut interval = None;
    let mut deadline = None;
    let mut wcet = None;
    let mut jitter = None;
    let mut blocking = None;
    let mut wcet_hi = None;
    let mut priority = None;
    let mut criticality = None;
    for item in &task_items {
        match item {
            Item::Token(token) if token.kind() == TokenKind::Identifier => {
                if token.span().start <= arrival_word.span().start {
                    continue;
                }
                match token.spelling() {
                    b"deadline" => member = b"deadline",
                    b"wcet" => member = b"wcet",
                    b"priority" => member = b"priority",
                    b"jitter" => member = b"jitter",
                    b"blocking" => member = b"blocking",
                    b"wcet_hi" => member = b"wcet_hi",
                    b"lo" => criticality = Some(CriticalityForm::Lo),
                    b"hi" => criticality = Some(CriticalityForm::Hi),
                    _ => return Err(unexpected(node)),
                }
            }
            Item::Token(token) if token.kind() == TokenKind::Integer && member == b"priority" => {
                priority = Some(PriorityForm {
                    value: integer(&text(token, node)?, node)?,
                    span: token.span(),
                });
            }
            Item::Node(quantity) if quantity.production() == Production::SignedQuantity => {
                let value = signed_quantity(cst, quantity)?;
                let slot = match member {
                    b"interval" => &mut interval,
                    b"deadline" => &mut deadline,
                    b"wcet" => &mut wcet,
                    b"jitter" => &mut jitter,
                    b"blocking" => &mut blocking,
                    b"wcet_hi" => &mut wcet_hi,
                    _ => return Err(unexpected(node)),
                };
                *slot = Some(value);
            }
            Item::Token(_) | Item::Node(_) => {}
        }
    }
    let (Some(interval), Some(deadline), Some(wcet), Some(criticality)) =
        (interval, deadline, wcet, criticality)
    else {
        return Err(unexpected(node));
    };
    Ok(TaskForm {
        name: name_of(name, node)?,
        arrival,
        interval,
        deadline,
        wcet,
        priority,
        jitter,
        blocking,
        criticality,
        wcet_hi,
        span: node.span(),
    })
}

/// `schedulable Name on TaskSet under policy;`.
pub(crate) fn schedulable_declaration(
    construct: Construct<'_>,
) -> Result<DeclarationForm, FormsRefusal> {
    let node = production_node(construct)?;
    let declaration_items = items(construct.cst, node);
    // `schedulable name on set under policy`: `on` is reserved, the rest are
    // identifier tokens.
    let [_, name, set, _, policy] = tokens_of(&declaration_items, TokenKind::Identifier)[..] else {
        return Err(unexpected(node));
    };
    let policy_form = match policy.spelling() {
        b"fixed_priority" => PolicyForm::FixedPriority,
        b"edf" => PolicyForm::Edf,
        b"amc" => PolicyForm::Amc,
        _ => return Err(unexpected(node)),
    };
    Ok(DeclarationForm::Schedulable(Box::new(SchedulableForm {
        name: name_of(name, node)?,
        task_set: name_of(set, node)?,
        policy: policy_form,
        policy_span: policy.span(),
        span: node.span(),
    })))
}

/// `response Name on TaskSet.task <= d under fixed_priority;`.
pub(crate) fn response_declaration(
    construct: Construct<'_>,
) -> Result<DeclarationForm, FormsRefusal> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let declaration_items = items(cst, node);
    let [_, name, set, task, _, _] = tokens_of(&declaration_items, TokenKind::Identifier)[..]
    else {
        return Err(unexpected(node));
    };
    Ok(DeclarationForm::Response(Box::new(ResponseForm {
        name: name_of(name, node)?,
        task_set: name_of(set, node)?,
        task: name_of(task, node)?,
        bound: quantity_form(
            cst,
            only(&declaration_items, Production::QuantityLiteral, node)?,
        )?,
        span: node.span(),
    })))
}
