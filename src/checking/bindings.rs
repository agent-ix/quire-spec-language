// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-016: exact source, authored identity and operation-anchor preflight.

use std::collections::{BTreeMap, BTreeSet};

use quire_contract_ir as ir;

use super::{failure, CheckBindings, Result};
use crate::linking::LinkedPackage;
use crate::syntax::ClauseKind;
use qsl_foundation::{Code, Span};

pub(super) fn validate(linked: &LinkedPackage<'_>, bindings: &CheckBindings) -> Result<Vec<usize>> {
    let unit = linked.unit();
    let at_start = Span { start: 0, end: 0 };
    let invalid = |message| failure(unit.source(), Code::InvalidModelBinding, at_start, message);
    linked.require_historical_native().map_err(|mut error| {
        error.diagnostic.phase = qsl_foundation::Phase::Check;
        error
    })?;
    let bound = bindings.source.source();
    if bound.identity() != unit.source().identity()
        || bound.path() != unit.source().path()
        || bound.digest() != unit.source().digest()
    {
        return Err(invalid(
            "checking source does not match the exact linked source",
        ));
    }
    for selected in linked.models() {
        let model = selected
            .native_model()
            .ok_or_else(|| invalid("selected model has no native correspondence"))?;
        if model.source().identity() == bindings.source.identity()
            && (model.source().source().identity() != bound.identity()
                || model.source().source().digest() != bound.digest())
        {
            return Err(invalid(
                "native clause and model sources conflict under one formal source identity",
            ));
        }
    }
    if bindings.clauses.len() != unit.clauses().len() {
        return Err(invalid(
            "authored clause mapping must be complete with no extra entries",
        ));
    }
    let mut by_name = BTreeMap::new();
    let mut identities = BTreeSet::new();
    for (index, binding) in bindings.clauses.iter().enumerate() {
        if by_name.insert(binding.name.as_str(), index).is_some()
            || !identities.insert((&binding.requirement, &binding.clause))
        {
            return Err(invalid(
                "authored clause names and qualified identities must be unique",
            ));
        }
    }
    let mut order = Vec::with_capacity(unit.clauses().len());
    for (clause, resolved) in unit.clauses().iter().zip(linked.clauses()) {
        let index = *by_name
            .get(clause.name.value.as_str())
            .ok_or_else(|| invalid("native clause has no exact authored binding"))?;
        let point = &bindings.clauses[index].execution_point;
        let matched = match (clause.kind, point) {
            (
                ClauseKind::Invariant,
                ir::ExecutionPoint::Initialization { .. } | ir::ExecutionPoint::Handler { .. },
            ) => true,
            (ClauseKind::Precondition, ir::ExecutionPoint::Pre { operation })
            | (ClauseKind::Postcondition, ir::ExecutionPoint::Post { operation }) => {
                let model = linked.models()[resolved.model()]
                    .native_model()
                    .ok_or_else(|| invalid("operation model has no native correspondence"))?;
                let selected = resolved
                    .operation()
                    .ok_or_else(|| invalid("native operation clause has no selected operation"))?;
                model.roles().operations.iter().any(|role| {
                    selected.identity.key
                        == crate::linking::DeclarationKey::Operation {
                            context: role.context.clone(),
                            name: role.name.clone(),
                        }
                        && &role.anchor == operation
                })
            }
            _ => false,
        };
        if !matched {
            return Err(failure(
                unit.source(),
                Code::InvalidModelBinding,
                clause.span,
                "authored execution point disagrees with the native clause or operation anchor",
            ));
        }
        order.push(index);
    }
    Ok(order)
}
