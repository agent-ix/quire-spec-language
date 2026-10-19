// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-337: the Boolean condition of each `holds` atom of a checked temporal
//! clause, typed so S4 can lower it as FR-093 lowers a Boolean expression.
//!
//! A condition reads the clause's `over` parameter and nothing else of the
//! clause: it is typed as a state clause types its body (FR-104), with the
//! `over` parameter bound at slot 0 and no `self`, so a field read through
//! the `over` object is a model read of the current state. Each condition
//! is located at its own declaration site (`lowering::temporal_site`), so
//! a refusal inside it, and every occurrence its lowered nodes record,
//! resolves to the condition's own source.

use std::collections::BTreeMap;

use qsl_forms::{ClauseKind, StateClauseKind, TemporalNodeForm};
use quire_exact::ValueType;

use super::CheckedTemporalClause;
use crate::check::check::{bind_parameters, Scope, StateContext, Typer};
use crate::check::facts::Definedness;
use crate::check::ir::{CheckedBody, Observation};
use crate::check::lowering::{temporal_location, TemporalSite};
use crate::check::observation::Observations;
use crate::check::refusal::{CheckCause, CheckRefusal};
use crate::check::state_clause::ClauseDeclarations;
use crate::family::CheckContext;

/// A slot no local carries: a condition has no `self`.
const NO_SLOT: usize = usize::MAX;

/// One `holds` atom's condition, typed (FR-337 "Inputs").
#[derive(Debug)]
pub(crate) struct TypedCondition {
    /// The checked Boolean condition.
    pub(crate) body: CheckedBody,
    /// The name each slot of the condition was bound under.
    pub(crate) slot_names: Vec<String>,
    /// The compound units typing formed, which lowering keys.
    pub(crate) formed_units: quire_semantic_value::quantity::UnitTable,
}

/// The typed condition of every `holds` atom of the clause at `clause`
/// (its index among the unit's temporal clauses), by its formula node's
/// index, typed over the clause's `over` parameter. `input.nodes_used` is
/// the package's running node total before the clause, and the total after
/// it is returned.
pub(crate) fn check_conditions(
    clause: usize,
    form: &qsl_forms::TemporalClauseForm,
    checked: &CheckedTemporalClause,
    input: &ClauseDeclarations<'_>,
    cx: &mut CheckContext<'_, Scope>,
) -> Result<(BTreeMap<usize, TypedCondition>, u64), CheckRefusal> {
    let mut nodes = input.nodes_used;
    let bound = [checked.over().clone()];
    let mut conditions = BTreeMap::new();
    for (id, node) in form.formula.iter() {
        let TemporalNodeForm::Holds { condition, .. } = node else {
            continue;
        };
        let location = temporal_location(clause, TemporalSite::Condition(id.index()));
        let mut typer = Typer::new(
            input.scope,
            input.signatures,
            input.checking_limits,
            &mut nodes,
            ClauseKind::Invariant,
        )
        .with_cancel(input.cancel)
        .with_context(cx.reborrow(input.scope));
        bind_parameters(&mut typer, &bound, &location)?;
        typer.enter_state_clause(StateContext {
            parent_projections: BTreeMap::new(),
            kind: StateClauseKind::Invariant,
            operation: None,
            self_slot: NO_SLOT,
            result_slot: None,
        });
        let body = typer.infer(&condition.expression, Some(&ValueType::Boolean), &location)?;
        if body.root().value_type() != &ValueType::Boolean {
            return Err(CheckRefusal {
                location,
                cause: CheckCause::NonBooleanRoot,
            });
        }
        let slot_names = typer.slot_names().to_vec();
        let formed_units = typer.into_formed_units();
        let observations = Observations::of(body.root(), Observation::Current, bound.len());
        Definedness::new(
            bound.len(),
            input.dispatch_tables,
            &input.scope.dispatch_operations,
        )
        .with_observations(&observations)
        .check(body.root())?;
        conditions.insert(
            id.index(),
            TypedCondition {
                body,
                slot_names,
                formed_units,
            },
        );
    }
    Ok((conditions, nodes))
}
