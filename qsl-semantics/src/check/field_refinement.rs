// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-151 refinement consumes the fact closure retained by genuine source
//! postcondition checking. The model index identifies immediate-parent
//! field pairs and exposed writing operations; no caller-provided guard
//! or independent checker can establish their obligations.

use super::facts::Established;
use crate::model::key::DeclarationKey;
use quire_exact::Integer;

/// The obligation belonging to one immediate-parent field pair and one
/// exposed writer. Both proof forms may fail independently.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RefinementObligation {
    /// The actual writer does not establish the narrowed single-field presence.
    Presence,
    /// Its retained interval does not establish the narrowed scalar domain.
    Domain,
    /// The narrowed object/collection obligation has no FR-146 proof form.
    NoProofForm,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RefinementFailure {
    pub(crate) member: DeclarationKey,
    pub(crate) parent: DeclarationKey,
    pub(crate) writer: DeclarationKey,
    pub(crate) obligation: RefinementObligation,
}

/// Consume only facts retained by actual admitted source clauses. Each
/// exposed writer is checked separately, with its own operation lineage;
/// a descendant or sibling clause cannot establish its ancestor's promise.
pub(crate) fn checked_refinement_failures(
    model: &super::lowering::AdmittedModel,
    member_key: &DeclarationKey,
    parent_key: &DeclarationKey,
    forms: &[super::state_clause::StateClauseDeclaration],
    checked: &[super::state_clause::TypedStateClause],
) -> Vec<RefinementFailure> {
    use std::collections::BTreeSet;
    use qsl_forms::StateClauseKind;
    use quire_exact::Presence;

    let index = model.model_index();
    let (Some(member), Some(parent)) = (index.field(member_key), index.field(parent_key)) else {
        return Vec::new();
    };
    let raises_lower = member.multiplicity.lower > parent.multiplicity.lower;
    let requires_presence = member.presence == Presence::Required && parent.presence == Presence::Optional;
    let single = member.multiplicity.upper.is_some_and(|upper| upper <= 1);
    let domain = model.narrows_value_type(&member.value_type, &parent.value_type);
    let smaller_upper = match (member.multiplicity.upper, parent.multiplicity.upper) {
        (Some(child), Some(parent)) => child < parent,
        (Some(_), None) => true,
        _ => false,
    };
    if !(raises_lower || requires_presence || domain || smaller_upper) { return Vec::new(); }
    let lineage = index.field_lineage(member_key);
    let exposed: Vec<_> = index.operations().filter(|operation|
        model.reaches_owner(&member.owner, &operation.owner)).collect();
    let operation_lineage = |key: &DeclarationKey| {
        let mut result = BTreeSet::new();
        let mut current = Some(key);
        while let Some(key) = current {
            if !result.insert(key.clone()) { break; }
            current = index.operation(key).and_then(|operation| operation.redefines.as_ref());
        }
        result
    };
    let mut hidden = BTreeSet::new();
    for operation in &exposed {
        hidden.extend(operation_lineage(&operation.key).into_iter().filter(|key| key != &operation.key));
    }
    let mut failures = Vec::new();
    for writer in exposed.into_iter().filter(|writer| !hidden.contains(&writer.key)) {
        if !writer.effect.modifies.iter().any(|written| lineage.contains(&written)) { continue; }
        let writer_lineage = operation_lineage(&writer.key);
        let mut established = Established::default();
        for (form, typed) in forms.iter().zip(checked) {
            if form.kind != StateClauseKind::Postcondition { continue; }
            let Some(context) = model.declaration_key(form.context) else { continue; };
            if !model.reaches_owner(&member.owner, context) { continue; }
            let Some(operation) = form.operation.as_ref().and_then(|operation| model.operation_key(operation)) else { continue; };
            if !writer_lineage.contains(operation) { continue; }
            for field in &lineage {
                if let Some(reference) = model.field_reference(field) {
                    established.conjoin(typed.facts.attribute(0, &reference, super::ir::Observation::Post));
                }
            }
        }
        let mut fail = |obligation| failures.push(RefinementFailure {
            member: member_key.clone(), parent: parent_key.clone(), writer: writer.key.clone(), obligation,
        });
        if (raises_lower || requires_presence) && single && !established.presence {
            fail(RefinementObligation::Presence);
        }
        let mut no_form = (raises_lower && !single) || smaller_upper;
        if domain {
            match member.value_type.as_package().and_then(|key| index.scalar_bounds(key)) {
                Some((lower, upper)) => {
                    let contained = established.interval.as_ref().is_some_and(|proved|
                        match (&proved.lower, &proved.upper) {
                            (Some(proved_lower), Some(proved_upper)) =>
                                *proved_lower >= Integer::from(lower) && *proved_upper <= Integer::from(upper),
                            _ => false,
                        });
                    if !contained { fail(RefinementObligation::Domain); }
                }
                None => no_form = true,
            }
        }
        if no_form { fail(RefinementObligation::NoProofForm); }
    }
    failures
}
