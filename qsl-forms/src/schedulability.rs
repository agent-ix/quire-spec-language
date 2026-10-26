// SPDX-License-Identifier: AGPL-3.0-or-later
//! S2 forms for QSpec FR-419's shared task-set front end.

use qsl_cst::{CstNode, LosslessCst, Production, TokenKind};

use crate::dispatch::{Construct, FormsRefusal};
use crate::syntax::{
    DeclarationForm, NameForm, QuantityForm, SchedulabilityClaimForm, SchedulabilityClaimKind,
    TaskArrivalKind, TaskCriticality, TaskForm, TaskQuantityForm, TaskSetForm,
};
use crate::value::{
    exact_value, items, name_form, nodes_of, only, production_node, quantity_form, text, tokens_of,
    Item,
};

pub(crate) fn taskset(construct: Construct<'_>) -> Result<DeclarationForm, FormsRefusal> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let parts = items(cst, node);
    let identifiers = tokens_of(&parts, TokenKind::Identifier);
    let Some(name) = identifiers.last().copied() else {
        return Err(unexpected(node));
    };
    let tasks = nodes_of(&parts, Production::TaskDeclaration)
        .into_iter()
        .map(|task| task_form(cst, task))
        .collect::<Result<_, _>>()?;
    Ok(DeclarationForm::TaskSet(Box::new(TaskSetForm {
        name: NameForm::new(text(name, node)?, name.span()),
        tasks,
        span: node.span(),
    })))
}

fn task_form(cst: &LosslessCst, node: &CstNode) -> Result<TaskForm, FormsRefusal> {
    let parts = items(cst, node);
    let identifiers = tokens_of(&parts, TokenKind::Identifier);
    let task_name = identifiers
        .iter()
        .copied()
        .find(|token| {
            token.spelling() != b"task"
                && token.spelling() != b"periodic"
                && token.spelling() != b"sporadic"
        })
        .ok_or_else(|| unexpected(node))?;
    let arrival = if identifiers
        .iter()
        .any(|token| token.spelling() == b"periodic")
    {
        TaskArrivalKind::Periodic
    } else if identifiers
        .iter()
        .any(|token| token.spelling() == b"sporadic")
    {
        TaskArrivalKind::Sporadic
    } else {
        return Err(unexpected(node));
    };
    let quantities = nodes_of(&parts, Production::TaskQuantity);
    let mut quantities = quantities.into_iter();
    let interval = task_quantity(cst, quantities.next().ok_or_else(|| unexpected(node))?)?;
    let deadline = task_quantity(cst, quantities.next().ok_or_else(|| unexpected(node))?)?;
    let wcet = task_quantity(cst, quantities.next().ok_or_else(|| unexpected(node))?)?;
    let words = identifiers
        .iter()
        .map(|token| token.spelling())
        .collect::<Vec<_>>();
    let wcet_hi = if words.contains(&b"wcet_hi".as_slice()) {
        Some(task_quantity(
            cst,
            quantities.next().ok_or_else(|| unexpected(node))?,
        )?)
    } else {
        None
    };
    let priority_token = parts.iter().find_map(|item| match item {
        Item::Token(token) if token.kind() == TokenKind::Integer => Some(*token),
        _ => None,
    });
    let priority = priority_token
        .map(|token| {
            text(token, node).and_then(|spelling| {
                spelling
                    .parse::<u64>()
                    .map(|value| (value, token.span()))
                    .map_err(|_| unexpected(node))
            })
        })
        .transpose()?;
    let criticality = if words.contains(&b"hi".as_slice()) {
        TaskCriticality::Hi
    } else if words.contains(&b"lo".as_slice()) {
        TaskCriticality::Lo
    } else {
        return Err(unexpected(node));
    };
    let jitter = if words.contains(&b"jitter".as_slice()) {
        Some(task_quantity(
            cst,
            quantities.next().ok_or_else(|| unexpected(node))?,
        )?)
    } else {
        None
    };
    let blocking = if words.contains(&b"blocking".as_slice()) {
        Some(task_quantity(
            cst,
            quantities.next().ok_or_else(|| unexpected(node))?,
        )?)
    } else {
        None
    };
    if quantities.next().is_some() {
        return Err(unexpected(node));
    }
    Ok(TaskForm {
        name: NameForm::new(text(task_name, node)?, task_name.span()),
        arrival,
        interval,
        deadline,
        wcet,
        wcet_hi,
        priority,
        criticality,
        jitter,
        blocking,
        span: node.span(),
    })
}

fn task_quantity(cst: &LosslessCst, node: &CstNode) -> Result<TaskQuantityForm, FormsRefusal> {
    let parts = items(cst, node);
    let negative = parts.iter().any(|item| {
        matches!(item,
        Item::Token(token) if token.spelling() == b"-")
    });
    let value = only(&parts, Production::ExactValue, node)?;
    let unit = only(&parts, Production::QualifiedName, node)?;
    Ok(TaskQuantityForm {
        quantity: QuantityForm {
            magnitude: exact_value(cst, value)?,
            unit: name_form(cst, unit)?,
            span: node.span(),
        },
        negative,
        span: node.span(),
    })
}

pub(crate) fn claim(construct: Construct<'_>) -> Result<DeclarationForm, FormsRefusal> {
    let cst = construct.cst;
    let node = production_node(construct)?;
    let parts = items(cst, node);
    let identifiers = tokens_of(&parts, TokenKind::Identifier);
    let spellings = identifiers
        .iter()
        .map(|token| token.spelling())
        .collect::<Vec<_>>();
    let is_response = spellings.first() == Some(&b"response".as_slice());
    let (name, task_set, kind) = if is_response {
        let [_, name, task_set, task, _, policy] = identifiers.as_slice() else {
            return Err(unexpected(node));
        };
        let bound = quantity_form(cst, only(&parts, Production::QuantityLiteral, node)?)?;
        if policy.spelling() != b"fixed_priority" {
            return Err(unexpected(node));
        }
        (
            name,
            task_set,
            SchedulabilityClaimKind::Response {
                task: NameForm::new(text(task, node)?, task.span()),
                bound: Box::new(bound),
            },
        )
    } else {
        let [_, name, task_set, _, policy] = identifiers.as_slice() else {
            return Err(unexpected(node));
        };
        (
            name,
            task_set,
            SchedulabilityClaimKind::Schedulable {
                policy: text(policy, node)?,
            },
        )
    };
    Ok(DeclarationForm::SchedulabilityClaim(Box::new(
        SchedulabilityClaimForm {
            name: NameForm::new(text(name, node)?, name.span()),
            task_set: NameForm::new(text(task_set, node)?, task_set.span()),
            kind,
            span: node.span(),
        },
    )))
}

fn unexpected(node: &CstNode) -> FormsRefusal {
    FormsRefusal::at(
        crate::dispatch::FormsCause::UnexpectedShape {
            production: node.production(),
        },
        node.span(),
    )
}
